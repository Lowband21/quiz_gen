use eval::similarity::similarity;
use quiz::quiz::quiz;
use quiz::quiz_parser::{parse_quiz_file, QuizQuestion};

use gen::explanation_gen::generate_explanations;
use gen::quiz_gen::*;
use openai_api_rust::*;
use requestty::Question;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

mod eval {
    pub mod bard;
    pub mod common_words;
    pub mod gpt;
    pub mod similarity;
    pub mod stat;
}

mod gen {
    pub mod explanation_gen;
    pub mod quiz_gen;
}

mod quiz {
    pub mod quiz;
    pub mod quiz_parser;
}

mod transcription {
    pub mod whisper;
}

pub fn pretty_print(quiz_questions: Vec<QuizQuestion>) {
    println!("Trying to print");
    for (i, question) in quiz_questions.iter().enumerate() {
        println!("Question {}: {}", i + 1, question.question);
        println!("Possible Answers: {}", question.answer);
        println!("Key: {}", question.key);
        println!("---------------------");
    }
}

fn select_operation_mode() -> String {
    let operation_mode_question = Question::select("operation_mode")
        .message("Choose an operation mode:")
        .choices(vec![
            "generate",
            "parse",
            "quiz",
            "analysis",
            "transcription",
        ])
        .build();
    let operation_mode_choice = &requestty::prompt_one(operation_mode_question).unwrap();
    operation_mode_choice.as_list_item().unwrap().clone().text
}

fn select_directory(directories: Vec<String>) -> String {
    let directory_question = Question::select("directory")
        .message("Choose a directory:")
        .choices(directories)
        .build();
    let directory_choice =
        &requestty::prompt_one(directory_question).expect("No directory selected");
    directory_choice.as_list_item().unwrap().clone().text
}

// Function to read files from a directory
fn read_files(directory_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = Vec::new();

    // Use WalkDir to recursively get all files in the directory
    for entry in WalkDir::new(directory_path) {
        let entry = entry?;

        // Check if the entry is a file
        if entry.file_type().is_file() {
            // Get the file name as a string and push it to the vector
            let file_name = entry
                .path()
                .to_str()
                .ok_or("Failed to read file name")?
                .to_string();
            files.push(file_name);
        }
    }

    // Print debug info
    println!("Found files: {:?}", files);

    Ok(files)
}

// Function to read directories from a specified path
fn read_directories(directory_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Try to read the directory and handle error if it fails
    let read_dir = fs::read_dir(directory_path).map_err(|e| {
        eprintln!("Failed to read directory {:?}: {}", directory_path, e);
        e
    })?;

    // Try to read each entry (directory) in the directory and collect to Vec
    let directories: Vec<String> = read_dir
        .filter_map(|entry| {
            match entry {
                Ok(d) => {
                    // Check if the entry is a directory
                    if d.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        Some(d.file_name().to_string_lossy().into_owned())
                    } else {
                        println!(
                            "\"{}\" is not a directory, skipping",
                            d.file_name().to_string_lossy()
                        );
                        None
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read directory entry: {}", e);
                    None
                }
            }
        })
        .collect();

    // Print debug info
    println!("Found directories: {:?}", directories);

    Ok(directories)
}

// Function to select files from a list
fn select_files(files: Vec<String>) -> Vec<String> {
    let file_question = Question::multi_select("files")
        .message("Choose one or more files:")
        .choices(files)
        .build();

    let file_choices = &requestty::prompt_one(file_question).unwrap();
    file_choices
        .as_list_items()
        .unwrap()
        .into_iter()
        .map(|item| item.text.clone())
        .collect()
}

// Function to select question type
fn select_question_type() -> String {
    let question_type_question = Question::select("question_type")
        .message("Choose a question type:")
        .choices(vec!["multiple_choice", "short_response"])
        .build();

    let question_type_choice = &requestty::prompt_one(question_type_question).unwrap();
    question_type_choice.as_list_item().unwrap().clone().text
}

