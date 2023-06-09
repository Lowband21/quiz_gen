import openai
import re
import os
import nltk
import logging
from datetime import datetime
from nltk.tokenize import sent_tokenize
from prompt_toolkit import PromptSession
from prompt_toolkit.completion import WordCompleter

nltk.download('punkt')
openai.api_key = os.environ.get("OPENAI_API_KEY")

# Configure the logging
logging.basicConfig(filename='openai_api_logs.log', level=logging.INFO)

def log_api_call(prompt, api_parameters):
    """
    Log the API call details including the prompt and API parameters.
    """
    log_message = f"{datetime.now()}: Prompt: {prompt}, API parameters: {api_parameters}"
    logging.info(log_message)

def preprocess_content(content):
    """
    Preprocess the content by removing unnecessary whitespace, special characters,
    and splitting it into smaller sections.
    """
    content = re.sub('\s+', ' ', content)
    content = re.sub('[^0-9a-zA-Z.,;:?!]+', ' ', content)

    sections = sent_tokenize(content)
    return sections

def question_difficulty(difficulty_level):
    """
    Convert difficulty_level from string to integer.
    """
    difficulty_mapping = {
        "easy": 1,
        "medium": 2,
        "hard": 3
    }

    return difficulty_mapping.get(difficulty_level, 1)

def generate_question(prompt):
    api_parameters = {
        "model": "gpt-3.5-turbo",
        "messages": [{"role": "system", "content": "You are a helpful assistant that generates quiz questions."},
                     {"role": "user", "content": prompt}],
        "max_tokens": 1000,
        "temperature": 0.8,
        "top_p": 1,
        "frequency_penalty": 0,
        "presence_penalty": 0,
    }

    response = openai.ChatCompletion.create(**api_parameters)

    question = response['choices'][0]['message']['content']

    # Log the API call
    log_api_call(prompt, api_parameters)

    return question

def generate_quiz_questions(parsed_content, question_type, difficulty_level):
    questions = []
    for idx, section in enumerate(parsed_content, start=1):
        prompt = f"Create a {question_type} question with difficulty {difficulty_level} about the following text: {section} in the format \"#### Question:[]\n#### Answers:[]\n#### Key:[] \""
        question = generate_question(prompt)
        questions.append(f"{idx}. {question}")

    return questions

def separate_topics(content):
    prompt = f"Separate the following content into descrete topics. Each topic in the output should be separated by a single \"|\" character. Simply separate the topics as best you can, do not change the meaning of the content. Content: {content}"
    question = generate_question(prompt)

    print("GPT Output topics: ")
    for i, topic in enumerate(question.split("|")):
        print("Topic: {}".format(topic))
    
    return question.split("|")

def main():
    session = PromptSession()

    # Get user input interactively
    content = session.prompt("Enter the text you want to generate questions for: ")

    question_type_completer = WordCompleter(["multiple_choice", "short_response"], ignore_case=True)
    question_type = session.prompt("Choose a question type (multiple_choice or short_response): ", completer=question_type_completer)

    difficulty_level_completer = WordCompleter(["easy", "medium", "hard"], ignore_case=True)
    difficulty_level = session.prompt("Choose the difficulty level (easy, medium, or hard): ", completer=difficulty_level_completer)

    preprocessed_content = separate_topics(content)
    quiz_questions = generate_quiz_questions(preprocessed_content, question_type, difficulty_level)

    print("\nGenerated quiz questions:")
    for question in quiz_questions:
        print(question)

if __name__ == "__main__":
    main()
