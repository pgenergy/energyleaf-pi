use anyhow::{Error, Result};
use api::SensorData;
use chrono::{DateTime, Utc};
use std::{env, sync::Arc, time::Duration};

use dotenvy::dotenv;
use libsql::Connection;
use tokio::{sync::mpsc, time::sleep};

mod api;
mod auth;
mod db;

#[tokio::main]
async fn main() {
    dotenv().expect("Cant find env file");
    let sensor_url = env::var("SENSOR_URL").expect("SENSOR_URL must be set");
    let admin_url = Arc::new(env::var("ADMIN_URL").expect("ADMIN_URL must be set"));
    let admin_url_clone = admin_url.clone();

    let conn = Arc::new(db::get_conn().await.expect("Could not connect to db"));
    let conn_req = Arc::clone(&conn);
    let conn_sync = Arc::clone(&conn);
    let (tx, mut rx) = mpsc::channel::<SensorData>(32);

    tokio::spawn(async move {
        let conn = conn_req;
        loop {
            match api::get_data_from_sensor(&sensor_url).await {
                Ok(d) => {
                    _ = tx.send(d).await;
                }
                Err(err) => {
                    println!("{:?}", err);
                    if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                        println!("{}", e.to_string())
                    }
                }
            };

            sleep(Duration::from_secs(15)).await
        }
    });

    tokio::spawn(async move {
        let conn = conn_sync;
        let admin_url = admin_url_clone;
        loop {
            match db::get_unsync_entries(&conn).await {
                Ok(d) => {
                    for entry in d {
                        let synced = process_data(
                            entry.value,
                            entry.value_out,
                            entry.value_out,
                            entry.timestamp,
                            true,
                            &admin_url,
                            &conn,
                        )
                        .await;

                        if synced {
                            if let Err(_) = db::mark_data_as_synced(entry.id, &conn).await {
                                db::increase_sync_retry_count(entry.id, &conn)
                                    .await
                                    .unwrap_or(());
                            };
                        }
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                    if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                        println!("{}", e.to_string())
                    }
                }
            }
            sleep(Duration::from_secs(60 * 60)).await
        }
    });

    while let Some(data) = rx.recv().await {
        let consumption = data.total_in;
        let outgoing = data.total_out;
        let current = data.power_curr;
        let time = chrono::Utc::now();

        process_data(
            consumption,
            outgoing,
            current,
            time,
            false,
            &admin_url,
            &conn,
        )
        .await;
    }
}

async fn process_data(
    consumption: f64,
    outgoing: Option<f64>,
    current: Option<f64>,
    time: DateTime<Utc>,
    synced_value: bool,
    admin_url: &str,
    conn: &Connection,
) -> bool {
    if consumption <= 0.0 {
        if let Err(e) = db::add_log(&"Consumption is zero value", &conn).await {
            println!("{}", e.to_string())
        }

        return false;
    }

    let token = match auth::get_token(&format!("{}/token", &admin_url), &conn).await {
        Ok(t) => t,
        Err(err) => {
            println!("{:?}", err);
            if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                println!("{}", e.to_string())
            }
            save_sensor_value(consumption, outgoing, current, false, time, &conn)
                .await
                .unwrap_or(());
            return false;
        }
    };

    match api::send_data_to_server(
        consumption,
        outgoing,
        current,
        if synced_value { Some(time) } else { None },
        &token,
        &format!("{}/sensor_input", &admin_url),
    )
    .await
    {
        Ok(_) => {
            save_sensor_value(consumption, outgoing, current, true, time, &conn)
                .await
                .unwrap_or(());

            return true;
        }
        Err(err) => {
            println!("{:?}", err);
            if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                println!("{}", e.to_string())
            }
            save_sensor_value(consumption, outgoing, current, false, time, &conn)
                .await
                .unwrap_or(());

            return false;
        }
    }
}

async fn save_sensor_value(
    value_in: f64,
    value_out: Option<f64>,
    value_current: Option<f64>,
    synced: bool,
    time: DateTime<Utc>,
    conn: &Connection,
) -> Result<(), Error> {
    if let Err(err) =
        db::add_sensor_value(value_in, value_out, value_current, synced, time, &conn).await
    {
        println!("{:?}", err);
        if let Err(e) = db::add_log(&err.to_string(), &conn).await {
            println!("{}", e.to_string())
        }
    }

    return Ok(());
}
