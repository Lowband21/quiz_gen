//use eval::similarity::similarity;
use quiz::quiz_parser::QuizQuestion;

use query::query::*;
use requestty::Question;

use rusqlite::Connection;

mod eval {
    pub mod gpt;
}

mod query {
    pub mod query;
}

mod quiz {
    pub mod quiz;
    pub mod quiz_parser;
}

pub fn pretty_print(quiz_questions: Vec<QuizQuestion>) {
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
        .choices(vec!["evaluate"])
        .build();
    let operation_mode_choice = &requestty::prompt_one(operation_mode_question).unwrap();
    operation_mode_choice.as_list_item().unwrap().clone().text
}

fn main() {
    // Ask for the operation mode
    let operation_mode = select_operation_mode();

    match operation_mode.as_str() {
        "evaluate" => {
            let filenames_question = Question::multi_select("filenames")
                .message("Select the filenames associated with the questions to be evaluated:")
                .choices(
                    get_unique_filenames(&Connection::open("quiz_questions.db").unwrap()).unwrap(),
                )
                .build();

            let filenames_answer = &requestty::prompt_one(filenames_question).unwrap();

            let filenames: Vec<String> = filenames_answer
                .as_list_items()
                .unwrap()
                .into_iter()
                .map(|item| item.text.clone())
                .collect();

            eval::gpt::run(filenames).unwrap();
        }
        _ => {
            panic!("Invalid operation mode selected");
        }
    }
}
