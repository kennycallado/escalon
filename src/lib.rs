mod builder;
mod client;
mod constants;
mod message;
mod handler;
mod heartbeat;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use builder::NoId;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{EscalonBuilder, NoAddr, NoPort};
use crate::client::{Client, ClientState};
use crate::constants::{BUFFER_SIZE, MAX_CONNECTIONS};
use crate::message::{Message, Action};

#[derive(Clone)]
pub struct Escalon<J> {
    pub id: String,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
    pub jobs: Arc<Mutex<Vec<J>>>,
    pub own_state: Arc<Mutex<ClientState>>,
    pub socket: Arc<UdpSocket>,
    pub start_time: std::time::SystemTime,
    pub tx_handler: Option<Sender<(Message, SocketAddr)>>,
    pub tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
}

impl<J> Escalon<J> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonBuilder<NoId, NoAddr, NoPort> {
        EscalonBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
        }
    }
}

impl<J: Send + Sync + 'static> Escalon<J> {
    pub async fn listen(&mut self) -> Result<()> {
        // udp sender
        self.tx_sender = Some(self.to_udp()?);
        // join
        self.send_join()?;
        // heartbeat
        self.start_heartbeat()?;
        // handler
        self.tx_handler = Some(self.handle_action()?);
        // udp reciver
        self.from_udp()?;

        println!("Server listen on: {}", self.socket.local_addr()?);

        Ok(())
    }

    fn send_join(&self) -> Result<()> {
        let tx = self.tx_sender.clone();
        let server_id = self.id.clone();
        let server_start_time = self.start_time;

        tokio::task::spawn(async move {
            let message = Message {
                action: Action::Join((server_id, server_start_time)),
            };

            tx.as_ref().unwrap().send((message, None)).await.unwrap();
        });

        Ok(())
    }

    fn to_udp(&self) -> Result<Sender<(Message, Option<SocketAddr>)>> {
        let socket = self.socket.clone();
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);

        tokio::task::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                let bytes = match serde_json::to_vec(&msg) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        println!("Error: {e}");

                        continue;
                    }
                };

                let addr = match addr {
                    Some(addr) => addr,
                    None => SocketAddr::from(([255, 255, 255, 255], 65056)),
                };

                socket.send_to(&bytes, addr).await.unwrap();
            }
        });

        Ok(tx)
    }

    #[allow(clippy::wrong_self_convention)]
    fn from_udp(&self) -> Result<()> {
        let socket = self.socket.clone();
        let tx = self.tx_handler.clone();

        tokio::task::spawn(async move {
            let mut buf = [0u8; BUFFER_SIZE];

            loop {
                let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
                let message: Message = match serde_json::from_slice(&buf[..len]) {
                    Ok(message) => message,
                    Err(e) => {
                        println!("Error: {e}");

                        continue;
                    }
                };

                tx.clone().unwrap().send((message, addr)).await.unwrap();
            }
        });

        Ok(())
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation_and_listen() -> Result<()> {
        let blah = vec![1, 2, 3];
        let blah = Arc::new(Mutex::new(blah));

        let mut server = Escalon::<i32>::new()
            .set_id("test")
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0) // Use a random available port
            .build(blah)
            .await?;

        assert!(server.listen().await.is_ok());

        drop(server);

        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_bind_twice() {
        let blah = vec![1, 2, 3];
        let blah = Arc::new(Mutex::new(blah));

        let mut server = Escalon::<i32>::new()
            .set_id("test")
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0) // Use a random available port
            .build(blah)
            .await
            .unwrap();

        assert!(server.listen().await.is_ok());
        tokio::net::UdpSocket::bind(server.socket.local_addr().unwrap()).await.unwrap();

        drop(server);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_server_invalid_port() {
        let blah = vec![1, 2, 3];
        let blah = Arc::new(Mutex::new(blah));

        let mut server = Escalon::<i32>::new()
            .set_id("test")
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(1)
            .build(blah)
            .await
            .unwrap();

        assert!(server.listen().await.is_ok());

        drop(server);
    }

    #[tokio::test]
    async fn test_intercept_before_send_join() -> Result<()> {
        let blah = vec![1, 2, 3];
        let blah = Arc::new(Mutex::new(blah));

        let mut server = Escalon::<i32>::new()
            .set_id("test")
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0) // Use a random available port
            .build(blah)
            .await?;

        let (tx_sender, mut rx_sender) =
            tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
        server.tx_sender = Some(tx_sender);

        assert!(server.send_join().is_ok());

        let received_message: (Message, Option<SocketAddr>) = rx_sender.recv().await.unwrap();

        assert_eq!(received_message.1, None);
        assert_eq!(
            received_message.0.action,
            Action::Join((server.id.clone(), server.start_time))
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_intercept_before_hertbeat() -> Result<()> {
        let blah = vec![1, 2, 3];
        let blah = Arc::new(Mutex::new(blah));

        let mut server = Escalon::<i32>::new()
            .set_id("test")
            .set_addr("127.0.0.1".parse().unwrap())
            .set_port(0)
            .build(blah)
            .await?;

        let (tx_sender, mut rx_sender) =
            tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);
        server.tx_sender = Some(tx_sender);

        assert!(server.start_heartbeat().is_ok());

        let received_message: (Message, Option<SocketAddr>) = rx_sender.recv().await.unwrap();

        assert_eq!(received_message.1, None);
        assert!(matches!(received_message.0.action, Action::Check(..)));

        Ok(())
    }

    // it doesn't work and I don't know why
    //
    // #[tokio::test]
    // async fn test_never_ends() -> Result<()> {
    //     let mut server = Server::new()
    //         .set_addr("0.0.0.0".parse().unwrap())
    //         .set_port(0)  // Use a random available port
    //         .set_count(|| 0) // Mock the count callback
    //         .build()
    //         .await?;

    //     let (tx_handler, mut rx_handler) = tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

    //     server.tx_sender = Some(server.to_udp()?);
    //     server.send_join()?;
    //     server.start_heartbeat()?;
    //     // server.tx_handler = Some(server.handle_action()?);
    //     server.tx_handler = Some(tx_handler);
    //     server.from_udp()?;

    //     let received_message: (Message, SocketAddr) = rx_handler.recv().await.unwrap();

    //     Ok(())
    // }
}
