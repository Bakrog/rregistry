use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Blob {
    pub digest: String,
    pub bytes: Vec<u8>,
}
