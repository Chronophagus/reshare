use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Error {
    pub error_msg: String,
}
