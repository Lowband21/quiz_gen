use openai_api_rust::chat::*;
use openai_api_rust::*;
use rusqlite::{Connection, Result};
use std::error::Error;
use std::thread;
use std::time::Duration;
use tokio;

use std::io::{self, Write};

pub type QuizTuple = (i32, String, String);

fn manual_evaluation(quiz: &QuizTuple, rubric: &str) -> Result<String, Box<dyn Error>> {
    println!(
        "Evaluate the following prompt-question pair based on the rubric below:\n{}\nPrompt: {}\nQuestion: {}",
        rubric,
        quiz.1,
        quiz.2
    );
    print!("Enter your evaluation (format: %d-%d-%d-%d): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
fn read_quiz_questions_by_filename(
    conn: &Connection,
    filename: &str,
) -> Result<Vec<QuizTuple>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT id, prompt, response FROM quiz WHERE filename = ?1")?;
    let rows = stmt.query_map(params![filename], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    let mut quiz_tuples = Vec::new();
    for row_result in rows {
        let row = row_result?;
        quiz_tuples.push(row);
    }
    Ok(quiz_tuples)
}

fn gpt_coherence_score(
    openai: &OpenAI,
    prompt: &str,
    question: &str,
    rubric: &str,
) -> Result<String, Box<dyn Error>> {
    let chat_messages = vec![
        Message {
            role: Role::System,
            //content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Your output should be strictly limited to the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the question \"{}\"", rubric, question),
            content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Explain your reasoning in detail followed by a score of the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the question response pair \"{}\"\"{}\"", rubric, prompt, question),
        }
    ];
    let api_parameters = ChatBody {
        model: "gpt-3.5-turbo-16k".to_string(),
        messages: chat_messages,
        max_tokens: Some(500),
        temperature: Some(0.2),
        top_p: None,
        n: None,
        stream: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
    };
    let mut tries = 0;
    loop {
        let response = openai.chat_completion_create(&api_parameters);
        match response {
            Ok(res) => {
                let mut score = res.choices[0].message.as_ref().unwrap().content.clone();

                if tries >= 10 {
                    return Ok(score);
                }
                /*
                if score.len() > 1000 {
                    println!("Response greater than 10 characters: {}", score);
                    tries += 1;
                    continue;
                } else {
                }
                */
                return Ok(score);
            }
            Err(e) => {
                tries += 1;
                thread::sleep(Duration::from_secs(10));
                println!("Error: {}. Trying again...", e);
                continue;
            }
        }
    }
}

fn store_score(
    conn: &Connection,
    id: i32,
    relevance: i32,
    complexity: i32,
    clarity: i32,
    creativity: i32,
    total_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT OR REPLACE INTO results (id, relevance, complexity, clarity, creativity, total_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, relevance, complexity, clarity, creativity, total_score],
    )?;
    Ok(())
}

use rusqlite::params;

use regex::Regex;

pub fn run(filenames: Vec<String>) -> Result<(), Box<dyn Error>> {
    let conn = &Connection::open("quiz_questions.db")?;
    let manual = false;
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
    let rubric = "
## **Relevance (0-10):**
**Definition:** How closely does the question align with the overarching topic rather than the nitty-gritty details of the prompt? A highly relevant question should address the core concepts and objectives of the topic.
### Example:
Topic: Algorithms.
Good Question (Score 9): \"Why is the Big O notation important when evaluating algorithms?\"
Irrelevant Question (Score 2): \"Who was the 15th employee hired by Google?\"

## **Complexity (0-10):**
**Definition:** Evaluates the depth of cognitive engagement the question demands. A complex question should tap into higher-order thinking skills such as analysis, synthesis, and evaluation, rather than just memory recall.
### Example:
Topic: Object-Oriented Programming (OOP).
Simple Question (Score 3): \"What does OOP stand for?\"
Complex Question (Score 9): \"How might encapsulation in OOP lead to more maintainable and scalable software, and what are potential pitfalls if it's not utilized properly?\"

## **Clarity (0-10):**
**Definition:** Assesses the question's understandability and preciseness. A clear question should be straightforward, not open to multiple interpretations, and should not confuse the respondent.

### Example:
Topic: Data Structures.
Clear Question (Score 9): \"How does a hash table resolve collisions?\"
Ambiguous Question (Score 2): \"Can you explain that thing with tables and matching stuff?\"

## **Creativity (0-10):**
**Definition:** Measures the originality of the question and its ability to provoke unconventional thought. A creative question will often approach a familiar topic from a novel angle or combine concepts in an unexpected way.

### Example:
Topic: Artificial Intelligence.
Standard Question (Score 4): \"What is the Turing Test?\"
Creative Question (Score 9): \"If a neural network, a decision tree, and a support vector machine were characters in a story, how might their personalities differ based on their algorithmic behaviors and learning methodologies?\"

    "; // Replace with your actual rubric.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS results (
        id INTEGER PRIMARY KEY,
        relevance INTEGER NOT NULL,
        complexity INTEGER NOT NULL,
        clarity INTEGER NOT NULL,
        creativity INTEGER NOT NULL,
        total_score INTEGER NOT NULL
     )",
        [],
    )?;
    let mut failures = 0;
    let mut count = 0;

    let mut high_score = 20;
    for filename in filenames {
        let quiz_tuples = read_quiz_questions_by_filename(conn, filename.as_str())?;

        println!("Evaluating {} prompt response pairs.", quiz_tuples.len());
        for quiz in &quiz_tuples {
            let score = if manual {
                manual_evaluation(&quiz, &rubric)?
            } else {
                gpt_coherence_score(&openai, &quiz.1, &quiz.2, &rubric)?
            };

            //println!("Eval: {:#?}", score);
            let score = score.split("\n").last().unwrap();

            let re = Regex::new(r"(\d+)").unwrap();
            let scores: Vec<i32> = re
                .find_iter(score)
                .map(|m| m.as_str().parse::<i32>())
                .filter_map(Result::ok)
                .collect();

            if scores.len() == 4 {
                let total_score: i32 = scores.iter().sum();

                if total_score > high_score {
                    high_score = total_score;

                    println!("High scoring question: {}", &quiz.2);
                    //println!("High scoring prompt: {}", &quiz.1);
                }

                store_score(
                    &conn,
                    quiz.0,
                    scores[0],
                    scores[1],
                    scores[2],
                    scores[3],
                    total_score,
                )
                .unwrap();
                println!("Total Score: {}", total_score);
                count += 1;
            } else {
                println!("Failed to extract score");
                failures += 1;
            }
        }
    }
    println!("Finished with {} failures out of {}", failures, count);
    Ok(())
}
