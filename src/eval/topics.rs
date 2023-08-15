use openai::OpenAI;
use rusqlite::{Connection, Row};
use std::error::Error;

pub fn categorize_top_100(
    conn: &Connection,
    openai: &OpenAI,
) -> Result<Vec<(i32, String, i32, String)>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT results.id, quiz.prompt, results.total_score
        FROM results
        JOIN quiz ON results.id = quiz.id
        ORDER BY results.total_score DESC
        LIMIT 100",
    )?;

    let rows = stmt.query_map([], |row| {
        let id: i32 = row.get(0)?;
        let prompt: String = row.get(1)?;
        let total_score: i32 = row.get(2)?;

        // Extract the main topic or concept of the question.
        let main_topic = extract_topic_from_question(openai, &prompt)?;

        Ok((id, response, total_score, main_topic))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

fn extract_topic_from_question(openai: &OpenAI, prompt: &str) -> Result<String, Box<dyn Error>> {
    let prompt = format!(
        "What is the main topic or concept of this question: \"{}\"?",
        prompt
    );

    let chat_messages = vec![openai::Message {
        role: openai::Role::System,
        content: prompt,
    }];

    let api_parameters = openai::ChatBody {
        model: "gpt-3.5-turbo-16k".to_string(), // Choose an appropriate model.
        messages: chat_messages,
        max_tokens: Some(100),
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
                let topic = res.choices[0].message.as_ref().unwrap().content.clone();
                if tries >= 10 {
                    return Ok(topic);
                }
                return Ok(topic);
            }
            Err(e) => {
                tries += 1;
                if tries >= 10 {
                    return Err(Box::new(e));
                }
                std::thread::sleep(std::time::Duration::from_secs(10));
                println!("Error: {}. Trying again...", e);
                continue;
            }
        }
    }
}

// Make sure to have imports and the correct dependencies for rusqlite and openai.
