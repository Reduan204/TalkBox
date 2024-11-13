use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessages {
    Connected { id: String },
    ServerError { message: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessages {
    Connect { username: String },
    Message { id: String, content: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub user_id: String,
    pub username: String,
    pub content: String,
}
