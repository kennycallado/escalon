mod builder;
mod constants;
mod implementations;
mod types;

pub use tokio;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{AddJobs, Addr, Count, Id, Port};
use crate::builder::{NoAddJobs, NoAddr, NoCount, NoId, NoPort};
use crate::types::client::{Client, ClientState};
use crate::types::message::Message;

type FnAddJobs = dyn Fn(String, usize, usize) + Send + Sync;
type FnCountJobs = dyn Fn() -> usize + Send + Sync;
type Distrib = (String, String, usize, usize, bool);

#[derive(Clone)]
pub struct EscalonFunctions {
    count: Arc<FnCountJobs>,
    add_from: Arc<FnAddJobs>, // server_id, from, jobs_to_add
}

#[derive(Clone)]
pub struct Escalon {
    id: String,
    start_time: std::time::SystemTime,

    functions: EscalonFunctions,

    clients: Arc<Mutex<HashMap<String, Client>>>,
    distribution: Arc<Mutex<Vec<Distrib>>>,

    socket: Arc<UdpSocket>,
    tx_handler: Option<Sender<(Message, SocketAddr)>>,
    tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
}

impl Escalon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonBuilder<NoId, NoAddr, NoPort, NoCount, NoAddJobs> {
        EscalonBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
            count: NoCount,
            add_jobs: NoAddJobs,
        }
    }
}

pub struct EscalonBuilder<I, A, P, C, J> {
    id: I,
    addr: A,
    port: P,
    count: C,
    add_jobs: J,
}

impl EscalonBuilder<Id, Addr, Port, Count, AddJobs> {
    pub async fn build(self) -> Escalon {
        let socket =
            UdpSocket::bind(format!("{:?}:{}", self.addr.0, self.port.0)).await.unwrap();
        socket.set_broadcast(true).unwrap();

        let functions = EscalonFunctions {
            count: self.count.0,
            add_from: self.add_jobs.0,
        };

        Escalon {
            id: self.id.0,
            start_time: std::time::SystemTime::now(),
            functions,
            clients: Arc::new(Mutex::new(HashMap::new())),
            distribution: Arc::new(Mutex::new(Vec::new())),
            socket: Arc::new(socket),
            tx_handler: None,
            tx_sender: None,
        }
    }
}
