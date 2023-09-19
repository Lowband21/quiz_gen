use openai_api_rust::chat::*;
use openai_api_rust::*;
use rand::prelude::SliceRandom;
use rand::Rng;
use rusqlite::{Connection, Result};
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub type QuizTuple = (i32, String, String);

#[derive(Debug)]
struct MyError {
    details: String,
}

impl MyError {
    fn new(msg: &str) -> MyError {
        MyError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MyError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn handle_error(error: Box<dyn Error>) -> Box<dyn Error> {
    Box::new(MyError::new(error.to_string().as_str()))
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
    feedback: &str,
    rubric: &str,
    model: String,
) -> Result<String, Box<dyn Error>> {
    let chat_messages = vec![
        Message {
            role: Role::System,
            content: format!("Your job is to evaluate the quality of the following feedback based on this rubric: {}. Provide a score of the form \"%d\". Where the single number represents a unique rating from 1-5, with 5 being the higest, corresponding to the rubric. This is the feedback \"{}\"", rubric, feedback),
        }
    ];
    let api_parameters = ChatBody {
        model,
        messages: chat_messages,
        max_tokens: Some(500),
        temperature: Some(0.0),
        top_p: Some(1.0),
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
                return Ok(score);
            }
            Err(e) => {
                tries += 1;
                println!("Error: {}. Trying again in 10 seconds...", e);
                thread::sleep(Duration::from_secs(10));
                continue;
            }
        }
    }
}

fn store_score(
    conn: &Connection,
    id: i32,
    total_score: i32,
    explanation: &String,
) -> Result<(), Box<dyn Error>> {
    conn.execute(
        "INSERT OR REPLACE INTO alt_eval_results (id, total_score, explanation) VALUES (?1, ?2, ?3)",
        params![id, total_score, explanation],
    )?;
    Ok(())
}

struct Feedback {
    id: i32,
    annotation_text: String,
}

fn read_feedback() -> Vec<Feedback> {
    // Create a CSV reader
    let mut rdr =
        csv::Reader::from_path("AnnotationsProofConcept_ID.csv").expect("Unable to open file");

    let mut result = Vec::new();

    // Iterate over records
    for record in rdr.records() {
        let record = record.expect("Error reading record");

        let feedback = Feedback {
            id: record[0].parse::<i32>().expect("Error parsing id"),
            annotation_text: record[1].to_string(),
        };

        result.push(feedback);
    }

    result
}
use rusqlite::params;

use regex::Regex;

