mod explanation_gen;
mod quiz;
mod quiz_gen;
mod quiz_parser;
mod similarity;

use crate::similarity::similarity;
use quiz::quiz;
use quiz_parser::{parse_quiz_file, QuizQuestion};

use explanation_gen::generate_explanations;
use openai_api_rust::*;
use quiz_gen::*;
use requestty::{question::Choice, Answer, Question};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn pretty_print(quiz_questions: Vec<QuizQuestion>) {
    println!("Trying to print");
    for (i, question) in quiz_questions.iter().enumerate() {
        println!("Question {}: {}", i + 1, question.question);
        println!("Possible Answers: {}", question.answer);
        println!("Key: {}", question.key);
        println!("---------------------");
    }
}

fn main() {
    // Load API key from environment OPENAI_API_KEY.
    let auth = Auth::from_env().unwrap();
    let openai = OpenAI::new(auth, "https://api.openai.com/v1/");

    // Ask for the operation mode
    let operation_mode_question = Question::select("operation_mode")
        .message("Choose an operation mode:")
        .choices(vec!["generate", "parse", "quiz", "analysis"])
        .build();

    let operation_mode_choice = &requestty::prompt_one(operation_mode_question).unwrap();
    let operation_mode = operation_mode_choice.as_list_item().unwrap().clone().text;
    if operation_mode == "analysis" {
        similarity().unwrap();
    } else if operation_mode == "generate" {
        // Read directories from the "input" folder
        let input_path = Path::new("input");
        let directories: Vec<_> = fs::read_dir(input_path)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|d| d.file_type().unwrap().is_dir())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();

        let directory_question = Question::select("directory")
            .message("Choose a directory:")
            .choices(directories)
            .build();

        let directory_choice =
            &requestty::prompt_one(directory_question).expect("No directory selected");
        let directory_name = directory_choice.as_list_item().unwrap().clone().text;

        // Read files from the selected directory
        let files_path = input_path.join(directory_name);
        let files: Vec<_> = fs::read_dir(&files_path)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|d| d.file_type().unwrap().is_file())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();

        let file_question = Question::multi_select("files")
            .message("Choose one or more files:")
            .choices(files)
            .build();

        let file_choices = &requestty::prompt_one(file_question).unwrap();
        let selected_files = file_choices
            .as_list_items()
            .unwrap()
            .into_iter()
            .map(|item| &item.text)
            .collect::<Vec<_>>();

        // Ask for the question type
        let question_type_question = Question::select("question_type")
            .message("Choose a question type:")
            .choices(vec!["multiple_choice", "short_response"])
            .build();

        let question_type_choice = &requestty::prompt_one(question_type_question).unwrap();
        let question_type = question_type_choice.as_list_item().unwrap().clone().text;

        // Ask for the difficulty level
        let difficulty_level_question = Question::select("difficulty_level")
            .message("Choose a difficulty level:")
            .choices(vec!["easy", "medium", "hard"])
            .build();

        let difficulty_level_choice = &requestty::prompt_one(difficulty_level_question).unwrap();
        let difficulty_level = difficulty_level_choice.as_list_item().unwrap().clone().text;

        // Ask for the generation mode
        let generation_mode_question = Question::select("generation_mode")
            .message("Choose a generation mode:")
            .choices(vec!["quiz", "explanation"])
            .build();

        let generation_mode_choice = &requestty::prompt_one(generation_mode_question).unwrap();
        let generation_mode = generation_mode_choice.as_list_item().unwrap().clone().text;

        // Generate content for each selected file
        for file in selected_files {
            let file_path = files_path.join(file);
            let content = fs::read_to_string(file_path).unwrap();

            let preprocessed_content = preprocess_content(&content);
            let processed_content: Vec<String>;

            // Determine the generation mode and generate content accordingly
            match generation_mode.as_str() {
                "quiz" => {
                    processed_content = generate_quiz_questions(
                        &openai,
                        &preprocessed_content,
                        &question_type,
                        &difficulty_level,
                    ).unwrap();
                }
                "explanation" => {
                    processed_content = generate_explanations(&openai, &preprocessed_content);
                }
                _ => panic!("Invalid generation mode selected"),
            }

            println!("Got back content");

            // Prepare the output directory
            let output_dir = Path::new("output");
            fs::create_dir_all(&output_dir).unwrap();

            // Prepare the content to be written to the file
            let mut output_content = String::new();
            for item in processed_content {
                output_content.push_str(&format!("{}\n", item));
            }

            // Write the output content to the file
            let output_file_name =
                format!("{}_{}.txt", generation_mode, file.trim_end_matches(".md"));
            let output_file_path = output_dir.join(output_file_name);
            fs::write(&output_file_path, output_content).expect("Unable to write file");

            match generation_mode.as_str() {
                "quiz" => {
                    let (quiz_questions, success_rate) =
                        parse_quiz_file(output_file_path.as_path()).unwrap();

                    pretty_print(quiz_questions);
                    println!("The failure rate was {}", success_rate);
                }
                "explanation" => {
                    todo!();
                }
                _ => {
                    println!("Invalid generation mode selected");
                }
            }
        }
    } else if operation_mode == "parse" {
        // Read files from the "output" folder
        let output_path = Path::new("output");
        let files: Vec<_> = fs::read_dir(&output_path)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|d| d.file_type().unwrap().is_file())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();

        let file_question = Question::multi_select("files")
            .message("Choose one or more files:")
            .choices(files)
            .build();

        let file_choices = &requestty::prompt_one(file_question).unwrap();
        let selected_files = file_choices
            .as_list_items()
            .unwrap()
            .into_iter()
            .map(|item| &item.text)
            .collect::<Vec<_>>();

        // Parse each selected file
        for file in selected_files {
            let file_path = output_path.join(file);
            println!("File Path: {:?}", file_path);
            let (quiz_questions, success_rate) = parse_quiz_file(file_path.as_path()).unwrap();
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
            fs::write(Path::new("./parsed_quizzes/").join(file), output_content)
                .expect("Unable to write file");
        }
    } else if operation_mode == "quiz" {
        let quizzes_path = Path::new("parsed_quizzes");
        let files: Vec<_> = fs::read_dir(&quizzes_path)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|d| d.file_type().unwrap().is_file())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();
        let file_question = Question::multi_select("files")
            .message("Choose a file:")
            .choices(files)
            .build();
        let quiz_file_choice = &requestty::prompt_one(file_question).unwrap();
        let quiz_file_paths = quiz_file_choice
            .as_list_items()
            .unwrap()
            .into_iter()
            .map(|item| &item.text)
            .collect::<Vec<_>>();

        for i in quiz_file_paths {
            quiz(
                Path::new("./parsed_quizzes")
                    .join(Path::new(i))
                    .to_path_buf(),
            )
            .expect("Could not read quiz data");
        }

        /*
        let mut correct_answers = 0;
        for question in quiz_data {
            if question.ask() {
                correct_answers += 1;
            }
        }

        println!(
            "You answered {} out of {} questions correctly.",
            correct_answers,
            quiz_data.len()
        );
        */
    } else {
        panic!("Invalid operation mode selected");
    }
}
