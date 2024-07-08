use anyhow::anyhow;
use anyhow::{Error, Result};
use chrono::DateTime;
use chrono::Utc;
use energyleaf_proto::prost::Message;
use serde_json::Value;

#[derive(Debug)]
pub struct SensorData {
    pub total_in: f64,
    pub total_out: Option<f64>,
    pub power_curr: Option<f64>,
}

fn extract_data(value: &Value, data: &mut SensorData) -> Result<(), Error> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key.as_str() == "Total_in" {
                    let parsed_value = value
                        .as_f64()
                        .ok_or(anyhow!("Could not parse total in value"))?;
                    data.total_in = parsed_value;
                } else if key.as_str() == "Total_out" {
                    let parsed_value = value
                        .as_f64()
                        .ok_or(anyhow!("Could not parse total out value"))?;
                    data.total_out = Some(parsed_value);
                } else if key.as_str() == "Power_curr" {
                    let parsed_value = value
                        .as_f64()
                        .ok_or(anyhow!("Could not parse power curr value"))?;
                    data.power_curr = Some(parsed_value);
                } else {
                    extract_data(value, data)?;
                }
            }
        }
        Value::Array(arr) => {
            for value in arr {
                extract_data(value, data)?;
            }
        }
        _ => {}
    }
    return Ok(());
}

pub async fn get_data_from_sensor(sensor_url: &str) -> Result<SensorData, Error> {
    let client = reqwest::Client::new();
    let res = client.get(sensor_url).send().await?;
    let mut data = SensorData {
        total_in: 0.0,
        total_out: None,
        power_curr: None,
    };
    extract_data(&res.json().await?, &mut data)?;

    return Ok(data);
}

pub async fn send_data_to_server(
    value_in: f64,
    value_out: Option<f64>,
    value_current: Option<f64>,
    timestamp: Option<DateTime<Utc>>,
    token: &str,
    url: &str,
) -> Result<(), Error> {
    let mut buf = Vec::new();
    let timestamp_value = match timestamp {
        Some(t) => {
            let value = t.timestamp_nanos_opt();
            match value {
                Some(v) => Some(v as u64),
                None => None,
            }
        }
        None => None,
    };
    _ = (energyleaf_proto::energyleaf::SensorDataRequestV2 {
        access_token: token.to_string(),
        r#type: energyleaf_proto::energyleaf::SensorType::DigitalElectricity as i32,
        value: value_in,
        value_out,
        value_current,
        timestamp: timestamp_value,
    })
    .encode(&mut buf)?;

    let client = reqwest::Client::new();

    let res = energyleaf_proto::energyleaf::SensorDataResponse::decode(
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
