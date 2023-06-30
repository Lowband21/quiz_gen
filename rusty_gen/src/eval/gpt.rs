use openai_api_rust::chat::*;
use openai_api_rust::*;
use rusqlite::{Connection, Result};
use std::error::Error;
use tokio;

type QuizTuple = (i32, String, String);

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

async fn gpt_coherence_score(
    openai: &OpenAI,
    prompt: &str,
    question: &str,
) -> Result<String, Box<dyn Error>> {
    let chat_messages = vec![
        Message {
            role: Role::System,
            content: format!("Your job is to evaluate the coherence of the following prompt response pairs. You should evaluate the coherence based on how well the questions address the major concepts in the prompt. Your output should be of the form [your coherence percent score]%. Do not explain your reasoning and do not include the questions/answers in your response. This is the prompt: \"{}\" and response \"{}\"", prompt, question),
        }
    ];
    let api_parameters = ChatBody {
        model: "gpt-3.5-turbo".to_string(),
        messages: chat_messages,
        max_tokens: Some(100),
        temperature: Some(0.5),
        top_p: None,
        n: None,
        stream: None,
        stop: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: None,
    };
    let response = openai.chat_completion_create(&api_parameters).unwrap();
    Ok(response.choices[0]
        .message
        .as_ref()
        .unwrap()
        .content
        .clone())
}

pub async fn run() -> Result<(), Box<dyn Error>> {
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
    let quiz_tuples = read_quiz_questions()?;
    for quiz in &quiz_tuples {
        let score = gpt_coherence_score(&openai, &quiz.1, &quiz.2).await?;
        println!("Coherence Score: {}", score);
        println!();
    }
    Ok(())
}
