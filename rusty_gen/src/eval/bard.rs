use reqwest::Client;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio;

type QuizTuple = (i32, String, String);

#[derive(Serialize)]
struct BardRequest {
    session_id: String,
    message: String,
}

#[derive(Deserialize, Debug)]
struct BardResponse {
    content: String,
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

async fn bard_coherence_score(
    client: &Client,
    session_id: &str,
    prompt: &str,
    question: &str,
) -> Result<String, Box<dyn Error>> {
    let request = BardRequest {
        session_id: session_id.to_string(),
        message: format!("Your output should exactly match this format: [your coherence percent score]%. Do not explain your reasoning and do not include the questions/answers in your response. This is the prompt: \"{}\" and response \"{}\"", prompt, question),
    };

    let response: BardResponse = client
        .post("http://localhost:8000/ask")
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    //println!("{:?}", response.content);
    Ok(response.content)
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let session_id = "your_session_id";
    let quiz_tuples = read_quiz_questions()?;
    for quiz in &quiz_tuples {
        //println!("ID: {}", quiz.0);
        //println!("Prompt: {}", quiz.1);
        //println!("Question: {}", quiz.2);
        let score = bard_coherence_score(&client, &session_id, &quiz.1, &quiz.2).await?;
        println!("Coherence Score: {}", score);
        println!();
    }
    Ok(())
}
