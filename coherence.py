
import nltk
from gensim.models import Word2Vec
from gensim.models.coherencemodel import CoherenceModel
from gensim.corpora import Dictionary
from gensim.utils import simple_preprocess
import os
nltk.download('punkt')

from gensim.models import LdaModel
def extract_topics(quiz, num_topics=10):
    # Create a gensim dictionary
    gensim_dict = Dictionary(quiz)

    # Convert quiz to bag-of-words format
    bow_corpus = [gensim_dict.doc2bow(text) for text in quiz]

    # Train an LDA model
    lda_model = LdaModel(corpus=bow_corpus, id2word=gensim_dict, num_topics=num_topics)
    
    # Extract topics
    topics = lda_model.show_topics(formatted=False)
    
    # Only keep the word IDs for each topic
    topics = [[word_id for word_id, _ in topic] for _, topic in topics]
    
    return topics

def compute_coherence(quiz):
    # Train a Word2Vec model on the tokenized quiz
    model = Word2Vec(sentences=quiz, vector_size=100, window=5, min_count=1, workers=4)

    # Create a gensim dictionary
    gensim_dict = Dictionary(quiz)

    # Compute coherence
    coherence_model = CoherenceModel(topics=quiz, texts=quiz, dictionary=gensim_dict, coherence='u_mass')
    coherence = coherence_model.get_coherence()

    return coherence

def read_quiz_file(file_path):
    with open(file_path, 'r') as file:
        lines = file.read().splitlines()

    # Split into questions, answers, and keys
    questions = lines[::3]
    answers = lines[1::3]
    keys = lines[2::3]

    # Tokenize the sentences
    tokenized_questions = [nltk.word_tokenize(question) for question in questions]
    tokenized_answers = [nltk.word_tokenize(answer) for answer in answers]
    tokenized_keys = [nltk.word_tokenize(key) for key in keys]

    return tokenized_questions, tokenized_answers, tokenized_keys

import concurrent.futures

# Modify your coherence computation function to include averaging over 100 runs
def compute_coherence_avg(topics, num_runs=10000):
    total = 0
    for _ in range(num_runs):
        total += compute_coherence(topics)
    return total / num_runs

def process_file(file):
    questions, answers, keys = read_quiz_file("./parsed_quizzes/"  + file)
    topics = extract_topics(questions)
    return compute_coherence_avg(topics)

files = os.listdir("./parsed_quizzes")

# Create a ThreadPoolExecutor
with concurrent.futures.ThreadPoolExecutor() as executor:
    # Use the executor to map the process_file function to the files
    results = list(executor.map(process_file, files))

# Now results is a list of the average coherence for each file
for file, result in zip(files, results):
    print(f"{file}: {result}")
