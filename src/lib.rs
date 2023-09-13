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

use builder::NoId;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

use crate::builder::{EscalonBuilder, NoAddr, NoPort};
use crate::types::client::{Client, ClientState};
use crate::types::message::Message;

pub struct Escalon<J> {
    pub id: String,
    pub clients: Arc<Mutex<HashMap<String, Client<J>>>>,
    pub own_state: Arc<Mutex<ClientState<Arc<Mutex<J>>>>>,
    pub socket: Arc<UdpSocket>,
    pub start_time: std::time::SystemTime,
    pub tx_handler: Option<Sender<(Message<J>, SocketAddr)>>,
    pub tx_sender: Option<Sender<(Message<J>, Option<SocketAddr>)>>,
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
