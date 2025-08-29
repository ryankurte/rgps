
use serde::{Serialize, Deserialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Req {
    Ping,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Resp {
    Pong,
}
