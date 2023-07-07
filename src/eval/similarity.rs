// src/similarity.rs
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn similarity() -> std::io::Result<()> {
    let keywords = fs::read_to_string("./keywords.txt")?;
    let keywords_vec: Vec<String> = keywords.lines().map(|line| line.to_lowercase()).collect();

    let dir = Path::new("./parsed_quizzes");
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file = fs::File::open(&path)?;
                let reader = BufReader::new(file);

                let mut total_word_count = 0;
                let mut matching_word_count = 0;

                for line in reader.lines() {
                    let line = line?;
                    let words: Vec<String> = line
                        .split_whitespace()
                        .map(|word| word.to_lowercase())
                        .collect();
                    total_word_count += words.len();

                    for word in &words {
                        if keywords_vec.contains(&word) {
                            matching_word_count += 1;
                        }
                    }
                }

                if total_word_count > 0 {
                    let percentage = (matching_word_count as f64 / total_word_count as f64) * 100.0;
                    println!(
                        "For file '{}', {:.2}% of words match the keywords.",
                        path.display(),
                        percentage
                    );
                } else {
                    println!("For file '{}', no words were found.", path.display());
                }
            }
        }
    }

    Ok(())
}
