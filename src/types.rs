use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Client {
    pub address: SocketAddr,
    pub last_seen: i64,
    pub start_time: std::time::SystemTime,
    pub state: ClientState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClientState {
    pub memory: usize,
    pub tasks: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Action {
    Join((String, std::time::SystemTime)),
    Check((String, ClientState)),
    // Test(u64),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Message {
    pub action: Action,
}
