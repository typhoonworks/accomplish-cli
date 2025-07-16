use crate::api::client::ApiClient;
use crate::api::endpoints::check_token_info;
use crate::api::errors::ApiError;
use crate::errors::{AppError, UnauthenticatedError};
use crate::storage::{clear_token, load_token, save_token};
use std::path::PathBuf;

pub struct AuthService {
    api_client: ApiClient,
    access_token: Option<String>,
    token_path: PathBuf,
}

impl AuthService {
    /// Initialize with per-profile token_path = `<credentials_dir>/<profile>/token`.
    pub fn new(api_base: String, mut credentials_dir: PathBuf, profile: &str) -> Self {
        credentials_dir.push(profile);
        let token_path = credentials_dir.join("token");
        let access_token = load_token(&token_path).unwrap_or(None);

        let mut api_client = ApiClient::new(&api_base);
        if let Some(ref t) = access_token {
            api_client.set_access_token(t.clone());
        }

        AuthService {
            api_client,
            access_token,
            token_path,
        }
    }

    pub fn api_client(&self) -> &ApiClient {
        &self.api_client
    }

    /// Validate token; clear it on failure.
    pub async fn ensure_authenticated(&mut self) -> Result<(), AppError> {
        if let Some(token) = &self.access_token {
            match check_token_info(self.api_client(), token).await {
                Ok(r) if r.active => Ok(()),
                Ok(_) | Err(ApiError::Unauthorized(_)) => {
                    self.clear_tokens();
                    Err(AppError::Auth(UnauthenticatedError))
                }
                Err(e) => Err(AppError::Api(e)),
            }
        } else {
            Err(AppError::Auth(UnauthenticatedError))
        }
    }

    /// Remove token from memory, disk, and client.
    pub fn clear_tokens(&mut self) {
        self.access_token = None;
        let _ = clear_token(&self.token_path);
        self.api_client.set_access_token(String::new());
    }

    /// Persist new token and set it on the API client.
    pub fn save_access_token(&mut self, token: &str) -> Result<(), AppError> {
        save_token(&self.token_path, token)?;
        self.access_token = Some(token.to_string());
        self.api_client.set_access_token(token.to_string());
        Ok(())
    }
}
