use std::str::FromStr;

use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use libsql::{Builder, Connection};

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
                value NUMERIC(12, 4) NOT NULL
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

pub async fn add_sensor_value(value: f32, conn: &Connection) -> Result<(), Error> {
    let mut stmn = conn
        .prepare(
            r#"
            INSERT INTO data (value) VALUES (?1)
        "#,
        )
        .await?;
    stmn.execute([value]).await?;

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
