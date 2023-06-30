use rusqlite::{Connection, Result};
use std::collections::HashMap;
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

fn count_common_words(prompt: &str, question: &str) -> usize {
    let prompt_words: HashMap<_, _> = prompt.split_whitespace().map(|word| (word, ())).collect();
    let question_words: Vec<&str> = question.split_whitespace().collect();

    question_words
        .iter()
        .filter(|&word| prompt_words.contains_key(word))
        .count()
}

fn count_new_words(prompt: &str, question: &str) -> usize {
    let prompt_words: HashMap<_, _> = prompt.split_whitespace().map(|word| (word, ())).collect();
    let question_words: Vec<&str> = question.split_whitespace().collect();

    question_words
        .iter()
        .filter(|&word| !prompt_words.contains_key(word))
        .count()
}

fn calculate_quality_score(common_word_ratio: f32, new_word_ratio: f32, length: usize) -> f32 {
    let ideal_common_word_ratio = 0.40..0.60;
    let ideal_new_word_ratio = 0.40..0.60;
    let ideal_length = 20..50;

    let common_word_score = if ideal_common_word_ratio.contains(&common_word_ratio) {
        1.0
    } else {
        0.0
    };

    let new_word_score = if ideal_new_word_ratio.contains(&new_word_ratio) {
        1.0
    } else {
        0.0
    };

    let length_score = if ideal_length.contains(&length) {
        1.0
    } else {
        0.0
    };

    // We'll just take the average of these four scores for simplicity, but you could also weight them differently
    (common_word_score + new_word_score + length_score) / 3.0
}
fn count_syllables(word: &str) -> u32 {
    let mut syllables_count = 0;
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let chars: Vec<char> = word.chars().collect();
    let mut prev_char_was_vowel = false;

    for &c in chars.iter() {
        let is_vowel = vowels.contains(&c);

        if is_vowel && !prev_char_was_vowel {
            syllables_count += 1;
        }

        prev_char_was_vowel = is_vowel;
    }

    if word.ends_with('e') {
        syllables_count -= 1;
    }

    syllables_count.max(1)
}

fn readability_score(text: &str) -> f32 {
    let total_sentences = text.split(|c| ".!?".contains(c)).count() as f32;
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .collect();
    let total_words = words.len() as f32;
    let total_syllables = words
        .iter()
        .map(|&word| count_syllables(word) as f32)
        .sum::<f32>();

    206.835 - 1.015 * (total_words / total_sentences) - 84.6 * (total_syllables / total_words)
}

fn semantic_similarity(text1: &str, text2: &str) -> f32 {
    unimplemented!(); // Semantic similarity cannot be calculated without machine learning libraries
}

fn run() -> Result<(), Box<dyn Error>> {
    let quiz_tuples = read_quiz_questions()?;
    for (i, quiz) in quiz_tuples.iter().enumerate() {
        let common_word_count = count_common_words(&quiz.1, &quiz.2);
        let new_word_count = count_new_words(&quiz.1, &quiz.2);
        let length = quiz.2.split_whitespace().count();
        let total_word_count = quiz.2.split_whitespace().count();

        let common_word_ratio = common_word_count as f32 / total_word_count as f32;
        let new_word_ratio = new_word_count as f32 / total_word_count as f32;

        let quality_score =
            calculate_quality_score(common_word_ratio, new_word_ratio, total_word_count);

        let readability = readability_score(&quiz.2);
        //let similarity = semantic_similarity(&quiz.1, &quiz.2);

        println!("Question Number: {}", i + 1);
        println!("Quality Score: {}", quality_score);
        println!("Readability Score: {}", readability);
        //println!("Semantic Similarity: {}", similarity);
        println!("Common Word Count: {}", common_word_count);
        println!("New Word Count: {}", new_word_count);
        println!("Question Length: {}", length);
        println!();
    }

    Ok(())
}
