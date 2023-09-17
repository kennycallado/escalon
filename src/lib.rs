mod builder;
mod constants;
mod implementations;
mod types;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{EscalonBuilder, NoAddr, NoCount, NoId, NoPort};
use crate::types::client::{Client, ClientState};
use crate::types::message::Message;

#[derive(Clone)]
pub struct Escalon {
    pub id: String,
    pub clients: Arc<Mutex<HashMap<String, Client>>>,
    pub own_state: Arc<dyn Fn() -> usize + Send + Sync>,
    pub socket: Arc<UdpSocket>,
    pub start_time: std::time::SystemTime,
    pub tx_handler: Option<Sender<(Message, SocketAddr)>>,
    pub tx_sender: Option<Sender<(Message, Option<SocketAddr>)>>,
    pub addr: Arc<Mutex<Option<SocketAddr>>>,
}

impl Escalon {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> EscalonBuilder<NoId, NoAddr, NoPort, NoCount> {
        EscalonBuilder {
            id: NoId,
            addr: NoAddr,
            port: NoPort,
            count: NoCount,
        }
    }
}
