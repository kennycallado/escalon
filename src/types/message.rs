use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub action: Action,
}

///
/// First String always refers to the emisor of the message
///
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Action {
    Join((String, std::time::SystemTime)),
    Check((String, usize)),
    FoundDead((String, String)), // for now just the id
}
