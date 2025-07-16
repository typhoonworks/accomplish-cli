// src/api/types.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub interval: u64,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct TokenInfoResponse {
    pub active: bool,
    pub scope: String,
    pub client_id: String,
    pub username: Option<String>,
    pub exp: u64,
}
