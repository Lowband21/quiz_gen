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

use serde::Deserialize;
use std::fs;

// Define structs to represent the Rubric data from the JSON
#[derive(Deserialize)]
struct Rubric {
    title: String,
    date_assessed: String,
    assessed_by: String,
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    section_id: String,
    title: String,
    questions: Vec<Question>,
}

#[derive(Deserialize, Clone)]
struct Question {
    question_id: String,
    task: String,
    score: String,
    action_yes: Option<String>,
    action_no: Option<String>,
    comments: String,
}

// Function to load the rubric from a JSON file
fn load_rubric_from_file(path: &str) -> Result<Rubric, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;
    let rubric: Rubric = serde_json::from_str(&data)?;
    Ok(rubric)
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
    question: &str,
    rubric: &Rubric, // Change this to a Rubric reference
    model: String,
    mut chat_messages: Vec<Message>,
) -> Result<(i32, Vec<Message>), Box<dyn Error>> {
    chat_messages.push(Message {
        role: Role::System,
        content: format!("Your job is to evaluate the quality of the following question ({}) based on the yes/no questions asked to you:", question.to_string()),
    });
    // Starting with the first question in the rubric
    let mut current_question_id = rubric.sections[0].questions[0].question_id.clone();

    // Will store the final score and feedback
    let mut final_score = 0;

    while let Some(question) = find_question_by_id(&rubric, &current_question_id) {
        // Using the question's task as a prompt for GPT
        chat_messages.push(Message {
            role: Role::User,
            content: format!(
                "This is the question being asked about the question being evaluated, respond with a yes or no: {}",
                question.task.clone()
            ),
        });

        let api_parameters = ChatBody {
            model: model.clone(),
            messages: chat_messages.clone(),
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
                    let response = res.choices[0].message.as_ref().unwrap().content.clone();
                    chat_messages.push(Message {
                        role: Role::Assistant,
                        content: response.clone(),
                    });
                    println!("GPT Response: {} ", response);
                    println!("GPT Question: {} ", question.task.clone());

                    // Determine next question based on GPT's response
                    if response.contains("yes") || response.contains("Yes") {
                        final_score += 1;
                        current_question_id = question
                            .action_yes
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                    } else {
                        current_question_id = question
                            .action_no
                            .as_ref()
                            .unwrap_or(&String::new())
                            .clone();
                    }

                    // If there's no next question, set the final score and feedback
                    if current_question_id.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    tries += 1;
                    if tries >= 10 {
                        final_score = -1;
                        println!("Error after 10 tries: {}", e);
                        break;
                    }
                    thread::sleep(Duration::from_secs(10));
                    println!("Error: {}. Trying again...", e);
                    continue;
                }
            }
        }
    }

    Ok((final_score, chat_messages))
}

// Helper function to find a question by its ID from the rubric
fn find_question_by_id(rubric: &Rubric, question_id: &str) -> Option<Question> {
    for section in &rubric.sections {
        for question in &section.questions {
            if &question.question_id == question_id {
                return Some(question.clone());
            }
        }
    }
    None
}

fn store_score(
    conn: &Connection,
    id: i32,
    relevance: i32,
    complexity: i32,
    clarity: i32,
    breadth: i32,
    feedback_pot: i32,
    total_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT OR REPLACE INTO results (id, relevance, complexity, clarity, breadth, feedback_pot, total_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, relevance, complexity, clarity, breadth, feedback_pot, total_score],
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
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
    let rubric = load_rubric_from_file("rubric.json")?;

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

        quiz_tuples.shuffle(&mut rng);
        if rng.gen_range(0..2) == 1 {
            if let Some(random_tuple) = quiz_tuples.choose(&mut rng) {
                quiz_tuples.push(random_tuple.clone());
            }
        }

        println!("Evaluating {} prompt response pairs.", quiz_tuples.len());
        for quiz in &quiz_tuples {
            let (score, _) =
                gpt_coherence_score(&openai, &quiz.1, &rubric, "gpt-4".to_string(), Vec::new())?;

            // Process the score returned by the model. You might want to update the database with the score or do some other operations.
            // As of now, I'm just printing it, but you can modify this as needed.
            println!("Score for quiz {}: {}", quiz.0, score);
        }
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
