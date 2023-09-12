use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};

use escalon::Escalon;

// remove this when done
use rand::prelude::*;
use uuid::Uuid;

struct MyStruct {
    job_id: Uuid,
    task: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("ADDR").unwrap_or("0.0.0.0".to_string()).parse::<IpAddr>()?;
    let port = std::env::var("PORT").unwrap_or("65056".to_string()).parse::<u16>()?;
    let iden = std::env::var("HOSTNAME").unwrap_or("server".to_string());

    // Generate a vector with random number of numbers
    let mut rng = rand::thread_rng();
    let mut blah = vec![];

    for _ in 0..rng.gen_range(1..10) {
        let job = MyStruct {
            job_id: Uuid::new_v4(),
            task: "test".to_string(),
        };

        blah.push(job);
    }

    let blah = Arc::new(Mutex::new(blah));

    let mut udp_server = Escalon::<MyStruct>::new()
        .set_id(iden)
        .set_addr(addr)
        .set_port(port)
        .build(blah)
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
            .set_id("test")
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
            .set_id("test")
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
