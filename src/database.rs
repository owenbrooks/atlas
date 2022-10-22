use std::path::PathBuf;

use rusqlite::Connection;

/// Adds new record for song title. Returns track_id.
/// If track is already in table, returns existing id
pub fn add_track(conn: &Connection, title: &str) -> Result<u32, anyhow::Error> {
    let id: Result<u32, rusqlite::Error> = conn.query_row(
        "SELECT rowid from tracks WHERE title = (?)",
        &[&title.to_string()],
        |row| row.get(0),
    );

    let insert_track = |conn: &Connection, title: &str| {
        let insertion_result = conn.execute(
            "INSERT INTO tracks (title) VALUES (?1)",
            &[&title.to_string()],
        );
        insertion_result.unwrap(); // TODO: handle this error
        let id = conn.last_insert_rowid() as u32;
        id
    };

    let id = id.unwrap_or_else(|_| insert_track(conn, title));

    Ok(id)
}

/// Opens the specified database, creating it and the tables if it doesn't yet exist
pub fn connect(database: &PathBuf) -> Result<Connection, anyhow::Error> {
    let conn = Connection::open(database)?;

    // Create tracks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL
        )",
        (), // empty list of parameters.
    )?;
    // Create fingerprints table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS fingerprints (
            id INTEGER PRIMARY KEY,
            hash INTEGER NOT NULL,
            track_time INTEGER NOT NULL,
            track_id INTEGER NOT NULL
        )",
        (),
    )?;

    Ok(conn)
}
