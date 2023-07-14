use rusqlite::Result;
use rusqlite::{params, Connection};
use std::error::Error;

#[derive(Debug)]
pub struct DatabaseQuestion {
    id: i32,
    prompt: String,
    response: String,
    filename: String,
    question_type: String,
    difficulty: Option<String>,
}

pub fn get_unique_filenames(conn: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT DISTINCT filename FROM quiz")?;

    let filename_rows = stmt.query_map([], |row| Ok(row.get(0)?))?;

    let mut filenames = Vec::new();
    for filename_row in filename_rows {
        filenames.push(filename_row?);
    }

    Ok(filenames)
}

pub fn query_by_filename(
    conn: &Connection,
    filename: &str,
) -> Result<Vec<DatabaseQuestion>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT * FROM quiz WHERE filename = ?1")?;

    let quiz_rows = stmt.query_map(params![filename], |row| {
        Ok(DatabaseQuestion {
            id: row.get(0)?,
            prompt: row.get(1)?,
            response: row.get(2)?,
            filename: row.get(3)?,
            question_type: row.get(4)?,
            difficulty: row.get(5)?,
        })
    })?;

    let mut quizzes = Vec::new();
    for quiz_row in quiz_rows {
        quizzes.push(quiz_row?);
    }

    Ok(quizzes)
}

pub fn get_top_10(
    conn: &Connection,
) -> Result<Vec<(i32, String, i32, i32, i32, i32, i32)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT results.id, quiz.response, results.relevance, results.complexity, results.clarity, results.creativity, results.total_score
        FROM results
        JOIN quiz ON results.id = quiz.id
        ORDER BY results.total_score DESC
        LIMIT 10",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?, // id
            row.get(1)?, // response
            row.get(2)?, // relevance score
            row.get(3)?, // complexity score
            row.get(4)?, // clarity score
            row.get(5)?, // creativity score
            row.get(6)?, // total_score
        ))
    })?;

    let mut top_10 = Vec::new();
    for row_result in rows {
        let row = row_result?;
        top_10.push(row);
    }

    Ok(top_10)
}

pub fn get_statistics() -> Result<
    (
        (Option<f64>, Option<i32>, Option<i32>, Option<f64>), // Total Score
        (Option<f64>, Option<i32>, Option<i32>, Option<f64>), // Relevance
        (Option<f64>, Option<i32>, Option<i32>, Option<f64>), // Complexity
        (Option<f64>, Option<i32>, Option<i32>, Option<f64>), // Clarity
        (Option<f64>, Option<i32>, Option<i32>, Option<f64>),
    ), // Creativity
    Box<dyn Error>,
> {
    let conn = Connection::open("quiz_questions.db")?;

    let categories = vec![
        "total_score",
        "relevance",
        "complexity",
        "clarity",
        "creativity",
    ];

    let mut results = Vec::new();

    for category in categories {
        let avg: Option<f64> = conn.query_row(
            &format!("SELECT AVG({}) FROM results", category),
            params![],
            |row| row.get(0),
        )?;

        let min: Option<i32> = conn.query_row(
            &format!("SELECT MIN({}) FROM results", category),
            params![],
            |row| row.get(0),
        )?;

        let max: Option<i32> = conn.query_row(
            &format!("SELECT MAX({}) FROM results", category),
            params![],
            |row| row.get(0),
        )?;

        let median: Option<f64> = conn.query_row(&format!("SELECT {} FROM results ORDER BY {} LIMIT 1 OFFSET (SELECT COUNT(*) FROM results) / 2", category, category), params![], |row| row.get(0))?;

        results.push((avg, min, max, median));
    }

    Ok((results[0], results[1], results[2], results[3], results[4]))
}
