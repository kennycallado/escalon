use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub action: Action,
}

///
/// String always refers to the emisor of the message
///
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Action {
    Join((String, std::time::SystemTime)),
    Check((String, usize)),
    // UpdateDead((String, Client<J>)),
    // Test(u64),
}
