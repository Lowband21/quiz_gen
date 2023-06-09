use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Clone)]
struct QuizQuestion {
    question: String,
    answers: Vec<String>,
    key: char,
}

impl QuizQuestion {
    fn ask(&self) -> bool {
        println!("{}", self.question.trim_start_matches(": "));
        for (i, answer) in self.answers.iter().enumerate() {
            println!("{}) {}", (b'a' + i as u8) as char, answer);
        }
        print!("Your answer: ");
        io::stdout().flush().expect("Failed to flush stdout");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read user input");
        if input
            .trim()
                .chars()
                .next()
                .unwrap_or('\0')
                .to_ascii_lowercase()
                == self.key
                {
                    true
                } else {
                    println!(
                        "Question answered incorrectly, correct answer is {}",
                        self.key
            );
            false
        }
    }
}

pub fn quiz(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut quiz_data: Vec<QuizQuestion> = vec![];
    let mut question: Option<String> = None;
    let mut answers: Vec<String> = vec![];
    let mut key: char = '\0'; // Temporary key
    let mut is_answering = false;
    let mut is_question = false;
    let mut is_key = false;

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("Question:") {
            // Save previous question, if any
            if let Some(q) = question {
                quiz_data.push(QuizQuestion {
                    question: q,
                    answers: answers.clone(),
                    key,
                });
            }
            question = None;
            answers.clear();
            is_answering = false;
            is_question = true;
            is_key = false;
        } else if line.starts_with("Possible Answers:") {
            is_answering = true;
            is_question = false;
            is_key = false;
        } else if line.starts_with("Key:") {
            is_answering = false;
            is_question = false;
            is_key = true;
        } else {
            if is_answering {
                answers.push(line.trim().to_string());
            } else if is_question {
                question = Some(line.trim().to_string());
            } else if is_key {
                key = line
                    .trim()
                    .chars()
                    .next()
                    .unwrap_or('\0')
                    .to_ascii_lowercase();
                is_key = false;
            }
        }
    }

    // Save the last question
    if let Some(q) = question {
        quiz_data.push(QuizQuestion {
            question: q,
            answers: answers.clone(),
            key,
        });
    }

    let mut correct_answers = 0;
    for question in &quiz_data {
        if question.ask() {
            correct_answers += 1;
        }
    }

    println!(
        "You answered {} out of {} questions correctly.",
        correct_answers,
        quiz_data.len()
    );

    Ok(())
}
