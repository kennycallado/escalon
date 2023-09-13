use serde::{Deserialize, Serialize};

use crate::types::client::ClientState;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Message<J> {
    pub action: Action<J>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Action<J> {
    Join((String, std::time::SystemTime)),
    Check((String, ClientState<J>)),
    // Test(u64),
}
