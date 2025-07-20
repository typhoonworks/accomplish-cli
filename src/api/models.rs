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

#[derive(Debug, serde::Deserialize)]
pub struct RecapResponse {
    pub recap_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sse_url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RecapStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters: Option<RecapFilters>,
    #[serde(default, deserialize_with = "deserialize_optional_metadata")]
    pub metadata: Option<RecapMetadata>,
}

fn deserialize_optional_metadata<'de, D>(deserializer: D) -> Result<Option<RecapMetadata>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Helper {
        #[serde(default)]
        entry_count: u32,
        #[serde(default)]
        projects: Vec<String>,
        #[serde(default)]
        tags: Vec<String>,
    }

    let helper = Option::<Helper>::deserialize(deserializer)?;
    Ok(helper.map(|h| RecapMetadata {
        entry_count: h.entry_count,
        projects: h.projects,
        tags: h.tags,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct RecapFilters {
    #[serde(default)]
    pub project_ids: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RecapMetadata {
    #[serde(default)]
    pub entry_count: u32,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct SseEvent {
    pub recap_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u32>,
}
