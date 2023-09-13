use serde::{Deserialize, Serialize};

use crate::client::ClientState;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Message {
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Action {
    Join((String, std::time::SystemTime)),
    Check((String, ClientState)),
    // Test(u64),
}
