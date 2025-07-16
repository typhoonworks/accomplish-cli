use crate::api::errors::ApiError;
use crate::user_agent::generate_user_agent;
use reqwest::Client;
use serde::de::DeserializeOwned;

pub struct ApiClient {
    base_url: String,
    access_token: Option<String>,
    client: Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent(generate_user_agent())
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.to_string(),
            access_token: None,
            client,
        }
    }

    // pub fn new_with_token(base_url: String, access_token: Option<String>) -> Self {
    //     Self { base_url, access_token }
    // }

    pub fn set_access_token(&mut self, token: String) {
        self.access_token = Some(token);
    }

    // pub fn clear_access_token(&mut self) {
    //     self.access_token = None;
    // }

    pub async fn get<T>(&self, endpoint: &str, use_auth: bool) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let full_url = format!("{}/{}", self.base_url, endpoint);

        let mut request = self.client.get(&full_url);

        if use_auth {
            if let Some(token) = &self.access_token {
                request = request.bearer_auth(token);
            } else {
                return Err(ApiError::Unauthorized(
                    "Authorization required but no token is set.".into(),
                ));
            }
        }

        let response = request.send().await;

        match response {
            Ok(resp) if resp.status().is_success() => resp
                .json::<T>()
                .await
                .map_err(|e| ApiError::DecodeError(e.to_string())),
            Ok(resp) => match resp.status().as_u16() {
                400 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Bad Request".to_string());
                    Err(ApiError::BadRequest(error_msg))
                }
                401 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unauthorized".to_string());
                    Err(ApiError::Unauthorized(error_msg))
                }
                404 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Not Found".to_string());
                    Err(ApiError::NotFound(error_msg))
                }
                422 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unprocessable Entity".to_string());
                    Err(ApiError::InvalidInput(error_msg))
                }
                500 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Internal Server Error".to_string());
                    Err(ApiError::ServerError(error_msg))
                }
                _ => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unexpected Error".to_string());
                    Err(ApiError::Unexpected(error_msg))
                }
            },
            Err(e) => Err(ApiError::Unexpected(e.to_string())),
        }
    }

    pub async fn post<T>(
        &self,
        endpoint: &str,
        body: serde_json::Value,
        use_auth: bool,
    ) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let full_url = format!("{}/{}", self.base_url, endpoint);

        let mut request = self.client.post(&full_url).json(&body);

        if use_auth {
            if let Some(token) = &self.access_token {
                request = request.bearer_auth(token);
            } else {
                return Err(ApiError::Unauthorized(
                    "Authorization required but no token is set.".into(),
                ));
            }
        }

        let response = request.send().await;

        match response {
            Ok(resp) if resp.status().is_success() => resp
                .json::<T>()
                .await
                .map_err(|e| ApiError::DecodeError(e.to_string())),
            Ok(resp) => match resp.status().as_u16() {
                400 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Bad Request".to_string());
                    Err(ApiError::BadRequest(error_msg))
                }
                401 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unauthorized".to_string());
                    Err(ApiError::Unauthorized(error_msg))
                }
                404 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Not Found".to_string());
                    Err(ApiError::NotFound(error_msg))
                }
                422 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unprocessable Entity".to_string());
                    Err(ApiError::InvalidInput(error_msg))
                }
                500 => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Internal Server Error".to_string());
                    Err(ApiError::ServerError(error_msg))
                }
                _ => {
                    let error_msg = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unexpected Error".to_string());
                    Err(ApiError::Unexpected(error_msg))
                }
            },
            Err(e) => Err(ApiError::Unexpected(e.to_string())),
        }
    }
}
