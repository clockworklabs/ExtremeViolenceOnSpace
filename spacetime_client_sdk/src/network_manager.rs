use serde::{Deserialize, Serialize};

use crate::errors::ClientError;
use spacetimedb::serde_json;
use spacetimedb::TypeValue;

pub enum TableOp {
    Insert,
    Delete,
    Update,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub fn_: String,
    pub args: Vec<TypeValue>,
}

pub struct NetworkManager {}

impl NetworkManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn internal_call_reducer(&self, msg: &Message) -> Result<(), ClientError> {
        let json = serde_json::to_string(msg)?;

        Ok(())
    }
}
