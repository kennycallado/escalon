use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use sysinfo::{System, SystemExt};

use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::constants::{BUFFER_SIZE, HEARTBEAT_SECS, MAX_CONNECTIONS, THRESHOLD_SECS};
use crate::server_builder::ServerBuilder;
use crate::server_builder::{NoAddr, NoCount, NoPort};
use crate::types::ClientState;
use crate::types::{Action, Client, Message};

pub type Callback = Arc<dyn Fn() -> usize + Send + Sync>;

pub struct Server {
    pub id: String,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
    pub count: Callback,
    pub socket: Arc<UdpSocket>,
    pub start_time: std::time::SystemTime,
    pub tx_handler: Option<Sender<(Message, SocketAddr)>>,
    pub tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
}

impl Server {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> ServerBuilder<NoAddr, NoPort, NoCount> {
        let hostname = match System::new().host_name() {
            Some(hostname) => hostname,
            None => {
                panic!("Hostname not found");
            }
        };

        ServerBuilder {
            id: hostname,
            addr: NoAddr,
            port: NoPort,
            count: NoCount,
        }
    }

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

    fn start_heartbeat(&self) -> Result<()> {
        let clients = self.clients.clone();
        let server_count = self.count.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::task::spawn(async move {
            loop {
                // sleeps
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT_SECS)).await;

                // build message
                let memory = procinfo::pid::statm(std::process::id().try_into().unwrap())
                    .unwrap()
                    .resident;
                let client_state = ClientState {
                    memory,
                    tasks: server_count(),
                };
                let message = Message {
                    action: Action::Check((server_id.clone(), client_state)),
                };

                // send message
                tx_sender.as_ref().unwrap().send((message, None)).await.unwrap();

                // update clients
                let mut clients = clients.lock().unwrap();
                let now = Utc::now().timestamp();

                // detect dead clients
                // were update on handle_action
                let dead_clients = clients
                    .iter()
                    .filter(|(_, client)| now - client.last_seen > THRESHOLD_SECS)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                for id in dead_clients {
                    let dead = clients.get(&id).unwrap().clone();
                    println!("{:?} is dead", dead);

                    clients.remove(&id);
                }
            }
        });

        Ok(())
    }

    fn handle_action(&self) -> Result<Sender<(Message, SocketAddr)>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let server_start_time = self.start_time;
        let tx_sender = self.tx_sender.clone();

        tokio::task::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action {
                    Action::Join((id, start_time)) => {
                        if id != server_id {
                            if !clients.lock().unwrap().contains_key(&id) {
                                let message = Message {
                                    action: Action::Join((
                                        server_id.clone(),
                                        server_start_time,
                                    )),
                                };

                                tx_sender
                                    .as_ref()
                                    .unwrap()
                                    .send((message, Some(addr)))
                                    .await
                                    .unwrap();
                            }

                            insert(&mut clients.lock().unwrap(), id.clone(), start_time, addr);
                        }
                    }
                    Action::Check((id, state)) => {
                        if id != server_id {
                            update(&mut clients.lock().unwrap(), id.clone(), state);
                        }
                    }
                }
            }

            #[rustfmt::skip]
            fn insert(clients: &mut HashMap<String, Client>, id: String, start_time: std::time::SystemTime, addr: SocketAddr) {
                clients
                    .entry(id)
                    .or_insert(Client {
                        start_time,
                        address: addr,
                        last_seen: Utc::now().timestamp(),
                        state: ClientState { memory: 0, tasks: 0 },
                    });
            }

            #[rustfmt::skip]
            fn update(clients: &mut HashMap<String, Client>, id: String, state: ClientState) {
                clients
                    .entry(id)
                    .and_modify(|client| {
                        client.last_seen = Utc::now().timestamp();
                        client.state = state;
                    });
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
