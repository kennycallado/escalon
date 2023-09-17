use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Client {
    pub address: SocketAddr,
    pub last_seen: i64,
    pub start_time: std::time::SystemTime,
    pub state: ClientState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientState {
    // pub memory: usize,
    pub jobs: usize,
}
