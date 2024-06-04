use std::str::FromStr;

use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use libsql::{params, Builder, Connection};

pub async fn get_conn() -> Result<Connection, Error> {
    let db = Builder::new_local("./energyleaf.db").build().await?;
    let conn = db.connect()?;
    _ = create_tables(&conn).await?;

    return Ok(conn);
}

async fn create_tables(conn: &Connection) -> Result<(), Error> {
    // create token table
    conn.execute(
        r#"
            CREATE TABLE IF NOT EXISTS token (
                token TEXT NOT NULL PRIMARY KEY,
                expires_at DATETIME NOT NULL
            )
        "#,
        (),
    )
    .await?;

    // create sensor table
    conn.execute(
        r#"
            CREATE TABLE IF NOT EXISTS data (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                value NUMERIC(30, 6) NOT NULL,
                value_out NUMERIC(30,6),
                value_current NUMERIC(30,6),
                synced BOOLEAN NOT NULL DEFAULT FALSE
            )
        "#,
        (),
    )
    .await?;

    // create log table
    conn.execute(
        r#"
            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                log TEXT NOT NULL
            )
        "#,
        (),
    )
    .await?;

    return Ok(());
}

pub async fn add_sensor_value(
    value_in: f64,
    value_out: Option<f64>,
    value_current: Option<f64>,
    synced: bool,
    time: DateTime<Utc>,
    conn: &Connection,
) -> Result<(), Error> {
    let synced_value = if synced { 1.0 } else { 0.0 };
    conn.execute(
        r#"
            INSERT INTO data (value, value_out, value_current, synced, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![value_in, value_out, value_current, synced_value, time.to_rfc3339()],
    )
    .await?;

    // delete oldest value if data has more than 10 million entries
    conn.execute(
        r#"
            DELETE FROM data
            WHERE timestamp = (SELECT MIN(timestamp) FROM data)
            AND (SELECT COUNT(*) FROM data) > 10000000;
        "#,
        (),
    )
    .await?;

    return Ok(());
}

pub async fn add_log(value: &str, conn: &Connection) -> Result<(), Error> {
    let mut stmn = conn
        .prepare(
            r#"
                INSERT INTO logs (log) VALUES (?1)
            "#,
        )
        .await?;
    stmn.execute([value]).await?;

    return Ok(());
}

pub async fn update_token(
    value: &str,
    expires_at: DateTime<Utc>,
    conn: &Connection,
) -> Result<(), Error> {
    let tx = conn.transaction().await?;
    tx.execute("DELETE FROM token", ()).await?;
    let mut stmn = tx
        .prepare(
            r#"
                INSERT INTO token (token, expires_at) VALUES (?1, ?2)
            "#,
        )
        .await?;
    stmn.execute([value, &expires_at.to_rfc3339()]).await?;
    tx.commit().await?;

    return Ok(());
}

pub async fn get_local_token(conn: &Connection) -> Result<Option<String>, Error> {
    let mut rows = conn.query("SELECT * FROM token LIMIT 1", ()).await?;
    if rows.column_count() <= 0 {
        return Ok(None);
    }
    match rows.next().await? {
        Some(row) => {
            let token = row.get::<String>(0)?;
            let expires_at = DateTime::<Utc>::from_str(&row.get::<String>(1)?)?;

            if expires_at < Utc::now() {
                conn.execute("DELETE FROM token", ()).await?;
                return Ok(None);
            }

            return Ok(Some(token));
        }
        None => {
            return Ok(None);
        }
    }
}

#[derive(Debug)]
pub struct DBData {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub value_out: Option<f64>,
    pub value_current: Option<f64>,
    pub synced: bool,
}

pub async fn get_unsync_entries(conn: &Connection) -> Result<Vec<DBData>, Error> {
    let mut db_data: Vec<DBData> = vec![];
    let mut rows = conn
        .query(
            r#"
            SELECT * FROM DATA
            WHERE synced = 0
            ORDER BY timestamp DESC
        "#,
            (),
        )
        .await?;

    while let Ok(Some(row)) = rows.next().await {
        let data = DBData {
            id: row.get::<i32>(0)?,
            timestamp: DateTime::<Utc>::from_str(&row.get::<String>(1)?)?,
            value: row.get::<f64>(2)?,
            value_out: row.get::<Option<f64>>(3)?,
            value_current: row.get::<Option<f64>>(4)?,
            synced: row.get(5)?,
        };
        db_data.push(data);
    }

    return Ok(db_data);
}

pub async fn mark_data_as_synced(id: i32, conn: &Connection) -> Result<(), Error> {
    conn.execute(
        r#"
            UPDATE DATA
            SET synced = 1
            WHERE id = ?1
        "#,
        params![id],
    )
    .await?;

    return Ok(());
}
