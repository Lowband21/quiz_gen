use openai_api_rust::chat::*;
use openai_api_rust::*;
use rusqlite::{Connection, Result};
use std::error::Error;
use std::thread;
use std::time::Duration;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::sync::mpsc;
use std::io::{self, Write};
use std::fmt;

pub type QuizTuple = (i32, String, String);

#[derive(Debug)]
struct MyError {
    details: String
}

impl MyError {
    fn new(msg: &str) -> MyError {
        MyError{details: msg.to_string()}
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
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
    _prompt: &str,
    question: &str,
    rubric: &str,
) -> Result<String, Box<dyn Error>> {
    let chat_messages = vec![
        Message {
            role: Role::System,
            //content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Your output should be strictly limited to the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the question \"{}\"", rubric, question),
            content: format!("Your job is to evaluate the quality of the following responses based on this rubric: {}. Explain your reasoning in detail followed by a score of the form \"%d-%d-%d-%d\". Where each digit represents a unique rating corresponding to the rubric. This is the question \"{}\"", rubric, question),
        }
    ];
    let api_parameters = ChatBody {
        model: "gpt-3.5-turbo".to_string(),
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
    total_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT OR REPLACE INTO results (id, relevance, complexity, clarity, creativity, total_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, relevance, complexity, clarity, creativity, total_score],
    )?;
    Ok(())
}

fn store_evaluation_score(
    conn: &Connection,
    id: i32,
    hr_relevance: i32,
    hr_complexity: i32,
    hr_clarity: i32,
    hr_creativity: i32,
    gr_relevance: i32,
    gr_complexity: i32,
    gr_clarity: i32,
    gr_creativity: i32,
    hr_score: i32,
    gr_score: i32,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT INTO evaluations (id, hr_relevance, hr_complexity, hr_clarity, hr_creativity, gr_relevance, gr_complexity, gr_clarity, gr_creativity, hr_score, gr_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, hr_relevance, hr_complexity, hr_clarity, hr_creativity, gr_relevance, gr_complexity, gr_clarity, gr_creativity, hr_score, gr_score],
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
    For each item, you will rank 0-10, with 0 being the lowest and 10 being the highest:
    
    Relevance (0-10): Does the question relate to the general topic more than the specifics of the prompt?
    
    Complexity (0-10): Does the question encourage thoughtful reflection or merely factual recall?
    
    Clarity (0-10): Is the question clear, specific, and free of ambiguity?
    
    Creativity (0-10): Does the question offer a fresh perspective on the topic, inspiring creative thought?

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

    // Randomize filenames
    let mut rng = rand::thread_rng();
    let mut filenames = filenames.clone();
    filenames.shuffle(&mut rng);

    for filename in filenames {
        let mut quiz_tuples = match read_quiz_questions_by_filename(&conn, filename.as_str()) {
            Ok(tuples) => tuples,
            Err(e) => {
                println!("Failed to read quiz questions from file {}: {}", filename, e);
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
            let gr = gpt_coherence_score(&openai, &quiz.1, &quiz.2, &rubric)?;
            let hr = manual_evaluation(&quiz, &rubric)?;
        
            //println!("Eval: {:#?}", score);
            let hr_score = hr.split("\n").last().unwrap();
            let gr_score = gr.split("\n").last().unwrap();

            let re = Regex::new(r"(\d+)").unwrap();
            let hr_scores: Vec<i32> = re
                .find_iter(hr_score)
                .map(|m| m.as_str().parse::<i32>())
                .filter_map(Result::ok)
                .collect();
            let gr_scores: Vec<i32> = re
                .find_iter(gr_score)
                .map(|m| m.as_str().parse::<i32>())
                .filter_map(Result::ok)
                .collect();

            if hr_scores.len() == 4 && gr_scores.len() == 4 {
                let hr_total_score: i32 = hr_scores.iter().sum();
                let gr_total_score: i32 = gr_scores.iter().sum();
            
                store_evaluation_score(
                    &conn,
                    quiz.0,
                    hr_scores[0],
                    hr_scores[1],
                    hr_scores[2],
                    hr_scores[3],
                    gr_scores[0],
                    gr_scores[1],
                    gr_scores[2],
                    gr_scores[3],
                    hr_total_score,
                    gr_total_score,
                )?;
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
                    gr_total_score,
                )
                .unwrap();
                println!("Human total Score: {}", hr_total_score);
                println!("GPT total Score: {}", gr_total_score);
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

fn create_evaluations_table(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS evaluations (
            id INTEGER,
            hr_relevance INTEGER NOT NULL,
            hr_complexity INTEGER NOT NULL,
            hr_clarity INTEGER NOT NULL,
            hr_creativity INTEGER NOT NULL,
            gr_relevance INTEGER NOT NULL,
            gr_complexity INTEGER NOT NULL,
            gr_clarity INTEGER NOT NULL,
            gr_creativity INTEGER NOT NULL,
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
