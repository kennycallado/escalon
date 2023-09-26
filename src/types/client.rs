// use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct Client {
    pub address: SocketAddr,
    pub last_seen: i64,
    pub start_time: std::time::SystemTime,
    pub state: ClientState,
}

#[derive(Clone, Debug)]
pub struct ClientState {
    // pub memory: usize,
    pub jobs: usize,
}
