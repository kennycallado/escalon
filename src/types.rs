use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

pub struct Client {
    pub address: SocketAddr,
    pub last_seen: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Action {
    Join(String),
    Check(String),
    // Test(u64),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub action: Action,
}