// Function to select difficulty level
fn select_difficulty_level() -> String {
    let difficulty_level_question = Question::select("difficulty_level")
        .message("Choose a difficulty level:")
        .choices(vec!["easy", "medium", "hard"])
        .build();

    let difficulty_level_choice = &requestty::prompt_one(difficulty_level_question).unwrap();
    difficulty_level_choice.as_list_item().unwrap().clone().text
}

// Function to select generation mode
fn select_generation_mode() -> String {
    let generation_mode_question = Question::select("generation_mode")
        .message("Choose a generation mode:")
        .choices(vec!["quiz", "explanation"])
        .build();

    let generation_mode_choice = &requestty::prompt_one(generation_mode_question).unwrap();
    generation_mode_choice.as_list_item().unwrap().clone().text
}

// Function to select difficulty level
fn select_n_gen() -> i64 {
    let n_gen_question = Question::int("n_gen")
        .message("Choose a number of questions to generate per block of content:")
        .build();

    let n_gen_choice = &requestty::prompt_one(n_gen_question).unwrap();
    n_gen_choice.clone().try_into_int().unwrap()
}

// Function to prepare output directory
fn prepare_output_dir(path: &str) -> PathBuf {
    let output_dir = Path::new(path);
    fs::create_dir_all(&output_dir).unwrap();
    output_dir.to_path_buf()
}

fn main() {
    // Load API key from environment OPENAI_API_KEY.
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");

    // Ask for the operation mode
    let operation_mode = select_operation_mode();

    match operation_mode.as_str() {
        "analysis" => {
            eval::bard::run();
            eval::common_words::run();
            eval::gpt::run();
            eval::similarity::similarity();
            eval::stat::run();
        }
        "transcription" => {
            let gpu_or_cpu = Question::select("difficulty_level")
                .message("Select a method of running whisper:")
                .choices(vec!["CPU (Rust)", "GPU (Python)"])
                .build();

            let gpu_or_cpu = &requestty::prompt_one(gpu_or_cpu).unwrap();

            match gpu_or_cpu.as_list_item().unwrap().clone().text.as_str() {
                "CPU (Rust)" => {
                    let model_size_question = Question::select("model_size")
                        .message("Select a model size:")
                        .choices(vec![
                            "tiny",
                            "tiny.en",
                            "base",
                            "base.en",
                            "small",
                            "small.en",
                            "medium",
                            "medium.en",
                            "large-v1",
                            "large",
                        ])
                        .build();
                    let model_size_answer = &requestty::prompt_one(model_size_question).unwrap();
                    let model_size = model_size_answer.as_list_item().unwrap().clone().text;
                    transcription::whisper::transcribe_all(model_size);
                }
                "GPU (Python)" => {
                    transcription::whisper::py_whisper();
                }
                _ => panic!("Impossible choice"),
            }
        }
        "generate" => {
            // Read directories from the "input" folder
            let directories =
                read_directories(Path::new("input")).expect("Failed to read directories");
            for i in directories.clone() {
                println!("Something: {}", i);
            }

            let directory_name = select_directory(directories);

            // Read files from the selected directory
            let files = read_files(&Path::new("input").join(directory_name.clone()))
                .expect("Failed to read files");

            let selected_files = select_files(files);

            // Ask for the difficulty level
            let difficulty_level = select_difficulty_level();

            // Ask for the generation mode
            let generation_mode = select_generation_mode();

            let mut question_type = String::from("None");

            if generation_mode.as_str() == "quiz" {
                // Ask for the question type
                question_type = select_question_type();
            }

            let n_gen = select_n_gen();

            // Generate content for each selected file
            for file in selected_files {
                let file_path = Path::new(file.as_str());
                println!("File Path {:?}:", file_path);
                let content = fs::read_to_string(file_path).unwrap();

                let preprocessed_content = preprocess_content(&content);

                for i in 0..n_gen {
                    // Determine the generation mode and generate content accordingly
                    match generation_mode.as_str() {
                        "quiz" => {
                            let processed_content = generate_quiz_questions(
                                &openai,
                                &preprocessed_content,
                                &question_type,
                                &difficulty_level,
                            )
                            .unwrap();
                            // Write the output content to the file
                            let output_string = format!(
                                "{}",
                                file.trim_end_matches(".md").trim_end_matches(".txt"),
                            );

                            let split_strings: Vec<_> = output_string.split('/').collect();
                            let output_file_name = format!(
                                "{}_{}_{}",
                                generation_mode,
                                question_type,
                                split_strings.last().unwrap_or(&"")
                            );

                            let output_file_ext = format!("_{}.txt", i);

                            let output_file_path = save_processed(
                                processed_content,
                                output_file_name.to_string(),
                                output_file_ext,
                            );
                            let (quiz_questions, success_rate) =
                                parse_quiz_file(output_file_path.as_path()).unwrap();

                            pretty_print(quiz_questions);
                            println!("The failure rate was {}", success_rate);
                        }
                        "explanation" => {
                            let processed_content =
                                generate_explanations(&openai, &preprocessed_content);
                            // Write the output content to the file
                            let output_file_name = format!(
                                "{}_{}",
                                generation_mode,
                                file.trim_end_matches(".md").trim_end_matches(".txt")
                            );
                            let output_file_ext = format!("_{}.txt", i);
                            let _output_file_path = save_processed(
                                processed_content,
                                output_file_name,
                                output_file_ext,
                            );
                            todo!("Parse explanation output");
                        }
                        _ => panic!("Invalid generation mode selected"),
                    }
                }
            }
        }
        "parse" => {
            let output_files =
                read_files(Path::new("output")).expect("Failed to read output files");
            let selected_files = select_files(output_files);

            parse_files_and_output(selected_files, "output", "parsed_quizzes"); // This will be another helper function like the ones above
        }
        "quiz" => {
            let quizzes_files =
                read_files(Path::new("parsed_quizzes")).expect("Failed to read parsed quiz files");
            let selected_files = select_files(quizzes_files);

            run_quiz(selected_files, "parsed_quizzes").expect("Failed to run quiz");
            // This will be another helper function like the ones above
        }
        _ => {
            panic!("Invalid operation mode selected");
        }
    }
}