pub fn run() -> Result<(), Box<dyn Error>> {
    let conn = create_connection()?;
    //let manual = true;
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");
    let rubric = "
## **Tier 1 - Least Robust:**
**Definition:** \"Annotations categorized as Tier 1 are the least specific and do not provide constructive feedback for the author. They are generic and can be applied to any thinklet. Authors do not attempt to connect to the work in the thinklet.\"
### Example:
Annotation: 
Example 1: \"Good job explaining it (#219299)\" 
Example 2: \"I like the way you how you organize it (#216124)\"
Example 3: \"it was good (#217438)\"

## **Tier 2 - Somewhat Robust:**
**Definition:** \"Annotations categorized as Tier 2 are lacking in specific details. They provide little or no elaboration aside from the mathematics. Authors may attempt to connect to the work in the thinklet, but the connection is vague and not clearly explained.\"
### Example:
Example 1: \"I hadn't thought of the way you found the answer. Although while you were explaining it, it became clear. (#216655)\"
Example 2: \"I hadn't thought of the way you found the answer. Although while you were explaining it, it became clear. <3 (#217906)\"
Example 3: \"I like the way you sorted the data. I also like the way you showed your work clearly so it wouldn't be confusing. (#218212)\"

## **Tier 3 - Robust:**
**Definition:** \"Annotations categorized as Tier 3 begin to elaborate on a specific piece of the thinklet or problem related to the problem-solving process. Authors may attempt to connect to the work in the thinklet, with specificity.\"
### Example:
Example 1: \"I like the way you showed your work step by step, but the question asks what was the total amount of money he lost or gained by the end of the day. (#217908)\"
Example 2: \"I agree with your answer but maybe next time make the numbers that are supposed to be negative negative in the equation to make it more clear. (#217899)\"
Example 3: \"I respectfully disagree with your answer. I think you might have messed up a step in the subtraction part. (#216643)\"

## **Tier 4 - More Robust:**
**Definition:** \"Annotations categorized as Tier 4 elaborate on a specific piece of the thinklet or problem related to the problem-solving process. Authors may connect to the work in the thinklet, with specificity but lack recommendations for next steps.\"
### Example:
Example 1: \"I like the way you added all the deposits first then subtracted the withdrawals. I didn't think of that. (#216645)\"
Example 2: \"My strategy is like yours because I put the information in almost the exact same way. I think I just switched the places of the two withdrawals. (#216652)\"
Example 3: \"I like the way you added the positive numbers together then subtracted the negatives to make the equation simpler. (#216647)\"

## **Tier 5 - Most Robust:**
**Definition:** \"Annotations categorized as Tier 5 elaborate on a specific piece of the thinklet or problem related to the problem-solving process. Authors provide helpful feedback, and peer-to-peer learning is evident.\"
### Example:
Score 1: \"I respectfully disagree with you on the last pieces of your math as you had added a positive with a negative. While you should have added -30 with the -83 and gotten -113, then subtracted that with the positive 76 and gotten -37. (#216197)\"
Score 2: \"Hi. First, that is how you find the median, not the mean. So first, you have to find the mean, add all of the numbers, and then divide it by how many numbers there are. Then once you found the mean, you estimate... (#218215)\"
    ";
    conn.execute(
        "CREATE TABLE IF NOT EXISTS alt_eval_results (
        id INTEGER PRIMARY KEY,
        total_score INTEGER NOT NULL,
        explanation STRING NOT NULL
     )",
        [],
    )?;
    let mut failures = 0;
    let mut count = 0;

    // Randomize feedback
    let mut rng = rand::thread_rng();
    let mut feedback_vec = read_feedback();
    let feedback_len = feedback_vec.len();
    feedback_vec.shuffle(&mut rng);

    println!("Evaluating {} prompt response pairs.", feedback_len);

    // Track the start time
    let start_time = std::time::Instant::now();

    for (index, feedback) in feedback_vec.iter().enumerate() {
        /*
        if rng.gen_range(0..2) == 1 {
            if let Some(random_tuple) = quiz_tuples.choose(&mut rng) {
                quiz_tuples.push(random_tuple.clone());
            }
        }
        */

        let gr = gpt_coherence_score(
            &openai,
            feedback.annotation_text.as_str(),
            rubric,
            "gpt-4".to_string(),
        )?;

        let gr_score = gr.split("\n").last().unwrap();

        let re = Regex::new(r"(\d+)").unwrap();

        let all_matches: Vec<i32> = re
            .find_iter(gr_score)
            .map(|m| m.as_str().parse::<i32>())
            .filter_map(Result::ok)
            .collect();

        let mut gr_scores = Vec::new();
        if let Some(&last_num) = all_matches.last() {
            gr_scores.push(last_num);
        }

        while gr_scores.len() < 1 {
            gr_scores.push(0);
        }

        if gr_scores.len() == 1 {
            let gr_total_score: i32 = gr_scores.iter().sum();

            store_score(&conn, feedback.id, gr_total_score, &gr).unwrap();

            println!("GPT total Score: {}\nExplanation: {}", gr_total_score, gr);
            count += 1;
        } else {
            println!("Failed to extract score from: {:?}", gr);
            failures += 1;
        } // After processing each feedback or after every nth feedback, print progress and ETA
        if index % 1 == 0 && index > 0 {
            // adjust this to control how often you want to print
            let elapsed = start_time.elapsed();
            let progress = (index + 1) as f64 / feedback_len as f64;
            let elapsed_secs = elapsed.as_secs_f64(); // Convert Duration to seconds (as f64)
            let estimated_total_time_secs = elapsed_secs / progress;
            let estimated_time_remaining_secs = estimated_total_time_secs - elapsed_secs;
            let estimated_time_remaining =
                std::time::Duration::from_secs_f64(estimated_time_remaining_secs);

            // Format estimated time remaining
            let hours_remaining = estimated_time_remaining.as_secs() / 3600;
            let minutes_remaining = (estimated_time_remaining.as_secs() % 3600) / 60;
            let seconds_remaining = estimated_time_remaining.as_secs() % 60;

            // Format total estimated time
            let estimated_total_time =
                std::time::Duration::from_secs_f64(estimated_total_time_secs);
            let hours_total = estimated_total_time.as_secs() / 3600;
            let minutes_total = (estimated_total_time.as_secs() % 3600) / 60;
            let seconds_total = estimated_total_time.as_secs() % 60;

            // Create a progress bar
            let bar_length = 50; // 50 characters
            let progress_position = (progress * bar_length as f64).round() as usize;
            let progress_bar: String = format!(
                "[{}>{}]",
                "=".repeat(progress_position),
                " ".repeat(bar_length - progress_position)
            );

            println!(
        "{} Progress: {:.2}% | Elapsed: {:02}h {:02}m {:02}s | Estimated Time Remaining: {:02}h {:02}m {:02}s | Estimated Total Completion Time: {:02}h {:02}m {:02}s",
        progress_bar,
        progress * 100.0,
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60,
        hours_remaining,
        minutes_remaining,
        seconds_remaining,
        hours_total,
        minutes_total,
        seconds_total
    );
        }
    }
    println!("Finished with {} failures out of {}", failures, count);
    Ok(())
}

fn create_connection() -> rusqlite::Result<Connection> {
    Connection::open("quiz_questions.db")
}
