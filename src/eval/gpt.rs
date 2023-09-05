use openai_api_rust::chat::*;
use openai_api_rust::*;
use rand::prelude::SliceRandom;
use rand::Rng;
use rusqlite::{Connection, Result};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub type QuizTuple = (i32, String, String);

#[derive(Debug)]
struct MyError {
    details: String,
}

impl MyError {
    fn new(msg: &str) -> MyError {
        MyError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MyError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn handle_error(error: Box<dyn Error>) -> Box<dyn Error> {
    Box::new(MyError::new(error.description()))
}

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
    let mut stmt = match conn.prepare("SELECT id, prompt, response FROM quiz WHERE filename = ?1") {
        Ok(stmt) => stmt,
        Err(e) => return Err(handle_error(Box::new(e))),
    };

    let rows = match stmt.query_map(params![filename], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    }) {
        Ok(rows) => rows,
        Err(e) => return Err(handle_error(Box::new(e))),
    };

    let mut quiz_tuples = Vec::new();

    for row_result in rows {
        let row = match row_result {
            Ok(row) => row,
            Err(e) => return Err(handle_error(Box::new(e))),
        };

        quiz_tuples.push(row);
    }

    Ok(quiz_tuples)
}

fn gpt_coherence_score(
    openai: &OpenAI,
    prompt: &str,
    question: &str,
    rubric: &str,
    model: String,
) -> Result<String, Box<dyn Error>> {
    let chat_messages = vec![
        Message {
            role: Role::System,
            //content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Your output should be strictly limited to the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the question \"{}\"", rubric, question),
            content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Explain your reasoning in detail followed by a score of the form \"%d-%d-%d-%d-%d-%d\". Where each number represents a unique rating 1-10, with 10 being the higest, corresponding to the rubric. This is the question prompt pair \"{}\"\"{}\"", rubric, question, prompt),
        }
    ];
    let api_parameters = ChatBody {
        model,
        messages: chat_messages,
        max_tokens: Some(500),
        temperature: Some(0.0),
        top_p: Some(1.0),
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
                let score = res.choices[0].message.as_ref().unwrap().content.clone();

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
    breadth: i32,
    feedback_pot: i32,
    total_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT OR REPLACE INTO results (id, relevance, complexity, clarity, creativity, breadth, feedback_pot, total_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, relevance, complexity, clarity, creativity, breadth, feedback_pot, total_score],
    )?;
    Ok(())
}

fn store_evaluation_score(
    conn: &Connection,
    id: i32,
    gr_relevance: i32,
    gr_complexity: i32,
    gr_clarity: i32,
    gr_creativity: i32,
    gr_feedback_pot: i32,
    gr_breadth: i32,
    gr_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "UPDATE evaluations SET gr_relevance = ?2, gr_complexity = ?3, gr_clarity = ?4, gr_creativity = ?5, gr_feedback_pot = ?6, gr_breadth = ?7, gr_score = ?8 WHERE id = ?1",
        params![id, gr_relevance, gr_complexity, gr_clarity, gr_creativity, gr_feedback_pot, gr_breadth, gr_score],
    )?;
    Ok(())
}

use rusqlite::params;

use regex::Regex;

