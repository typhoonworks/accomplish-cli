use crate::api::errors::ApiError;
use crate::api::models::SseEvent;
use crate::user_agent::generate_user_agent;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::pin::Pin;

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
                429 => Err(ApiError::RateLimited),
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
                429 => Err(ApiError::RateLimited),
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

    /// Stream Server-Sent Events from an endpoint
    pub async fn stream_sse(
        &self,
        endpoint: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<SseEvent, ApiError>> + Send>>, ApiError> {
        let full_url = format!("{}/{}", self.base_url, endpoint);

        let mut request = self.client.get(&full_url);

        if let Some(token) = &self.access_token {
            request = request.bearer_auth(token);
        } else {
            return Err(ApiError::Unauthorized(
                "Authorization required but no token is set.".into(),
            ));
        }

        let response = request
            .send()
            .await
            .map_err(|e| ApiError::Unexpected(e.to_string()))?;

        // Check if we got an error response instead of SSE stream
        if !response.status().is_success() {
            return match response.status().as_u16() {
                404 => {
                    let error_msg = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Stream not found".to_string());
                    Err(ApiError::NotFound(error_msg))
                }
                _ => {
                    let error_msg = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "SSE connection failed".to_string());
                    Err(ApiError::Unexpected(error_msg))
                }
            };
        }

        let stream = response
            .bytes_stream()
            .map(|chunk_result| match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    parse_sse_events(&text)
                }
                Err(e) => vec![Err(ApiError::Unexpected(format!("Stream error: {}", e)))],
            })
            .flat_map(futures::stream::iter);

        Ok(Box::pin(stream))
    }
}

/// Parse SSE events from text
fn parse_sse_events(text: &str) -> Vec<Result<SseEvent, ApiError>> {
    let mut events = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        // Look for data: lines in SSE format
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim().is_empty() {
                continue;
            }

            // Try to parse the JSON data
            match serde_json::from_str::<SseEvent>(data) {
                Ok(event) => events.push(Ok(event)),
                Err(e) => {
                    // Try to parse as a generic error response
                    if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(error_msg) = error_obj.get("error").and_then(|v| v.as_str()) {
                            events.push(Err(ApiError::NotFound(error_msg.to_string())));
                        } else {
                            events.push(Err(ApiError::DecodeError(format!(
                                "Failed to parse SSE event: {}",
                                e
                            ))));
                        }
                    } else {
                        events.push(Err(ApiError::DecodeError(format!(
                            "Failed to parse SSE event: {}",
                            e
                        ))));
                    }
                }
            }
        }
    }

    events
}
