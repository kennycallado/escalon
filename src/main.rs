mod constants;
mod server;
mod server_builder;
mod types;

use std::net::IpAddr;

use anyhow::Result;
use sysinfo::{System, SystemExt};
use tokio::signal::unix::{signal, SignalKind};

use server::Server;
use server_builder::ServerBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    let mut sys = System::new();
    sys.refresh_all();

    let hostname = match sys.host_name() {
        Some(hostname) => hostname,
        None => {
            println!("Hostname not found");
            std::process::exit(1);
        }
    };

    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;

    // let (tx, mut _rx) = tokio::sync::mpsc::channel::<Message>(100);
    let mut udp_server: Server = ServerBuilder::new(hostname)
        .set_addr(addr)
        .set_port(port)
        // .set_sender(tx)
        .build()
        .await?;

    println!("Server started at {}:{}", addr, port);
    udp_server.listen().await?;

    signal(SignalKind::terminate())?.recv().await;
    println!(" ->> Shutting down the server");

    Ok(())
}
