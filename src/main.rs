use anyhow::{Error, Result};
use std::{env, sync::Arc, time::Duration};

use crate::api::ResponseData;
use dotenvy::dotenv;
use libsql::Connection;
use tokio::{sync::mpsc, time::sleep};

mod api;
mod auth;
mod db;
mod proto;

#[tokio::main]
async fn main() {
    dotenv().expect("Cant find env file");
    let sensor_url = env::var("SENSOR_URL").expect("SENSOR_URL must be set");
    let admin_url = env::var("ADMIN_URL").expect("ADMIN_URL must be set");

    let conn = Arc::new(db::get_conn().await.expect("Could not connect to db"));
    let conn_clone = Arc::clone(&conn);
    let (tx, mut rx) = mpsc::channel::<ResponseData>(32);

    tokio::spawn(async move {
        let conn = conn_clone;
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

    while let Some(data) = rx.recv().await {
        let consumption = data.data.sensor.total_in;
        let outgoing = data.data.sensor.total_out;
        let current = data.data.sensor.power_curr;

        if consumption <= 0.0 {
            if let Err(e) = db::add_log(&"Consumption is zero value", &conn).await {
                println!("{}", e.to_string())
            }

            return;
        }

        let token = match auth::get_token(&format!("{}/token", &admin_url), &conn).await {
            Ok(t) => t,
            Err(err) => {
                println!("{:?}", err);
                if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                    println!("{}", e.to_string())
                }
                save_sensor_value(consumption, outgoing, current, false, &conn).await.unwrap_or(());
                continue;
            }
        };

        match api::send_data_to_server(
            consumption,
            outgoing,
            current,
            &token,
            &format!("{}/sensor_input", &admin_url),
        )
        .await
        {
            Ok(_) => {
                save_sensor_value(consumption, outgoing, current, true, &conn).await.unwrap_or(());
            }
            Err(err) => {
                println!("{:?}", err);
                if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                    println!("{}", e.to_string())
                }
                save_sensor_value(consumption, outgoing, current, false, &conn).await.unwrap_or(());
            }
        }
    }
}

async fn save_sensor_value(
    value_in: f64,
    value_out: f64,
    value_current: f64,
    synced: bool,
    conn: &Connection,
) -> Result<(), Error> {
    if let Err(err) = db::add_sensor_value(value_in, value_out, value_current, synced, &conn).await
    {
        println!("{:?}", err);
        if let Err(e) = db::add_log(&err.to_string(), &conn).await {
            println!("{}", e.to_string())
        }
    }

    return Ok(());
}
