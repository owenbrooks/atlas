use std::path::PathBuf;

use rusqlite::Connection;

use crate::hash::PeakPair;

pub fn create_track_table(conn: &Connection) {}

pub fn create_fingerprint_table(conn: &Connection) {}

pub fn clear_fingerprints_by_track(conn: &Connection, track_id: u64) {}

/// Adds new record for song title. Returns track_id.
/// If track is already in table, returns existing id
pub fn add_track(conn: &Connection, title: &str) -> Result<u64, anyhow::Error> {
    let id: Result<u64, rusqlite::Error> = conn.query_row(
        "SELECT rowid from track_info WHERE title = (?)",
        &[&title.to_string()],
        |row| Ok(row.get(0)),
    )?;

    let id = id.unwrap_or({
        conn.execute(
            "INSERT INTO track_info (title) VALUES (?1)",
            &[&title.to_string()],
        )?;
        let id = conn.last_insert_rowid() as u64;
        id
    });

    Ok(id)
}

pub fn add_print_for_track(pair: PeakPair, time: usize, track_id: u64) {}

/// Opens the specified database, creating it and the tables if it doesn't yet exist
pub fn connect(database: &PathBuf) -> Result<Connection, anyhow::Error> {
    let conn = Connection::open(database)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS track_info (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL
        )",
        (), // empty list of parameters.
    )?;

    Ok(conn)
}
