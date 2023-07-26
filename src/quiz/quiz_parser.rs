use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct QuizQuestion {
    pub question: String,
    pub answer: String,
    pub key: String,
}

impl QuizQuestion {
    pub fn new(question: String, answer: String, key: String) -> Self {
        Self {
            question,
            answer,
            key,
        }
    }
}

use rusqlite::{params, Connection, Result};
use regex::Regex;
use log::{info, error, debug};

pub fn filter_and_parse(top_n: i32) -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Connecting to the database");
    let conn = Connection::open("quiz_questions.db")?;

    // Define your regex pattern for parsing question, answer, key
    let re_question = Regex::new(r"Question: (.*)").unwrap();
    let re_answer = Regex::new(r"Possible Answers:\s*((?s).*)Key:").unwrap();
    let re_key = Regex::new(r"Key: (.*)").unwrap();

    info!("Preparing SQL statement");
    let sql_query = format!("
        SELECT response, total_score, filename
        FROM (
            SELECT quiz.response AS response, 
                   results.total_score AS total_score, 
                   quiz.filename AS filename,
                   ROW_NUMBER() OVER(PARTITION BY quiz.filename ORDER BY results.total_score DESC) rn
            FROM results
            JOIN quiz ON quiz.id = results.id
        )
        WHERE rn <= {}
    ", top_n);

    info!("Executing SQL query: {}", sql_query);
    let mut stmt = match conn.prepare(&sql_query) {
        Ok(stmt) => stmt,
        Err(e) => {
            error!("Failed to prepare SQL statement: {}", e);
            return Err(e);
        }
    };

    let rows = stmt.query_map([], |row| {
        let response: String = row.get(0)?;
        let total_score: i32 = row.get(1)?;
        let filename: String = row.get(2)?;

        // Parse response into question, answer, key
        let question = re_question.captures(&response)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
            .unwrap_or("").to_string().trim().to_string();
        
        let answer = re_answer.captures(&response)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
            .unwrap_or("").trim().replace("\n\n", "\n").to_string();
        
        let key = re_key.captures(&response)
            .and_then(|cap| cap.get(1).map(|m| m.as_str()))
            .unwrap_or("").trim().to_string();

        Ok((QuizQuestion::new(question, answer, key), total_score, filename))
    })?;

    info!("Creating new table");
    conn.execute("
        CREATE TABLE IF NOT EXISTS top_questions (
            id INTEGER PRIMARY KEY,
            question TEXT NOT NULL,
            answer TEXT NOT NULL,
            key TEXT NOT NULL,
            total_score INTEGER NOT NULL,
            filename TEXT NOT NULL
        )
    ", [])?;

    // Insert data into the new table
    info!("Inserting data into new table");
    let mut i = 0;
    for row_result in rows {
        match row_result {
            Ok((quiz_question, total_score, filename)) => {
                debug!("Inserting question: {}", &quiz_question.question);
                conn.execute("
                    INSERT OR REPLACE INTO top_questions (id, question, answer, key, total_score, filename)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ", params![i, quiz_question.question, quiz_question.answer, quiz_question.key, total_score, filename])?;
                i += 1;
            },
            Err(e) => {
                error!("Failed to process a row: {}", e);
            }
        }
    }
    
    info!("Successfully finished operation");
    Ok(())
}

pub fn parse_quiz_file(path: &Path) -> io::Result<(Vec<QuizQuestion>, f64)> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut quiz_questions = Vec::new();
    let mut question = String::new();
    let mut answer = String::new();
    let mut key = String::new();

    let mut total = 0;
    let mut improperly_formatted = 0;

    let mut last_heading = None;

    for line in reader.lines() {
        let line = line?;
        // Update this to use regex instead of contains:
        if line.contains("Question:") {
            if !question.is_empty() && (answer.is_empty() || key.is_empty()) {
                improperly_formatted += 1;
                total += 1;
            }
            question = line["Question:".len() + 3..].trim().to_string();
            answer.clear();
            key.clear();
            last_heading = Some("Question");
        } else if line.starts_with("Possible Answers:") {
            answer = line["Possible Answers:".len()..].trim().to_string();
            last_heading = Some("Possible Answers");
        } else if line.starts_with("Key:") {
            key = line["Key:".len()..].trim().to_string();
            last_heading = Some("Key");
        } else {
            match last_heading {
                Some("Question") => question += &("\n".to_owned() + line.trim()),
                Some("Possible Answers") => answer += &("\n".to_owned() + line.trim()),
                Some("Key") => key += &("\n".to_owned() + line.trim()),
                _ => {}
            }
        }

        // If we've filled out all fields, add the question to the list
        if !question.is_empty() && !answer.is_empty() && !key.is_empty() {
            quiz_questions.push(QuizQuestion::new(
                question.clone(),
                answer.clone(),
                key.clone(),
            ));
            question.clear();
            answer.clear();
            key.clear();
            last_heading = None;
            total += 1;
        }
    }

    // Catch any remaining question
    if !question.is_empty() && !answer.is_empty() && !key.is_empty() {
        quiz_questions.push(QuizQuestion::new(question, answer, key));
        total += 1;
    } else if !question.is_empty() || !answer.is_empty() || !key.is_empty() {
        total += 1;
        improperly_formatted += 1;
    }

    Ok((quiz_questions, improperly_formatted as f64 / total as f64))
}

