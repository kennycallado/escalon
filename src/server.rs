use std::net::IpAddr;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use anyhow::anyhow;

use chrono::Utc;

use tokio::net::UdpSocket;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::Sender;

use crate::types::{Client, Message, Action};
use crate::constants::{THRESHOLD_SECS, HEARTBEAT_SECS, BUFFER_SIZE, MAX_CONNECTIONS};

pub struct Server {
    id: String,
    socket: Arc<UdpSocket>,
    clients: Arc<Mutex<HashMap<String, Client>>>,
    tx_handler: Option<Sender<(Message, SocketAddr)>>,
    tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
    tx_up: Sender<Message>,
}

pub struct ServerBuilder {
    id: String,
    addr: Option<IpAddr>,
    port: Option<u16>,
    tx_up: Option<Sender<Message>>,
}

impl ServerBuilder {
    pub fn new(id: String) -> Self {
        Self {
            id,
            addr: None,
            port: None,
            tx_up: None,
        }
    }

    pub fn set_addr(mut self, addr: IpAddr) -> Self {
        self.addr = Some(addr);

        self
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = Some(port);

        self
    }

    pub fn set_sender(mut self, tx_up: Sender<Message>) -> Self {
        self.tx_up = Some(tx_up);

        self
    }

    pub async fn build(self) -> Result<Server> {
        let addr = self.addr.ok_or(anyhow!("Address not set"))?;
        let port = self.port.ok_or(anyhow!("Port not set"))?;
        let tx_up = self.tx_up.ok_or(anyhow!("Sender not set"))?;

        let socket = UdpSocket::bind(format!("{:?}:{}", addr, port)).await?;
        socket.set_broadcast(true)?;

        let server = Server {
            id: self.id,
            socket: Arc::new(socket),
            clients: Arc::new(Mutex::new(HashMap::new())),
            tx_handler: None,
            tx_sender: None,
            tx_up,
        };
        
        Ok(server)
    }
}

impl Server {
    pub async fn listen(&mut self) -> Result<()> {
    // pub async fn listen(&mut self, cb: impl FnOnce(String, String, String)) -> Result<()> {
        // bind
        // let socket = UdpSocket::bind(&self.addr).await?;
        // socket.set_broadcast(true)?;

        // sets
        // self.socket = Arc::new(socket);
        self.tx_sender = Some(self.to_udp()?);
        self.tx_handler = Some(self.handle_action()?);

        // join and listen
        self.send_join().await?;
        self.from_udp()?;

        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = sigterm.recv() => { },
            _ = tokio::signal::ctrl_c() => { },
        }

        // signal(SignalKind::terminate())?.recv().await;
        println!(" ->> Shutting down the server");

        Ok(())
    }

    async fn send_join(&self) -> Result<()> {
        let tx = self.tx_sender.as_ref().unwrap();
        let message = Message { action: Action::Join(self.id.clone()) };

        tx.send((message, None)).await?;

        Ok(())
    }

    fn start_heartbeat(&self) -> Result<()> {
        let id = self.id.clone();
        let clients = self.clients.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::spawn(async move {
            let message = Message { action: Action::Check(id) };

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
        self.start_heartbeat()?;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Message, SocketAddr)>(MAX_CONNECTIONS);

        let clients = self.clients.clone();
        let server_id = self.id.clone();
        let tx_sender = self.tx_sender.clone();

        tokio::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                match msg.action {
                    Action::Join(id) => {
                        if id != server_id {
                            update_timestamp_or_insert(&mut clients.lock().unwrap(), id.clone(), addr);

                            if !clients.lock().unwrap().contains_key(&id) {
                                let message = Message { action: Action::Join(server_id.clone()) };

                                tx_sender.as_ref().unwrap().send((message, Some(addr))).await.unwrap();
                            }
                        }
                    },
                    Action::Check(id) => {
                        if id != server_id {
                            update_timestamp_or_insert(&mut clients.lock().unwrap(), id.clone(), addr);

                            // let mut clients = clients.lock().unwrap();
                            // clients.entry(id).and_modify(|client| { client.last_seen = Utc::now().timestamp(); });
                        }
                    },
                }

                fn update_timestamp_or_insert(clients: &mut HashMap<String, Client>, id: String, addr: SocketAddr) {
                    clients
                        .entry(id)
                        .and_modify(|client| { client.last_seen = Utc::now().timestamp() })
                        .or_insert(Client { address: addr, last_seen: Utc::now().timestamp() })
                    ;
                }
            }
        });

        Ok(tx)
    }

    fn to_udp(&self) -> Result<Sender<(Message, Option<SocketAddr>)>> {
        let socket = self.socket.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(Message, Option<SocketAddr>)>(MAX_CONNECTIONS);

        tokio::task::spawn(async move {
            while let Some((msg, addr)) = rx.recv().await {
                let bytes = match serde_json::to_vec(&msg) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        println!("Error: {e}");

                        continue;
                    },
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
                    },
                };


                tx.clone().unwrap().send((message, addr)).await.unwrap();
            }
        });

        Ok(())
    }
}
