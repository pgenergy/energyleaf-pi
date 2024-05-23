use anyhow::{Error, Result};
use anyhow::anyhow;
use chrono::{Duration, Utc};
use libsql::Connection;
use prost::Message;

use crate::{db, proto};

async fn refresh_token(url: &str) -> Result<String, Error> {
    let mut buf = Vec::new();
    _ = (proto::energyleaf_proto::TokenRequest {
        client_id: "".to_string(),
        r#type: 1,
        need_script: Some(false),
    })
    .encode(&mut buf);

    let client = reqwest::Client::new();
    let res = proto::energyleaf_proto::TokenResponse::decode(
        client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-protobuf")
            .body(buf)
            .send()
            .await?
            .bytes()
            .await?,
    )?;

    return match res.status {
        200..=299 => Ok(res.access_token().to_string()),
        _ => match res.status_message {
            Some(msg) => Err(anyhow!("{}", msg)),
            None => Err(anyhow!("Error sending data")),
        },
    }
}

pub async fn get_token(url: &str, conn: &Connection) -> Result<String, Error> {
    return match db::get_local_token(conn).await? {
        Some(token) => Ok(token),
        None => {
            let token = refresh_token(url).await?;
            let expires_at = Utc::now() + Duration::minutes(50);
            db::update_token(&token, expires_at, conn).await?;

            Ok(token)
        },
    };
}
