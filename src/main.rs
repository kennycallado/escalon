mod constants;
mod server;
mod server_builder;
mod types;

use std::net::IpAddr;

use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};

use crate::server::Server;

// remove this when done
use rand::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;

    // let (tx, mut _rx) = tokio::sync::mpsc::channel::<Message>(100);
    let mut udp_server = Server::new()
        .set_addr(addr)
        .set_port(port)
        .set_count(|| {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..100)
        })
        // .set_sender(tx)
        .build()
        .await?;

    udp_server.listen().await?;

    signal(SignalKind::terminate())?.recv().await;
    println!("Shutting down the server");

    Ok(())
}
