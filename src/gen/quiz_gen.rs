// quiz.rs

use openai_api_rust::chat::*;
use openai_api_rust::*;
use regex::Regex;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

//use std::collections::HashMap;
use std::error::Error;

pub fn log_api_call(prompt: &str, api_parameters: &str) {
    let log_message = format!(
        "{}: Prompt: {}, API parameters: {}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        prompt,
        api_parameters
    );

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("openai_api_logs.log")
        .unwrap();
    writeln!(file, "{}", log_message).unwrap();
}

pub fn preprocess_content(content: &str) -> String {
    let re_whitespace = Regex::new(r"\s+").unwrap();
    let re_special_chars = Regex::new(r"[^0-9a-zA-Z.,;:?!#]+").unwrap();
    let content = re_whitespace.replace_all(content, " ");
    let content = re_special_chars.replace_all(&content, " ");
    //let sections = content
    //    .split("##### ")
    //    .map(|s| s.to_string())
    //    .collect::<Vec<_>>();
    content.to_string()
}


pub fn generate_question(openai: &OpenAI, prompt: &str) -> Result<String, Box<dyn Error>> {
    let mut model = "gpt-3.5-turbo".to_string();
    if prompt.split_whitespace().count() > 4097 {
        model = "gpt-3.5-turbo-16k".to_string();
    }

    let api_parameters = ChatBody {
        model: model,
        max_tokens: Some(200),
        temperature: Some(1.0),
        top_p: Some(0.6),
        n: None,
        stream: None,
        stop: None,
        presence_penalty: Some(0.0),
        frequency_penalty: Some(0.0),
        logit_bias: None,
        user: None,
        messages: vec![
            Message {
                role: Role::System,
                content: "You are a helpful assistant that generates quiz questions.".to_string(),
            },
            Message {
                role: Role::User,
                content: prompt.to_string(),
            },
        ],
    };

    loop {
        let response = openai.chat_completion_create(&api_parameters);
        match response {
            Ok(res) => {
                let question = res.choices[0].message.as_ref().unwrap().content.clone();

                // Log the API call
                log_api_call(prompt, &serde_json::to_string(&api_parameters).unwrap());

                /*
                if question.contains("the text")
                    || question.contains("the code")
                    || question.contains("given text")
                    || question.contains("given code")
                {
                    println!("Question is being filtered: {}", question);
                    continue;
                }
                */

                return Ok(question);
            }
            Err(e) => {
                thread::sleep(Duration::from_secs(10));
                eprintln!("Error: {}. Trying again...", e);
                continue;
            }
        }
    }
}

fn difficulty_to_num(difficulty: &str) -> i32 {
    match difficulty {
        "easy" => 1,
        "medium" => 2,
        "hard" => 3,
        _ => panic!("Invalid difficulty"),
    }
}

use rusqlite::{params, Connection, Result};

pub fn generate_quiz_questions(
    openai: &OpenAI,
    parsed_content: &String,
    question_type: &str,
    difficulty_level: &str,
    filename: &str,
) -> Result<String, rusqlite::Error> {
    // Create a new connection to an SQLite database
    let conn = Connection::open("quiz_questions.db")?;

    let difficulty = difficulty_to_num(difficulty_level);

    // Create a new table named "quiz" in the database, if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quiz (
            id INTEGER PRIMARY KEY,
            prompt TEXT NOT NULL,
            response TEXT NOT NULL,
            filename TEXT NOT NULL,
            type TEXT NOT NULL,
            difficulty TEXT
        )",
        params![],
    )?;

    let prompt = format!(
            "From the following text, please generate a {} question with a difficulty level of {}:\n\n{}\n\n  Your question should assess a core concept from the content. Do not reference the given text or the given code! Please format your output as follows:\nQuestion: [Your question here]\nPossible Answers: [The possible a, b, c, and d answers here]\nKey: [lowercase letter here]",
            question_type, difficulty_level, parsed_content.as_str()

        );
    let question = generate_question(openai, &prompt).unwrap();

    // Insert prompt and question into the "quiz" table
    conn.execute(
        "INSERT INTO quiz (prompt, response, filename, type, difficulty) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![prompt, question, filename, question_type, difficulty],
    )?;

    Ok(question)
}

/*
pub fn question_difficulty(difficulty_level: &str) -> u8 {
    match difficulty_level {
        "easy" => 1,
        "medium" => 2,
        "hard" => 3,
        _ => 1,
    }
}
*/