fn save_processed(
    processed_content: Vec<String>,
    output_file_name: String,
    output_file_ext: String,
) -> PathBuf {
    // Prepare the output directory
    let output_dir = prepare_output_dir(format!("output/{}", output_file_name).as_str());

    // Prepare the content to be written to the file
    let mut output_content = String::new();
    for item in processed_content {
        output_content.push_str(&format!("{}\n", item));
    }

    let output_file_path = output_dir.join(format!("{}{}", output_file_name, output_file_ext));
    println!("{:?}", output_file_path);
    fs::write(&output_file_path, output_content).expect("Unable to write file");
    output_file_path
}

use walkdir::WalkDir;

fn parse_files_and_output(selected_files: Vec<String>, input_dir: &str, output_dir: &str) {
    // Prepare the output directory
    let output_dir = prepare_output_dir(output_dir);

    // Parse each selected file
    for file in selected_files {
        let file_path = Path::new(&file);
        println!("File Path: {:?}", file_path);
        let (quiz_questions, success_rate) = parse_quiz_file(file_path).unwrap();
        pretty_print(quiz_questions.clone());
        println!("The failure rate was {}", success_rate);

        // Write the questions, answers, and keys to a file
        let mut output_content = String::new();
        for question in quiz_questions {
            output_content.push_str(&format!(
                "Question:\n{}\nPossible Answers:\n{}\nKey:\n{}\n------------------\n\n",
                question.question.trim(),
                question.answer.trim(),
                question.key
            ));
        }

        // Derive the output directory path without the file name
        let output_dir_path = output_dir.join(
            Path::new(&file.trim_start_matches("output/"))
                .parent()
                .unwrap(),
        );
        prepare_output_dir(&output_dir_path.to_str().unwrap());

        println!("{:?}", &file.trim_start_matches("output/"));
        fs::write(
            output_dir.join(&file.trim_start_matches("output/")),
            output_content,
        )
        .expect("Unable to write file");
    }
}

fn run_quiz(selected_files: Vec<String>, quiz_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    for i in selected_files {
        let quiz_file_path = Path::new(quiz_dir).join(i);
        // Run the quiz for each selected file
        quiz(quiz_file_path)?;
    }
    Ok(())
}
