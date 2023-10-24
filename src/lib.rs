mod builder;
mod constants;
mod implementations;
mod types;

use builder::{Manager, NoManager, NoService, Service};
pub use tokio;

#[cfg(test)]
mod tests;

use async_trait::async_trait;
use local_ip_address::local_ip;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{Addr, Id, Port};
use crate::builder::{NoAddr, NoId, NoPort};
pub use crate::types::client::{ClientState, EscalonClient};
use crate::types::message::Message;

pub struct Distrib {
    pub client_id: String,
    pub take_from: String,
    pub start_at: usize,
    pub n_jobs: usize,
    pub done: bool,
}

#[async_trait]
pub trait EscalonTrait: Send + Sync + 'static {
    fn count(&self) -> usize;
    async fn take_jobs(
        &self,
        take_from: String,
        start_at: usize,
        n_jobs: usize,
    ) -> Result<Vec<String>, ()>;
    async fn drop_jobs(&self, jobs: Vec<String>) -> Result<(), ()>;
}

#[derive(Clone)]
pub struct Escalon {
    id: String,
    address: SocketAddr,
    start_time: std::time::SystemTime,

    pub clients: Arc<Mutex<HashMap<String, EscalonClient>>>,
    distribution: Arc<Mutex<Vec<Distrib>>>,

    manager: Arc<dyn EscalonTrait>,

    socket: Arc<UdpSocket>,
    service: IpAddr,
    // TODO
    // quizá en un Arc
    tx_handler: Option<Sender<Message>>,
    tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
}

impl Escalon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonBuilder<NoId, NoAddr, NoService, NoPort, NoManager> {
        EscalonBuilder {
            id: NoId,
            addr: NoAddr,
            svc: NoService,
            port: NoPort,
            manager: NoManager,
        }
    }
}

pub struct EscalonBuilder<I, A, S, P, F> {
    id: I,
    addr: A,
    svc: S,
    port: P,
    manager: F,
}

impl EscalonBuilder<Id, Addr, Service, Port, Manager> {
    pub async fn build(self) -> Escalon {
        let socket =
            UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await.unwrap();
        socket.set_broadcast(true).unwrap();

        Escalon {
            id: self.id.0,
            address: SocketAddr::new(local_ip().unwrap(), self.port.0),
            start_time: std::time::SystemTime::now(),
            manager: self.manager.0,
            clients: Arc::new(Mutex::new(HashMap::new())),
            distribution: Arc::new(Mutex::new(Vec::new())),
            service: self.svc.0,
            socket: Arc::new(socket),
            tx_handler: None,
            tx_sender: None,
        }
    }
}
