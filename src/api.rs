use anyhow::anyhow;
use anyhow::{Error, Result};
use chrono::DateTime;
use chrono::Utc;
use prost::Message;
use serde::Deserialize;

use crate::proto;

#[derive(Debug, Deserialize)]
pub struct ResponseData {
    #[serde(rename = "StatusSNS")]
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Data {
    #[serde(rename = "Time")]
    pub time: DateTime<Utc>,
    #[serde(rename = "Haus")]
    pub sensor: SensorData,
}

#[derive(Debug, Deserialize)]
pub struct SensorData {
    #[serde(rename = "Total_in")]
    pub total_in: f64,
    #[serde(rename = "Total_out")]
    pub total_out: f64,
    #[serde(rename = "Power_curr")]
    pub power_curr: f64,
    #[serde(rename = "Meter_Number")]
    pub meter_number: String,
}

pub async fn get_data_from_sensor(sensor_url: &str) -> Result<ResponseData, Error> {
    let client = reqwest::Client::new();
    let res = client.get(sensor_url).send().await?;
    let data = serde_json::from_value::<ResponseData>(res.json().await?)?;

    return Ok(data);
}

pub async fn send_data_to_server(
    value_in: f64,
    value_out: f64,
    value_current: f64,
    token: &str,
    url: &str,
) -> Result<(), Error> {
    let mut buf = Vec::new();
    _ = (proto::energyleaf_proto::SensorDataRequest {
        access_token: token.to_string(),
        r#type: 1,
        value: value_in,
        value_out: Some(value_out),
        value_current: Some(value_current),
    })
    .encode(&mut buf)?;

    let client = reqwest::Client::new();

    let res = proto::energyleaf_proto::SensorDataResponse::decode(
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
        200..=299 => Ok(()),
        _ => match res.status_message {
            Some(msg) => Err(anyhow!("{}", msg)),
            None => Err(anyhow!("Error sending data")),
        },
    };
}
