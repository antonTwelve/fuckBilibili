use rusqlite::{params, Connection, Result};
use std::path::Path;

pub fn init_db<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            mid INTEGER PRIMARY KEY,
            username TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bv_cache (
            bvid TEXT PRIMARY KEY,
            mid INTEGER,
            updated_at INTEGER
        )",
        [],
    )?;

    Ok(conn)
}

pub fn add_user(conn: &Connection, mid: i64, username: Option<&str>) -> Result<bool> {
    let mut stmt = conn.prepare("INSERT OR IGNORE INTO users (mid, username) VALUES (?, ?)")?;
    let rows = stmt.execute(params![mid, username])?;
    Ok(rows > 0)
}

pub fn remove_user(conn: &Connection, mid: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM users WHERE mid = ?", params![mid])?;
    Ok(rows > 0)
}

pub fn is_user_exist(conn: &Connection, mid: i64) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM users WHERE mid = ?")?;
    let exists = stmt.exists(params![mid])?;
    Ok(exists)
}

pub fn get_mid_by_bv(conn: &Connection, bvid: &str) -> Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT mid FROM bv_cache WHERE bvid = ?")?;
    let mut rows = stmt.query(params![bvid])?;
    
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn cache_bv_mid(conn: &Connection, bvid: &str, mid: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO bv_cache (bvid, mid, updated_at) VALUES (?, ?, ?)",
        params![bvid, mid, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}

pub fn get_blocked_count(conn: &Connection) -> Result<usize> {
    let count: usize = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    Ok(count)
}

pub fn get_bv_cache_count(conn: &Connection) -> Result<usize> {
    let count: usize = conn.query_row("SELECT COUNT(*) FROM bv_cache", [], |row| row.get(0))?;
    Ok(count)
}

pub fn clean_expired_cache(conn: &Connection, expiration_secs: i64) -> Result<usize> {
    let threshold = chrono::Utc::now().timestamp() - expiration_secs;
    let rows = conn.execute("DELETE FROM bv_cache WHERE updated_at < ?", params![threshold])?;
    Ok(rows)
}
