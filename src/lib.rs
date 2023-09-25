mod builder;
mod constants;
mod implementations;
mod types;

use builder::{Manager, NoManager};
pub use tokio;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{Addr, Id, Port};
use crate::builder::{NoAddr, NoId, NoPort};
use crate::types::client::{Client, ClientState};
use crate::types::message::Message;

type Distrib = (String, String, usize, usize, bool);

#[async_trait]
pub trait EscalonTrait: Send + Sync + 'static {
    fn count(&self) -> usize;
    async fn take_jobs(
        &self,
        from_client: String,
        start_at: usize,
        n_jobs: usize,
    ) -> Result<Vec<String>, ()>;
    async fn drop_jobs(&self, jobs: Vec<String>) -> Result<(), ()>;
}

#[derive(Clone)]
pub struct Escalon {
    id: String,
    start_time: std::time::SystemTime,

    clients: Arc<Mutex<HashMap<String, Client>>>,
    distribution: Arc<Mutex<Vec<Distrib>>>,

    manager: Arc<dyn EscalonTrait>,

    socket: Arc<UdpSocket>,
    // TODO
    // quizá en un Arc
    tx_handler: Option<Sender<(Message, SocketAddr)>>,
    tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
}

impl Escalon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonBuilder<NoId, NoAddr, NoPort, NoManager> {
        EscalonBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
            manager: NoManager,
        }
    }
}

pub struct EscalonBuilder<I, A, P, F> {
    id: I,
    addr: A,
    port: P,
    manager: F,
}

impl EscalonBuilder<Id, Addr, Port, Manager> {
    pub async fn build(self) -> Escalon {
        let socket =
            UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await.unwrap();
        socket.set_broadcast(true).unwrap();

        Escalon {
            id: self.id.0,
            start_time: std::time::SystemTime::now(),
            manager: self.manager.0,
            clients: Arc::new(Mutex::new(HashMap::new())),
            distribution: Arc::new(Mutex::new(Vec::new())),
            socket: Arc::new(socket),
            tx_handler: None,
            tx_sender: None,
        }
    }
}