pub fn run(filenames: Vec<String>) -> Result<(), Box<dyn Error>> {
    let conn = create_connection()?;
    create_evaluations_table(&conn)?;
    //let manual = true;
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

    ## **Breadth (0-10):**
    **Definition:** Assesses the range or scope of the question in terms of content covered. A question with good breadth should not be too narrow that it feels nitpicky nor too broad that it feels vague or overwhelming.
    ### Example:
    Topic: History of Computers.
    Narrow Question (Score 3): \"On what exact date was the first punch card created?\"
    Broad Question (Score 9): \"Trace the evolution of data storage from punch cards to solid-state drives, highlighting key technological advancements.\"
    
    ## **Feedback Potential (0-10):**
    **Definition:** Evaluates how effectively a question can be used to diagnose misunderstandings or knowledge gaps. A question with high feedback potential will provide insights into the respondent's thought process or areas of weakness, facilitating targeted feedback..
    ### Example:
    Topic: Thermodynamics.
    Low Feedback (Score 3): \"Is the first law of thermodynamics about conservation of energy?\"
    High Feedback (Score 9): \"Describe a scenario where the first law of thermodynamics is violated, and explain why such a scenario is considered impossible.\"
    ";
    conn.execute(
        "CREATE TABLE IF NOT EXISTS results (
        id INTEGER PRIMARY KEY,
        relevance INTEGER NOT NULL,
        complexity INTEGER NOT NULL,
        clarity INTEGER NOT NULL,
        creativity INTEGER NOT NULL,
        breadth INTEGER NOT NULL,
        feedback_pot INTEGER NOT NULL,
        total_score INTEGER NOT NULL
     )",
        [],
    )?;
    let mut failures = 0;
    let mut count = 0;

    // Randomize filenames
    let mut rng = rand::thread_rng();
    let mut filenames = filenames.clone();
    filenames.shuffle(&mut rng);

    for filename in filenames {
        let mut quiz_tuples = match read_quiz_questions_by_filename(&conn, filename.as_str()) {
            Ok(tuples) => tuples,
            Err(e) => {
                println!(
                    "Failed to read quiz questions from file {}: {}",
                    filename, e
                );
                continue;
            }
        };

        // Randomize quiz tuples and possibly duplicate some of them
        quiz_tuples.shuffle(&mut rng);
        if rng.gen_range(0..2) == 1 {
            if let Some(random_tuple) = quiz_tuples.choose(&mut rng) {
                quiz_tuples.push(random_tuple.clone());
            }
        }

        //let mut high_score = 10;
        println!("Evaluating {} prompt response pairs.", quiz_tuples.len());
        // Inside your quiz_tuples loop
        for quiz in &quiz_tuples {
            // Removed the manual flag and hr initialization
            let gr = gpt_coherence_score(&openai, &quiz.1, &quiz.2, &rubric, "gpt-4".to_string())?;

            //println!("Eval: {:#?}", score);
            let gr_score = gr.split("\n").last().unwrap();

            let re = Regex::new(r"(\d+)").unwrap();

            // Commented out the hr_scores processing
            // let mut hr_scores: Vec<i32> = re
            //     .find_iter(hr_score)
            //     .map(|m| m.as_str().parse::<i32>())
            //     .filter_map(Result::ok)
            //     .collect();
            let mut gr_scores: Vec<i32> = re
                .find_iter(gr_score)
                .map(|m| m.as_str().parse::<i32>())
                .filter_map(Result::ok)
                .collect();

            // Commented out the hr_scores default values
            // while hr_scores.len() < 6 {
            //     hr_scores.push(0);
            // }
            while gr_scores.len() < 6 {
                gr_scores.push(0);
            }

            if gr_scores.len() == 6 {
                // Removed hr_total_score as it's not needed
                let gr_total_score: i32 = gr_scores.iter().sum();

                // Removed hr_scores from store_evaluation_score function
                //store_evaluation_score(
                //    &conn,
                //    quiz.0,
                //    gr_scores[0],
                //    gr_scores[1],
                //    gr_scores[2],
                //    gr_scores[3],
                //    gr_scores[4], // Breadth
                //    gr_scores[5], // Feedback Potential
                //    gr_total_score,
                //)?;
                /*
                if total_score > high_score {
                    high_score = total_score;

                    println!("High scoring question: {}", &quiz.2);
                    //println!("High scoring prompt: {}", &quiz.1);
                }
                */

                store_score(
                    &conn,
                    quiz.0,
                    gr_scores[0],
                    gr_scores[1],
                    gr_scores[2],
                    gr_scores[3],
                    gr_scores[4],
                    gr_scores[5],
                    gr_total_score,
                )
                .unwrap();

                // Commented out the human total score print
                // println!("Human total Score: {}", hr_total_score);
                println!("GPT total Score: {}", gr_total_score);
                count += 1;
            } else {
                println!("Failed to extract score");
                failures += 1;
            }
        }
        println!("Finished with {} failures out of {}", failures, count);
    }
    Ok(())
}

fn create_evaluations_table(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS evaluations (
            id INTEGER,
            hr_relevance INTEGER NOT NULL,
            hr_complexity INTEGER NOT NULL,
            hr_clarity INTEGER NOT NULL,
            hr_creativity INTEGER NOT NULL,
            hr_breadth INTEGER NOT NULL,
            hr_feedback_pot INTEGER NOT NULL,
            gr_relevance INTEGER NOT NULL,
            gr_complexity INTEGER NOT NULL,
            gr_clarity INTEGER NOT NULL,
            gr_creativity INTEGER NOT NULL,
            gr_breadth INTEGER NOT NULL,
            gr_feedback_pot INTEGER NOT NULL,
            hr_score INTEGER NOT NULL,
            gr_score INTEGER NOT NULL
         )",
        [],
    )?;
    Ok(())
}

fn create_connection() -> rusqlite::Result<Connection> {
    Connection::open("quiz_questions.db")
}
