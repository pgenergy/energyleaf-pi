use anyhow::{Error, Result};
use api::SensorData;
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use energyleaf_coordpi::command::{
    Command, CommandToPiType, DevicePayload, DeviceType, ElectricityDigitalPayload,
};
use libsql::Connection;
use log::debug;
use std::{env, io, sync::Arc, time::Duration};
use tokio::io::AsyncReadExt;
use tokio::{sync::mpsc, time::sleep};
use tokio_serial::SerialPortBuilderExt;

mod api;
mod auth;
mod db;

//following const maybe set directly in an interface on the pi ?
const KWH_PER_ANALOG_ELECTRICITY_ROTATION: f64 = 0.0f64;
const KWH_PER_GAS_ROTATION: f64 = 0.0f64;

#[tokio::main]
async fn main() {
    dotenv().expect("Cant find env file");
    env_logger::init();
    let sensor_url = env::var("SENSOR_URL").expect("SENSOR_URL must be set");
    let admin_url = Arc::new(env::var("ADMIN_URL").expect("ADMIN_URL must be set"));
    let admin_url_clone = admin_url.clone();
    let coordinator_enable =
        env::var("COORDINATOR_ENABLE").expect("COORDINATOR_ENABLE must be set");

    let conn = Arc::new(db::get_conn().await.expect("Could not connect to db"));
    let conn_req = Arc::clone(&conn);
    let conn_sync = Arc::clone(&conn);
    let conn_coord = Arc::clone(&conn);
    let (tx, mut rx) = mpsc::channel::<SensorData>(32);
    let tx_coord = tx.clone();

    if &coordinator_enable == "true" {
        //dev/ttyAMA1 is used from a pi (changing code to allow the user to define it by the .env file, is easy)
        let mut port = tokio_serial::new("/dev/ttyAMA1", 9600)
            .open_native_async()
            .unwrap();

        tokio::spawn(async move {
            let conn = conn_coord;
            let tx = tx_coord;
            let mut incoming_command_message = vec![];
            let mut incoming_command_message_buffer = vec![0u8; 16];
            //EnergyDataRequest
            let mut last_digitial_electricity_reading = api::SensorData {
                total_in: 0.0,
                total_out: None,
                power_curr: None,
            };
            loop {
                match port.read(&mut incoming_command_message_buffer).await {
                    Ok(t) => {
                        if t > 0 {
                            debug!(
                                "Incoming message (UART): {:?}",
                                &incoming_command_message_buffer[..t]
                            );

                            if incoming_command_message_buffer[..t]
                                .eq(&energyleaf_coordpi::command::COMMAND_MESSAGE_END)
                            {
                                let remaining = incoming_command_message[0] as usize;
                                let end_index = incoming_command_message.len() - remaining;
                                let opt_command = energyleaf_coordpi::command::Command::from_cbor(
                                    &incoming_command_message[1..end_index],
                                );

                                if let Some(cmd) = opt_command {
                                    match cmd {
                                        Command::ToCoordinator { .. } => {}
                                        Command::ToPi { cmd_to_pi, .. } => {
                                            //src is the zigbee u16 identifier from the sensor, can be maybe useful

                                            match cmd_to_pi {
                                                CommandToPiType::NewDeviceConnected { .. } => {
                                                    //payload contains the manufacturer and model of the new connected or reconnected device
                                                }
                                                CommandToPiType::NewValue { device_type } => {
                                                    match device_type {
                                                        DeviceType::GasAnalog => {
                                                            //one rotation of gas meter
                                                            let value = KWH_PER_GAS_ROTATION;
                                                            //ToDo: add a way for gas, for sending and saving
                                                        }
                                                        DeviceType::ElectricityAnalog => {
                                                            //one rotation of analog electricity meter
                                                            _ = tx.send(api::SensorData{
                                                                total_in: KWH_PER_ANALOG_ELECTRICITY_ROTATION,
                                                                total_out: None,
                                                                power_curr: None,
                                                            }).await;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                CommandToPiType::NewValueWithPayload {
                                                    device_type,
                                                    payload,
                                                } => {
                                                    match device_type {
                                                        DeviceType::ElectricityDigital => {
                                                            //data from electricity meter
                                                            let DevicePayload::ElectricityDigital(
                                                                cmd_payload,
                                                            ) = payload;

                                                            match cmd_payload {
                                                                    ElectricityDigitalPayload::ReadingIn(value) => {
                                                                        if last_digitial_electricity_reading.total_in > 0.0f64 {
                                                                            //currently it is sending the value if the next value is read, maybe a better way possible
                                                                            _ = tx.send(last_digitial_electricity_reading).await;
                                                                            last_digitial_electricity_reading = api::SensorData {
                                                                                total_in: 0.0,
                                                                                total_out: None,
                                                                                power_curr: None,
                                                                            };
                                                                        }
                                                                        last_digitial_electricity_reading.total_in = value;
                                                                    }
                                                                    ElectricityDigitalPayload::ReadingOut(value) => {
                                                                        last_digitial_electricity_reading.total_out = Option::from(value);
                                                                    }
                                                                    ElectricityDigitalPayload::PowerCurrent(value) => {
                                                                        last_digitial_electricity_reading.power_curr = Option::from(value);
                                                                    }
                                                                }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                //do stuff

                                incoming_command_message.clear();
                            } else {
                                incoming_command_message
                                    .append(&mut incoming_command_message_buffer);
                                incoming_command_message_buffer.resize(16, 0u8);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        ();
                    }
                    Err(err) => {
                        eprintln!("{:?}", err);
                        if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                            eprintln!("{}", e.to_string())
                        }
                    }
                }
            }
        });
    }

    if &sensor_url != "localhost" {
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
    }

    if admin_url_clone.as_str() != "localhost/api/v1" && admin_url_clone.as_str() != "localhost" {
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
    }

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

    return match api::send_data_to_server(
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

            true
        }
        Err(err) => {
            println!("{:?}", err);
            if let Err(e) = db::add_log(&err.to_string(), &conn).await {
                println!("{}", e.to_string())
            }
            save_sensor_value(consumption, outgoing, current, false, time, &conn)
                .await
                .unwrap_or(());

            false
        }
    };
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
