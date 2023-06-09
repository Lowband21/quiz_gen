import os
import re
import sqlite3
import nltk
from nltk.corpus import stopwords
from nltk.tokenize import word_tokenize

nltk.download('punkt')
nltk.download('stopwords')

def parse_sections(input_folder):
    sections = {}
    for section_name in os.listdir(input_folder):
        section_path = os.path.join(input_folder, section_name)
        if os.path.isdir(section_path):
            chapters = {}
            for chapter_name in os.listdir(section_path):
                chapter_path = os.path.join(section_path, chapter_name)
                with open(chapter_path, 'r') as file:
                    text = file.read()
                    cleaned_text = re.sub(r'[^\w\s]', '', text)
                    cleaned_text = cleaned_text.lower()
                    tokens = word_tokenize(cleaned_text)
                    stop_words = set(stopwords.words('english'))
                    filtered_tokens = [token for token in tokens if token not in stop_words]
                    chapters[chapter_name] = filtered_tokens
            sections[section_name] = chapters
    return sections

def create_db(db_name):
    conn = sqlite3.connect(db_name)
    return conn

def create_tables(conn):
    cursor = conn.cursor()
    
    cursor.execute('''
        CREATE TABLE IF NOT EXISTS sections (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL
        )
    ''')

    cursor.execute('''
        CREATE TABLE IF NOT EXISTS chapters (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            content TEXT NOT NULL,
            section_id INTEGER,
            FOREIGN KEY (section_id) REFERENCES sections (id)
        )
    ''')

    conn.commit()

def insert_section(conn, section_name):
    cursor = conn.cursor()
    cursor.execute("INSERT INTO sections (name) VALUES (?)", (section_name,))
    conn.commit()
    return cursor.lastrowid

def insert_chapter(conn, chapter_name, content, section_id):
    cursor = conn.cursor()
    cursor.execute("INSERT INTO chapters (name, content, section_id) VALUES (?, ?, ?)", (chapter_name, content, section_id))
    conn.commit()

# Replace 'path/to/sections' with the actual path to the sections folder
sections_folder = './sections'
parsed_sections = parse_sections(sections_folder)

# Create and set up the SQLite database
db_name = 'sections_chapters.db'
conn = create_db(db_name)
create_tables(conn)

# Insert the preprocessed sections and chapters into the database
for section_name, chapters in parsed_sections.items():
    print(section_name)
    section_id = insert_section(conn, section_name)
    
    for chapter_name, preprocessed_content in chapters.items():
        content = ' '.join(preprocessed_content)
        insert_chapter(conn, chapter_name, content, section_id)

# Close the database connection
conn.close()
