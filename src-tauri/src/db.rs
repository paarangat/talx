use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug)]
pub struct TranscriptionRow {
    pub id: String,
    pub created_at: i64,
    pub original_text: String,
    pub polished_text: String,
    pub word_count: u32,
    pub duration_secs: u32,
}

pub fn init_db(app_data_dir: &Path) -> Result<Connection> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let db_path = app_data_dir.join("talx.db");
    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transcriptions (
            id            TEXT PRIMARY KEY,
            created_at    INTEGER NOT NULL,
            original_text TEXT NOT NULL,
            polished_text TEXT NOT NULL,
            word_count    INTEGER NOT NULL,
            duration_secs INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_transcriptions_created_at
            ON transcriptions(created_at DESC);
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;

    Ok(conn)
}

pub fn save(
    conn: &Connection,
    original_text: &str,
    polished_text: &str,
    word_count: u32,
    duration_secs: u32,
) -> Result<TranscriptionRow> {
    let id = Uuid::new_v4().to_string();
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT INTO transcriptions (id, created_at, original_text, polished_text, word_count, duration_secs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, created_at, original_text, polished_text, word_count, duration_secs],
    )?;

    Ok(TranscriptionRow {
        id,
        created_at,
        original_text: original_text.to_string(),
        polished_text: polished_text.to_string(),
        word_count,
        duration_secs,
    })
}

pub fn get(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<TranscriptionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, original_text, polished_text, word_count, duration_secs
         FROM transcriptions
         ORDER BY created_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(TranscriptionRow {
            id: row.get(0)?,
            created_at: row.get(1)?,
            original_text: row.get(2)?,
            polished_text: row.get(3)?,
            word_count: row.get::<_, u32>(4)?,
            duration_secs: row.get::<_, u32>(5)?,
        })
    })?;

    rows.collect()
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM transcriptions WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn clear(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM transcriptions", [])?;
    Ok(())
}

#[derive(Serialize, Clone, Debug)]
pub struct TodayStats {
    pub words: u64,
    pub recording_secs: u64,
    pub sessions: u64,
}

// --- Settings (key-value store for API keys etc.) ---

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}

pub fn get_today_stats(conn: &Connection, since_ms: i64) -> Result<TodayStats> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(SUM(word_count), 0), COALESCE(SUM(duration_secs), 0), COUNT(*)
         FROM transcriptions
         WHERE created_at >= ?1",
    )?;

    stmt.query_row(params![since_ms], |row| {
        Ok(TodayStats {
            words: row.get::<_, i64>(0)? as u64,
            recording_secs: row.get::<_, i64>(1)? as u64,
            sessions: row.get::<_, i64>(2)? as u64,
        })
    })
}
