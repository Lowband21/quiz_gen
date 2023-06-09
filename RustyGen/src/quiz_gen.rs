// quiz.rs

use itertools::Itertools;
use openai_api_rust::chat::*;
use openai_api_rust::*;
use regex::Regex;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::SystemTime;

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

pub fn preprocess_content(content: &str) -> Vec<String> {
    let re_whitespace = Regex::new(r"\s+").unwrap();
    let re_special_chars = Regex::new(r"[^0-9a-zA-Z.,;:?!#]+").unwrap();
    let content = re_whitespace.replace_all(content, " ");
    let content = re_special_chars.replace_all(&content, " ");
    let sections = content
        .split("##### ")
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    sections
}

pub fn question_difficulty(difficulty_level: &str) -> u8 {
    match difficulty_level {
        "easy" => 1,
        "medium" => 2,
        "hard" => 3,
        _ => 1,
    }
}

pub fn generate_question(openai: &OpenAI, prompt: &str) -> String {
    let api_parameters = ChatBody {
        model: "gpt-3.5-turbo".to_string(),
        max_tokens: Some(1000),
        temperature: Some(0.8),
        top_p: Some(1.0),
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

    let response = openai.chat_completion_create(&api_parameters).unwrap();
    let question = response.choices[0]
        .message
        .as_ref()
        .unwrap()
        .content
        .clone();

    // Log the API call
    log_api_call(prompt, &serde_json::to_string(&api_parameters).unwrap());

    question
}

use rusqlite::{params, Connection, Result};

pub fn generate_quiz_questions(
    openai: &OpenAI,
    parsed_content: &[String],
    question_type: &str,
    difficulty_level: &str,
) -> Result<Vec<String>, rusqlite::Error> {
    // Create a new connection to an SQLite database
    let conn = Connection::open("quiz_questions.db")?;

    // Create a new table named "quiz" in the database, if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quiz (
            id INTEGER PRIMARY KEY,
            prompt TEXT NOT NULL,
            question TEXT NOT NULL
        )",
        params![],
    )?;

    let mut questions = Vec::new();
    for (idx, section) in parsed_content.iter().enumerate() {
        if idx > 10 {
            break;
        }
        let prompt = format!(
            "From the following text, please generate a {} question with a difficulty level of {}:\n\n{}\n\nPlease format your output as follows:\nQuestion: [Your question here]\nPossible Answers: [The possible a, b, c, and d answers here]\nKey: [lowercase letter here]",
            question_type, difficulty_level, section

        );
        let question = generate_question(openai, &prompt);
        questions.push(format!("{}. {}", idx + 1, question));
        println!("{}", format!("{}. {}", idx + 1, question));

        // Insert prompt and question into the "quiz" table
        conn.execute(
            "INSERT INTO quiz (prompt, question) VALUES (?1, ?2)",
            params![prompt, question],
        )?;
    }

    Ok(questions)
}
