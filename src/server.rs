use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;

use chrono::Utc;

use tokio::net::UdpSocket;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::Sender;

use crate::types::{Client, Message, Action};
use crate::constants::{THRESHOLD, HEARTBEAT, BUFFER_SIZE, MAX_CONNECTIONS};

pub struct Server {
    id: String,
    socket: Arc<UdpSocket>,
    clients: Arc<Mutex<HashMap<String, Client>>>,
    tx_handler: Option<Sender<(Vec<u8>, SocketAddr)>>,
    tx_sender: Option<Sender<(Vec<u8>, Option<SocketAddr>)>>,
}

impl Server {
    pub async fn new(addr: &str, id: String) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        socket.set_broadcast(true).unwrap();

        let server = Self {
            id,
            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            tx_handler: None,
            tx_sender: None,
        };

        Ok(server)
    }

    pub async fn listen(&mut self, cb: impl FnOnce(String, String, String)) -> Result<()> {

        self.tx_sender = Some(self.to_udp());
        self.tx_handler = Some(self.handle_action());
        self.send_join().await?;
        self.from_udp();

        let id = self.id.clone();
        let addr = self.socket.local_addr().unwrap().ip().to_string();
        let port = self.socket.local_addr().unwrap().port().to_string();

        cb(id, addr, port);

        // let sigterm = signal(SignalKind::terminate())?;
        // tokio::select! {
        //     _ = sigterm.recv() => { },
        //     _ = tokio::signal::ctrl_c() => { },
        // }

        signal(SignalKind::terminate())?.recv().await;
        println!(" ->> Shutting down the server");

        Ok(())
    }

    async fn send_join(&self) -> Result<()> {
        let tx = self.tx_sender.as_ref().unwrap();
        let message = Message { action: Action::Join(self.id.clone()) };

        tx.send((serde_json::to_vec(&message).unwrap(), None)).await?;

        Ok(())
    }

    fn start_heartbeat(&self) {
        let id = self.id.clone();
        let socket = self.socket.clone();
        let clients_clone = self.clients.clone();

        tokio::spawn(async move {
            let message = Message { action: Action::Check(id) };

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(HEARTBEAT)).await;
                socket.send_to(&serde_json::to_vec(&message).unwrap(), SocketAddr::from(([255, 255, 255, 255], 65056))).await.unwrap();

                let mut clients = clients_clone.lock().unwrap();
                let now = Utc::now().timestamp();

                let dead_clients = clients
                    .iter()
                    .filter(|(_, client)| now - client.last_seen > THRESHOLD)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<String>>();

                for id in dead_clients {
                    println!("Client dead: {}", id);
                    clients.remove(&id);
                }
            }
        });
    }

    fn handle_action(&self) -> Sender<(Vec<u8>, SocketAddr)> {
        self.start_heartbeat();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Vec<u8>, SocketAddr)>(MAX_CONNECTIONS);

        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::spawn(async move {
            while let Some((bytes, addr)) = rx.recv().await {
                let message: Message = match serde_json::from_slice(&bytes) {
                    Ok(message) => message,
                    Err(e) => {
                        // let string = String::from_utf8(bytes).unwrap();
                        // println!("Error: {}", string);
                        println!("Error: {e}");

                        continue;
                    },
                };

                let mut client_id = None;
                match message.action {
                    Action::Join(id) => {
                        if id != server_id {
                            client_id = Some(id.clone());

                            if !clients.lock().unwrap().contains_key(&id) {
                                let bytes = serde_json::to_vec(&Message { action: Action::Join(server_id.clone()) }).unwrap();
                                tx_sender.as_ref().unwrap().send((bytes, Some(addr))).await.unwrap();
                            }
                        }
                    },
                    Action::Check(id) => {
                        if id != server_id {
                            client_id = Some(id.clone());
                            let mut clients = clients.lock().unwrap();
                            clients.entry(id).and_modify(|client| { client.last_seen = Utc::now().timestamp(); });
                        }
                    },
                    // Action::Test(num) => {
                    //     println!("Test: {}", num);

                    //     // for each client spawn a task that sends a message to the client
                    //     let clients = clients.lock().unwrap();
                    //     for (_id, client) in clients.iter() {
                    //         let tx_sender = tx_sender.clone();
                    //         let addr = client.address.clone();

                    //         tokio::spawn(async move {
                    //             let bytes = serde_json::to_vec(&Message { action: Action::Test(num) }).unwrap();
                    //             tx_sender.as_ref().unwrap().send((bytes, Some(addr))).await.unwrap();
                    //         });
                    //     }
                    // },
                }

                if let Some(id) = client_id {
                    let now = Utc::now().timestamp();

                    clients.lock().unwrap().entry(id).and_modify(|client| { client.last_seen = now; }).or_insert(Client { last_seen: now, address: addr });
                }

            }
        });

        tx
    }

    fn to_udp(&self) -> Sender<(Vec<u8>, Option<SocketAddr>)> {
        let socket = self.socket.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Vec<u8>, Option<SocketAddr>)>(MAX_CONNECTIONS);

        tokio::task::spawn(async move {
            while let Some((bytes, addr)) = rx.recv().await {
                let addr = match addr {
                    Some(addr) => addr,
                    None => SocketAddr::from(([255, 255, 255, 255], 65056)),
                };

                socket.send_to(&bytes, addr).await.unwrap();
            }
        });

        tx
    }

    fn from_udp(&self) {
        let socket = self.socket.clone();
        let tx = self.tx_handler.clone();

        tokio::task::spawn(async move {
            let mut buf = [0u8; BUFFER_SIZE];

            loop {
                let (len, addr) = socket.recv_from(&mut buf).await.unwrap();
                tx.clone().unwrap().send((buf[..len].to_vec(), addr)).await.unwrap();
            }
        });
    }
}
