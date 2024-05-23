use std::{env, sync::Arc, time::Duration};

use crate::api::ResponseData;
use dotenvy::dotenv;
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
        if consumption <= 0.0 {
            if let Err(e) = db::add_log(&"Consumption is zero value", &conn).await {
                println!("{}", e.to_string())
            }

            return;
        }

        if let Err(err) = db::add_sensor_value(consumption.clone(), &conn).await {
            if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                println!("{}", e.to_string())
            }
        }

        println!("{}", &consumption);
        let token = match auth::get_token(&format!("{}/token", &admin_url), &conn).await {
            Ok(t) => t,
            Err(err) => {
                if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                    println!("{}", e.to_string())
                }
                continue;
            }
        };

        if let Err(err) = api::send_data_to_server(consumption, &token).await {
            if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                println!("{}", e.to_string())
            }
        }
    }
}
