use std::time::Duration;

use tokio::{sync::mpsc, time::sleep};

mod proto;

struct ResponseData {}

#[tokio::main]
async fn main() {
    let(tx, mut rx) = mpsc::channel::<ResponseData>(36);

    tokio::spawn(async move {
        loop {
            // request here
            
            sleep(Duration::from_secs(15)).await
        }
    });

    while let Some(data) = rx.recv().await {
    }
}
