use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Client<J> {
    pub address: SocketAddr,
    pub last_seen: i64,
    pub start_time: std::time::SystemTime,
    pub state: ClientState<J>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ClientState<J> {
    pub memory: usize,
    pub jobs: J,
}
