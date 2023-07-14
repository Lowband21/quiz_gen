// explanation.rs

use openai_api_rust::chat::*;
use openai_api_rust::*;
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

/*
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
*/

pub fn generate_explanation(openai: &OpenAI, prompt: &str) -> String {
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
                content: "You are a helpful assistant that rephrases explanations.".to_string(),
            },
            Message {
                role: Role::User,
                content: prompt.to_string(),
            },
        ],
    };

    let response = openai.chat_completion_create(&api_parameters).unwrap();
    let explanation = response.choices[0]
        .message
        .as_ref()
        .unwrap()
        .content
        .clone();

    // Log the API call
    log_api_call(prompt, &serde_json::to_string(&api_parameters).unwrap());

    explanation
}

pub fn generate_explanations(openai: &OpenAI, parsed_content: &String) -> String {
    let prompt = format!(
            "Please rephrase the following explanation:\n\n{}\n\nPlease format your output as follows:\nExplanation: [Your rephrased explanation here]",
            parsed_content
        );
    let explanation = generate_explanation(openai, &prompt);

    explanation
}
