import pickle
from gensim.models import LdaModel

# Load the saved data from pickle files
def load_data(file_prefix):
    with open(f'{file_prefix}_dictionary.pkl', 'rb') as f:
        dictionary = pickle.load(f)

    with open(f'{file_prefix}_corpus.pkl', 'rb') as f:
        corpus = pickle.load(f)

    with open(f'{file_prefix}_corpus_tfidf.pkl', 'rb') as f:
        corpus_tfidf = pickle.load(f)

    return dictionary, corpus, corpus_tfidf

# Load data from pickle files
file_prefix = 'your_data'
dictionary, corpus, corpus_tfidf = load_data(file_prefix)

# Train the LDA model
num_topics = 200  # Change this value to experiment with the number of topics
passes = 2  # Change this value to experiment with the number of passes

lda_model = LdaModel(corpus, num_topics=num_topics, id2word=dictionary, passes=passes)

# Function to extract the most relevant keywords and topics from the LDA model
def analyze_lda_model(lda_model, num_keywords=10):
    topic_keywords = {}

    for topic_id in range(lda_model.num_topics):
        keywords = lda_model.show_topic(topic_id, num_keywords)
        topic_keywords[topic_id] = [keyword[0] for keyword in keywords]

    return topic_keywords

# Extract the most relevant keywords and topics
num_keywords = 10  # Change this value to experiment with the number of keywords per topic
topic_keywords = analyze_lda_model(lda_model, num_keywords=num_keywords)

# Print the extracted keywords for each topic
with open("keywords.txt", 'w') as f:
    for topic_id, keywords in topic_keywords.items():
        f.write(f"{', '.join(keywords)}\n")
        print(f"Topic {topic_id}: {', '.join(keywords)}")
    

log_perplexity = lda_model.log_perplexity(corpus)
perplexity = 2**(-log_perplexity)
print("Perplexity: ", perplexity)

import openai
import re

openai.api_key = "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"

# Add this function to extract keywords from the top N topics
def extract_keywords(lda_model, num_topics=3, num_words=5):
    topics = lda_model.show_topics(num_topics=num_topics, num_words=num_words, formatted=False)
    keywords = set()
    for topic in topics:
        for word, _ in topic[1]:
            keywords.add(word)
    return keywords

# Extract keywords from the LDA model (use the lda_model you've trained earlier)
keywords = extract_keywords(lda_model)



