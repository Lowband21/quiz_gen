use rusqlite::{Connection, Result};
use std::collections::HashSet;
use std::error::Error;

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

fn count_common_words(prompt: &str, question: &str) -> f64 {
    let prompt_words: HashSet<_> = prompt.split_whitespace().collect();
    let question_words: HashSet<_> = question.split_whitespace().collect();

    let common_words: HashSet<_> = prompt_words.intersection(&question_words).collect();

    let total_unique_words = prompt_words.union(&question_words).count();

    (common_words.len() as f64 / total_unique_words as f64) * 100.0
}

fn main() -> Result<(), Box<dyn Error>> {
    let quiz_tuples = read_quiz_questions()?;
    for quiz in &quiz_tuples {
        let common_word_percent = count_common_words(&quiz.1, &quiz.2);
        println!("Percentage of Common Words: {:.2}%", common_word_percent);
    }
    Ok(())
}
