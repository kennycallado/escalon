use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use escalon::Escalon;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::signal::unix::{signal, SignalKind};
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct MyStruct {
    job_id: Uuid,
    task: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;
    let iden = std::env::var("HOSTNAME").unwrap_or("server".to_string());

    let jobs: Arc<Mutex<Vec<MyStruct>>> = Arc::new(Mutex::new(Vec::new()));
    let cloned_jobs = jobs.clone();

    let mut udp_server = Escalon::new()
        .set_id(iden)
        .set_addr(addr)
        .set_port(port)
        .set_count(move || { cloned_jobs.lock().unwrap().len() })
        .build()
        .await?;

    tokio::spawn(async move {
        loop {
            for _ in 0..rand::thread_rng().gen_range(1..10) {
                let job = MyStruct {
                    job_id: Uuid::new_v4(),
                    task: "test".to_string(),
                };

                let mut blah = jobs.lock().unwrap();
                blah.push(job);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    udp_server.listen().await?;

    signal(SignalKind::terminate())?.recv().await;
    println!("Shutting down the server");

    Ok(())
}
