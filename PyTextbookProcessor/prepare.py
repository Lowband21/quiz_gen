import sqlite3
import gensim
from gensim.corpora import Dictionary
from gensim.models import TfidfModel

# Fetch chapters from SQLite database
def get_chapters_from_db(db_file):
    conn = sqlite3.connect(db_file)
    cursor = conn.cursor()

    cursor.execute("SELECT content FROM chapters")
    chapters = cursor.fetchall()

    conn.close()
    return chapters

# Replace 'your_db_file.db' with the name of your SQLite database file
db_file = 'sections_chapters.db'

# Get chapters from the database
chapters_from_db = get_chapters_from_db(db_file)

# Assuming the chapters are tokenized and stored as strings, we need to convert them back to lists
preprocessed_chapters = [chapter[0].split() for chapter in chapters_from_db]

# Create a Dictionary from the preprocessed sections
dictionary = Dictionary(preprocessed_chapters)

# Filter out words that occur in less than 20 documents or more than 50% of the documents.
dictionary.filter_extremes(no_below=1, no_above=0.5)

# Create a Bag-of-Words representation of the preprocessed sections
corpus = [dictionary.doc2bow(chapter) for chapter in preprocessed_chapters]

# Optional: you can also create a TF-IDF model and use it to transform the corpus.
# This will give higher weight to less frequent words.
tfidf_model = TfidfModel(corpus)
corpus_tfidf = tfidf_model[corpus]

import pickle

# Save dictionary, corpus, and optional TF-IDF model to pickle files
def save_data(dictionary, corpus, corpus_tfidf, file_prefix):
    with open(f'{file_prefix}_dictionary.pkl', 'wb') as f:
        pickle.dump(dictionary, f)

    with open(f'{file_prefix}_corpus.pkl', 'wb') as f:
        pickle.dump(corpus, f)

    with open(f'{file_prefix}_corpus_tfidf.pkl', 'wb') as f:
        pickle.dump(corpus_tfidf, f)

# Save data to pickle files with a specified prefix (e.g., 'your_data')
file_prefix = 'your_data'
save_data(dictionary, corpus, corpus_tfidf, file_prefix)
