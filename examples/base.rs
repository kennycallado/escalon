#![allow(dead_code)]

use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use async_trait::async_trait;
use escalon::{Escalon, EscalonTrait};
use rand::prelude::*;
use tokio::signal::unix::{signal, SignalKind};
use uuid::Uuid;

struct MyStruct {
    job_id: Uuid,
    task: String,
}

#[derive(Clone)]
struct Manager {
    jobs: Arc<Mutex<Vec<MyStruct>>>,
}

#[async_trait]
impl EscalonTrait for Manager {
    fn count(&self) -> usize {
        let count = self.jobs.lock().unwrap().len();
        count
    }

    async fn take_jobs(
        &self,
        from: String,
        start_at: usize,
        count: usize,
    ) -> Result<Vec<String>, ()> {
        println!("{}: {} {}", from, start_at, count);
        Ok(Vec::new())
    }

    async fn drop_jobs(&self, jobs: Vec<String>) -> Result<(), ()> {
        println!("Dropping: {}", jobs.len());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;
    let iden = std::env::var("HOSTNAME").unwrap_or("server".to_string());
    let gran = std::env::var("GENRANGE").unwrap_or("10".to_string()).parse::<u16>()?;

    let manager = Manager {
        jobs: Arc::new(Mutex::new(Vec::new())),
    };
    let mut udp_server = Escalon::new()
        .set_id(iden)
        .set_addr(addr)
        .set_port(port)
        .set_manager(manager.clone())
        .build()
        .await;

    tokio::spawn(async move {
        // for _ in 0..gran {
        // for _ in 0..100 {
        //     let job = MyStruct {
        //         job_id: Uuid::new_v4(),
        //         task: "test".to_string(),
        //     };

        //     manager.jobs.lock().unwrap().push(job);
        // }

        loop {
            for _ in 0..rand::thread_rng().gen_range(1..gran) {
                let job = MyStruct {
                    job_id: Uuid::new_v4(),
                    task: "test".to_string(),
                };

                manager.jobs.lock().unwrap().push(job);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    udp_server.listen().await;

    signal(SignalKind::terminate())?.recv().await;
    println!("Shutting down the server");

    Ok(())
}
