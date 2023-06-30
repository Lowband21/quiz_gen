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

/*
pub fn start_quiz(quiz_questions: &[QuizQuestion]) {
    let mut score = 0;

    for (i, quiz_question) in quiz_questions.iter().enumerate() {
        println!("Question {}: {}", i + 1, quiz_question.question);

        let answer_question = Question::input("answer")
            .message("What's your answer?")
            .build();

        let user_answer = &requestty::prompt_one(answer_question).unwrap();
        let user_answer = user_answer.as_string().unwrap();

        if user_answer.to_lowercase() == quiz_question.answer.to_lowercase() {
            println!("Correct!");
            score += 1;
        } else {
            println!(
                "Incorrect. The correct answer was: {}",
                quiz_question.answer
            );
        }
    }

    println!(
        "Quiz finished! Your score: {}/{}",
        score,
        quiz_questions.len()
    );
}
*/
