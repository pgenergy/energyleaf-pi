use std::time::Duration;

use tokio::{sync::mpsc, time::sleep};

mod proto;

#[tokio::main]
async fn main() {
    let(tx, mut rx) = mpsc::channel(36);

    tokio::spawn(async move {
        loop {
            // request here
            
            sleep(Duration::from_secs(15))
        }
    });

    while let Some(data) = rx.recv() {
        // process data
    }
}
