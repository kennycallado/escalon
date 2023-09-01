use std::net::IpAddr;

use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};

use escalon::Escalon;

// remove this when done
use rand::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;

    // let (tx, mut _rx) = tokio::sync::mpsc::channel::<Message>(100);
    let mut udp_server = Escalon::new()
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

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation_and_listen() -> Result<()> {
        let mut server = Escalon::new()
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0) // Use a random available port
            .set_count(|| 0) // Mock the count callback
            .build()
            .await?;

        assert!(server.listen().await.is_ok());
        drop(server);

        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_bind_twice() {
        let mut server = Escalon::new()
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0) // Use a random available port
            .set_count(|| 0) // Mock the count callback
            .build()
            .await
            .unwrap();

        assert!(server.listen().await.is_ok());

        tokio::net::UdpSocket::bind(server.socket.local_addr().unwrap()).await.unwrap();

        drop(server);
    }
}
