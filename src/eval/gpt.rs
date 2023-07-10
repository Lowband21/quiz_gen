use openai_api_rust::chat::*;
use openai_api_rust::*;
use rusqlite::{Connection, Result};
use std::error::Error;
use std::thread;
use std::time::Duration;
use tokio;

use std::io::{self, Write};

type QuizTuple = (i32, String, String);

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
fn read_quiz_questions() -> Result<Vec<QuizTuple>, Box<dyn Error>> {
    let conn = Connection::open("quiz_questions.db")?;
    let mut stmt = conn.prepare("SELECT id, prompt, question FROM quiz")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
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
            content: format!("Your job is to evaluate the quality of the following prompt response pairs based on this rubric: {}. Your output should be strictly limited to the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the prompt: \"{}\" and question \"{}\"", rubric, prompt, question),
        }
    ];
    let api_parameters = ChatBody {
        model: "gpt-3.5-turbo".to_string(),
        messages: chat_messages,
        max_tokens: Some(9),
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
    loop {
        let response = openai.chat_completion_create(&api_parameters);
        match response {
            Ok(res) => {
                let mut score = res.choices[0].message.as_ref().unwrap().content.clone();
                if score.len() > 9 {
                    println!("Response greater than 10 characters: {}", score);
                    continue;
                } else {
                    return Ok(score);
                }
            }
            Err(e) => {
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
    score: String,
    total_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT INTO results (id, score, total_score) VALUES (?1, ?2, ?3)",
        params![id, score, total_score],
    )?;
    Ok(())
}

use rusqlite::params;

pub fn run() -> Result<(), Box<dyn Error>> {
    let manual = false;
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
    let quiz_tuples = read_quiz_questions()?;
    let rubric = "). For each item you will rank 1-5:

    Relevance: Does the question relate to the content described by the prompt?
        1: Not relevant
        2: Slightly relevant
        3: Moderately relevant
        4: Highly relevant
        5: Completely relevant

    Complexity: Does the question prompt deep thinking, or is it more surface-level? Does it encourage the learner to reflect on the topic in a meaningful way?
        1: Very basic, fact-recall type question
        2: Slightly challenging question
        3: Moderately complex question, encourages some deep thinking
        4: Complex question, requires a deep understanding of the topic
        5: Very complex, thought-provoking question

    Clarity: Is the question clear and unambiguous? Does the wording of the question make sense in the context of the prompt?
        1: Very unclear, ambiguous question
        2: Somewhat unclear question
        3: Moderately clear question
        4: Very clear and specific question
        5: Extremely clear, no ambiguity in the question

    Creativity: Does the question offer a unique or innovative way to consider the topic? Does it prompt the learner to think outside the box?
        1: Very straightforward, lacks creativity
        2: Somewhat creative question
        3: Moderately creative question
        4: Very creative question
        5: Extremely innovative and creative question

    "; // Replace with your actual rubric.
    let conn = Connection::open("quiz_questions.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS results (
                    id INTEGER PRIMARY KEY,
                    score TEXT NOT NULL,
                    total_score INTEGER NOT NULL
                  )",
        [],
    )?;

    let mut high_score = 0;
    println!("Len {}", quiz_tuples.len());
    for quiz in &quiz_tuples {
        let score = if manual {
            manual_evaluation(&quiz, &rubric)?
        } else {
            gpt_coherence_score(&openai, &quiz.1, &quiz.2, &rubric)?
        };
        println!("Score: {}", score);
        let total_score: i32 = score
            .split("-")
            .map(|s| s.parse::<i32>())
            .filter_map(Result::ok)
            .sum();
        if total_score > high_score {
            high_score = total_score;

            println!("High scoring question: {}", &quiz.2);
            println!("High scoring prompt: {}", &quiz.1);
        }
        store_score(&conn, quiz.0, score, total_score).unwrap();
        println!("Total Score: {}", total_score);
    }
    Ok(())
}
