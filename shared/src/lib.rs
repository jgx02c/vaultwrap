pub fn add(left: u64, right: u64) -> u64 {
    left + right
}


use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SecretRequest {
    pub client_id: String,
    pub command: String,
    pub environment: Option<String>, // New: specify which environment to use
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SecretResponse {
    pub success: bool,
    pub env_vars: Option<Vec<(String, String)>>, // key-value pairs
    pub message: Option<String>,
    pub environments: Option<Vec<String>>, // New: list of available environments (for listing)
}

//We're using serde for serialization + bincode or json for sending data across the wire.


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
