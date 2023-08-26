use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::constants::{BUFFER_SIZE, HEARTBEAT_SECS, MAX_CONNECTIONS, THRESHOLD_SECS};
use crate::types::{Action, Client, Message};

pub struct Server {
    pub id: String,
    pub socket: Arc<UdpSocket>,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
    pub tx_handler: Option<Sender<(Message, SocketAddr)>>,
    pub tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
    // tx_up: Sender<Message>,
}

impl Server {
    pub async fn listen(&mut self) -> Result<()> {
        // self.socket = Arc::new(socket);

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

        Ok(())
    }

    fn send_join(&self) -> Result<()> {
        let tx = self.tx_sender.clone();
        let id = self.id.clone();

        tokio::task::spawn(async move {
            let message = Message {
                action: Action::Join(id),
            };

            tx.as_ref().unwrap().send((message, None)).await.unwrap();
        });

        Ok(())
    }

    fn start_heartbeat(&self) -> Result<()> {
        let id = self.id.clone();
        let clients = self.clients.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::spawn(async move {
            let message = Message {
                action: Action::Check(id),
            };

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;
                tx_sender.as_ref().unwrap().send((message.clone(), None)).await.unwrap();

                let mut clients = clients.lock().unwrap();
                let now = Utc::now().timestamp();

                let dead_clients = clients
                    .iter()
                    .filter(|(_, client)| now - client.last_seen > THRESHOLD_SECS)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                for id in dead_clients {
                    println!("Client dead: {}", id);
                    clients.remove(&id);
                }
            }
        });

        Ok(())
    }

    fn handle_action(&self) -> Result<Sender<(Message, SocketAddr)>> {
        // self.start_heartbeat()?;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action {
                    Action::Join(id) => {
                        if id != server_id {
                            if !clients.lock().unwrap().contains_key(&id) {
                                let message = Message {
                                    action: Action::Join(server_id.clone()),
                                };

                                tx_sender
                                    .as_ref()
                                    .unwrap()
                                    .send((message, Some(addr)))
                                    .await
                                    .unwrap();
                            }

                            update_timestamp_or_insert(
                                &mut clients.lock().unwrap(),
                                id.clone(),
                                addr,
                            );
                        }
                    }
                    Action::Check(id) => {
                        if id != server_id {
                            update_timestamp_or_insert(
                                &mut clients.lock().unwrap(),
                                id.clone(),
                                addr,
                            );

                            // let mut clients = clients.lock().unwrap();
                            // clients.entry(id).and_modify(|client| { client.last_seen = Utc::now().timestamp(); });
                        }
                    }
                }

                #[rustfmt::skip]
                fn update_timestamp_or_insert(clients: &mut HashMap<String, Client>, id: String, addr: SocketAddr) {
                    clients
                        .entry(id)
                        .and_modify(|client| client.last_seen = Utc::now().timestamp())
                        .or_insert(Client {
                            address: addr,
                            last_seen: Utc::now().timestamp(),
                        });
                }
            }
        });

        Ok(tx)
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
