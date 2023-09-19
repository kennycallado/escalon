use anyhow::Result;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

use crate::constants::{BUFFER_SIZE, MAX_CONNECTIONS};
use crate::types::message::{Action, JoinContent, Message};
use crate::Escalon;

impl Escalon {
    pub async fn listen(&mut self) -> Result<()> {
        // udp sender
        self.tx_sender = Some(self.to_udp()?);
        // join
        self.send_join()?;
        // heartbeat
        self.start_heartbeat()?;
        self.scanner_dead()?;
        // handler
        self.tx_handler = Some(self.handle_action()?);
        // udp reciver
        self.from_udp()?;

        println!("Server listen on: {}", self.socket.local_addr()?);

        Ok(())
    }

    pub fn send_join(&self) -> Result<()> {
        let tx = self.tx_sender.clone();

        let message = Message {
            action: Action::Join(JoinContent {
                sender_id: self.id.clone(),
                start_time: self.start_time,
            }),
        };

        tokio::task::spawn(async move {
            tx.as_ref().unwrap().send((message, None)).await.unwrap();
        });

        Ok(())
    }

    pub fn to_udp(&self) -> Result<Sender<(Message, Option<SocketAddr>)>> {
        let socket = self.socket.clone();
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);

        tokio::spawn(async move {
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
    pub fn from_udp(&self) -> Result<()> {
        let socket = self.socket.clone();
        let tx = self.tx_handler.clone();

        tokio::spawn(async move {
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

                tx.as_ref().unwrap().send((message, addr)).await.unwrap();
            }
        });

        Ok(())
    }
}
