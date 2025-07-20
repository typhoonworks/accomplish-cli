use crate::api::client::ApiClient;
use crate::api::errors::ApiError;
use crate::api::models::{
    DeviceCodeResponse, RecapResponse, RecapStatusResponse, TokenInfoResponse, TokenResponse,
};
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use serde_json::{json, Value};

// Scopes requested by the official CLI
const CLI_SCOPES: &str = concat!(
    "user:read,user:write,",
    "project:read,project:write,",
    "worklog:read,worklog:write,",
    "repo:read,repo:write"
);

/// Formats a date string in YYYY-MM-DD format to ISO8601 datetime format.
/// For 'from' dates, uses start of day (00:00:00).
/// For 'to' dates, uses end of day (23:59:59).
fn format_date_for_api(date_str: &str, is_end_of_day: bool) -> Result<String, ApiError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
        ApiError::InvalidInput(format!(
            "Invalid date format: {date_str}. Expected YYYY-MM-DD"
        ))
    })?;

    let time = if is_end_of_day {
        NaiveTime::from_hms_opt(23, 59, 59).unwrap()
    } else {
        NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    };

    let datetime = date.and_time(time);
    let utc_datetime = Utc.from_utc_datetime(&datetime);

    Ok(utc_datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Initiates the OAuth device code flow, requesting all CLI scopes.
pub async fn initiate_device_code(
    api_client: &ApiClient,
    client_id: &str,
) -> Result<DeviceCodeResponse, ApiError> {
    let body = json!({
        "client_id": client_id,
        "scope": CLI_SCOPES,
    });

    api_client.post("auth/device/code", body, false).await
}

/// Exchanges a device code for an access token.
pub async fn exchange_device_code_for_token(
    api_client: &ApiClient,
    device_code: &str,
) -> Result<TokenResponse, ApiError> {
    let body = json!({
        "device_code": device_code,
    });

    api_client.post("auth/device/token", body, false).await
}

/// Checks the validity of an existing token.
pub async fn check_token_info(
    api_client: &ApiClient,
    token: &str,
) -> Result<TokenInfoResponse, ApiError> {
    let body = json!({ "token": token });

    let response: TokenInfoResponse = api_client.post("auth/token_info", body, true).await?;
    if response.active {
        Ok(response)
    } else {
        Err(ApiError::Unauthorized("Token is inactive".into()))
    }
}

/// Creates a new worklog entry.
pub async fn create_worklog_entry(
    api_client: &ApiClient,
    content: &str,
    recorded_at: &str,
    tags: &[String],
    project_id: Option<&str>,
) -> Result<Value, ApiError> {
    let mut body = json!({
        "content": content,
        "recorded_at": recorded_at,
    });

    if !tags.is_empty() {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("tags".to_string(), json!(tags));
        }
    }

    if let Some(id) = project_id {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("project_id".to_string(), json!(id));
        }
    }

    api_client.post("api/v1/worklog/entries", body, true).await
}

/// Associates commits with a worklog entry.
pub async fn associate_commits_with_entry(
    api_client: &ApiClient,
    entry_id: &str,
    commit_ids: &[String],
) -> Result<Value, ApiError> {
    let body = json!({
        "commit_ids": commit_ids
    });

    let endpoint = format!("api/v1/worklog/entries/{entry_id}/commits");
    api_client.post(&endpoint, body, true).await
}

/// Fetches all projects for the current user.
pub async fn fetch_projects(api_client: &ApiClient) -> Result<Value, ApiError> {
    api_client.get("api/v1/projects", true).await
}

/// Fetches all repositories for the current user.
pub async fn fetch_repositories(api_client: &ApiClient) -> Result<Value, ApiError> {
    api_client.get("api/v1/repositories", true).await
}

/// Creates a new project.
pub async fn create_project(
    api_client: &ApiClient,
    name: &str,
    description: Option<&str>,
    identifier: Option<&str>,
) -> Result<Value, ApiError> {
    let mut body = json!({
        "name": name,
    });

    if let Some(desc) = description {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("description".to_string(), json!(desc));
        }
    }

    if let Some(id) = identifier {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("identifier".to_string(), json!(id));
        }
    }

    api_client.post("api/v1/projects", body, true).await
}

/// Creates a new repository.
pub async fn create_repo(
    api_client: &ApiClient,
    name: &str,
    project_id: &str,
    local_path: Option<&str>,
    remote_url: Option<&str>,
    default_branch: Option<&str>,
) -> Result<Value, ApiError> {
    let mut body = json!({
        "name": name,
        "project_id": project_id,
    });

    if let Some(path) = local_path {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("local_path".to_string(), json!(path));
        }
    }

    if let Some(url) = remote_url {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("remote_url".to_string(), json!(url));
        }
    }

    if let Some(branch) = default_branch {
        if let Some(obj) = body.as_object_mut() {
            obj.insert("default_branch".to_string(), json!(branch));
        }
    }

    api_client.post("api/v1/repositories", body, true).await
}

/// Fetches uncaptured commits for a repository.
pub async fn fetch_uncaptured_commits(
    api_client: &ApiClient,
    repo_id: &str,
    commit_shas: &[String],
) -> Result<Value, ApiError> {
    let shas_param = commit_shas.join(",");
    let endpoint =
        format!("api/v1/repositories/{repo_id}/commits?uncaptured=true&shas={shas_param}");
    api_client.get(&endpoint, true).await
}

/// Creates commits for a repository.
pub async fn create_commits(
    api_client: &ApiClient,
    repo_id: &str,
    commits: &[CommitData],
) -> Result<Value, ApiError> {
    let body = json!({
        "commits": commits
    });

    let endpoint = format!("api/v1/repositories/{repo_id}/commits");
    api_client.post(&endpoint, body, true).await
}

/// Represents commit data for API requests.
#[derive(Debug, serde::Serialize)]
pub struct CommitData {
    pub sha: String,
    pub message: Option<String>,
    pub committed_at: Option<String>,
}

/// Fetches worklog entries with optional filtering.
pub async fn fetch_worklog_entries(
    api_client: &ApiClient,
    project_id: Option<&str>,
    tags: Option<&[String]>,
    from: Option<&str>,
    to: Option<&str>,
    limit: u32,
    starting_after: Option<&str>,
) -> Result<Value, ApiError> {
    let mut params = vec![format!("limit={}", limit)];

    if let Some(project) = project_id {
        params.push(format!("project_id={project}"));
    }

    if let Some(tags_list) = tags {
        if !tags_list.is_empty() {
            params.push(format!("tags={}", tags_list.join(",")));
        }
    }

    if let Some(from_date) = from {
        let formatted_date = format_date_for_api(from_date, false)?;
        params.push(format!("from={formatted_date}"));
    }

    if let Some(to_date) = to {
        let formatted_date = format_date_for_api(to_date, true)?;
        params.push(format!("to={formatted_date}"));
    }

    if let Some(cursor) = starting_after {
        params.push(format!("starting_after={cursor}"));
    }

    let query = if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    };

    let endpoint = format!("api/v1/worklog/entries{query}");
    api_client.get(&endpoint, true).await
}

/// Generates a new worklog recap using the API
pub async fn generate_worklog_recap(
    api_client: &ApiClient,
    from: Option<&str>,
    to: Option<&str>,
    project_ids: Option<&[String]>,
    tags: Option<&[String]>,
) -> Result<RecapResponse, ApiError> {
    let mut params = Vec::new();

    if let Some(from_date) = from {
        let formatted_date = format_date_for_api(from_date, false)?;
        params.push(format!("from={formatted_date}"));
    }

    if let Some(to_date) = to {
        let formatted_date = format_date_for_api(to_date, true)?;
        params.push(format!("to={formatted_date}"));
    }

    if let Some(projects) = project_ids {
        if !projects.is_empty() {
            params.push(format!("project_ids={}", projects.join(",")));
        }
    }

    if let Some(tags_list) = tags {
        if !tags_list.is_empty() {
            params.push(format!("tags={}", tags_list.join(" ")));
        }
    }

    let query = if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    };

    let endpoint = format!("api/v1/worklog/recaps{query}");
    api_client.post(&endpoint, json!({}), true).await
}

/// Fetches the status and content of a recap by ID
pub async fn get_recap_status(
    api_client: &ApiClient,
    recap_id: &str,
) -> Result<RecapStatusResponse, ApiError> {
    let endpoint = format!("api/v1/worklog/recaps/{recap_id}");
    api_client.get(&endpoint, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url, Matcher};
    use serde_json::{json, Value};
    use tokio;

    #[tokio::test]
    async fn test_initiate_device_code() {
        let _m = mock("POST", "/auth/device/code")
            .match_body(Matcher::Json(json!({
                "client_id": "test-client-id",
                "scope": CLI_SCOPES
            })))
            .with_status(200)
            .with_body(
                json!({
                    "device_code": "device_code_123",
                    "user_code": "user_code_456",
                    "verification_uri": "http://example.com",
                    "verification_uri_complete": "http://example.com?user_code=user_code_456",
                    "interval": 5
                })
                .to_string(),
            )
            .create();

        let api_client = ApiClient::new(&mockito::server_url());
        let got = initiate_device_code(&api_client, "test-client-id")
            .await
            .expect("Expected Ok");
        assert_eq!(got.user_code, "user_code_456");
        assert_eq!(got.verification_uri, "http://example.com");
        assert_eq!(
            got.verification_uri_complete,
            "http://example.com?user_code=user_code_456"
        );
    }

    #[tokio::test]
    async fn test_exchange_device_code_for_token() {
        let _m = mock("POST", "/auth/device/token")
            .match_body(Matcher::Json(json!({
                "device_code": "device_code_123"
            })))
            .with_status(200)
            .with_body(
                json!({
                    "access_token": "access_token_789",
                    "token_type": "bearer",
                    "expires_in": 3600,
                    "refresh_token": "refresh_token_101",
                    "scope": CLI_SCOPES
                })
                .to_string(),
            )
            .create();

        let api_client = ApiClient::new(&mockito::server_url());
        let tok = exchange_device_code_for_token(&api_client, "device_code_123")
            .await
            .expect("Expected Ok");

        assert_eq!(tok.access_token, "access_token_789");
        assert_eq!(tok.token_type, "bearer");
        assert_eq!(tok.expires_in, 3600);
        assert_eq!(tok.refresh_token, "refresh_token_101");
        assert_eq!(tok.scope, CLI_SCOPES);
    }

    #[tokio::test]
    async fn test_create_worklog_entry() {
        let payload = json!({
            "content": "Test entry",
            "recorded_at": "2025-05-16T12:00:00Z"
        });

        let response_body = json!({
            "id": "abcd-1234-uuid",
            "content": "Test entry",
            "recorded_at": "2025-05-16T12:00:00Z",
            "tags": [],
            "url": "/api/v1/worklog/entries/abcd-1234-uuid"
        })
        .to_string();

        let _m = mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload.clone()))
            .with_status(201)
            .with_body(response_body.clone())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        // Set a dummy token so that use_auth = true won't fail
        api_client.set_access_token("dummy-token".into());

        let resp =
            create_worklog_entry(&api_client, "Test entry", "2025-05-16T12:00:00Z", &[], None)
                .await
                .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("abcd-1234-uuid")
        );
        assert_eq!(
            resp.get("content").and_then(Value::as_str),
            Some("Test entry")
        );
        assert_eq!(
            resp.get("recorded_at").and_then(Value::as_str),
            Some("2025-05-16T12:00:00Z")
        );
    }

    #[tokio::test]
    async fn test_fetch_projects() {
        let response = json!({
            "projects": [
                {
                    "id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
                    "name": "website",
                    "slug": "website",
                    "description": "Company website",
                    "company": "Acme Inc",
                    "role": "Developer",
                    "start_date": "2025-01-01",
                    "end_date": null,
                    "url": "/api/v1/projects/3fa85f64-5717-4562-b3fc-2c963f66afa6",
                    "inserted_at": "2025-05-16T12:00:00Z",
                    "updated_at": "2025-05-16T12:00:00Z"
                },
                {
                    "id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
                    "name": "internal-ops",
                    "slug": "internal-ops",
                    "description": "Internal operations",
                    "company": "Acme Inc",
                    "role": "Manager",
                    "start_date": "2025-02-01",
                    "end_date": null,
                    "url": "/api/v1/projects/7c9e6679-7425-40de-944b-e07fc1f90ae7",
                    "inserted_at": "2025-05-16T12:00:00Z",
                    "updated_at": "2025-05-16T12:00:00Z"
                }
            ]
        });

        let _m = mock("GET", "/api/v1/projects")
            .match_header("authorization", "Bearer dummy-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let mut api_client = ApiClient::new(&server_url());
        api_client.set_access_token("dummy-token".into());

        let result = fetch_projects(&api_client).await.expect("Expected Ok");

        // Check that we got the projects array
        let projects = result.get("projects").expect("Expected projects key");
        assert!(projects.is_array());
        assert_eq!(projects.as_array().unwrap().len(), 2);

        // Check first project
        let first_project = &projects.as_array().unwrap()[0];
        assert_eq!(first_project["id"], "3fa85f64-5717-4562-b3fc-2c963f66afa6");
        assert_eq!(first_project["name"], "website");

        // Check second project
        let second_project = &projects.as_array().unwrap()[1];
        assert_eq!(second_project["id"], "7c9e6679-7425-40de-944b-e07fc1f90ae7");
        assert_eq!(second_project["name"], "internal-ops");
    }

    #[tokio::test]
    async fn test_create_worklog_entry_with_tags() {
        let payload = json!({
            "content": "Test entry with tags",
            "recorded_at": "2025-05-16T12:00:00Z",
            "tags": ["rust", "cli"]
        });

        let response_body = json!({
            "id": "efgh-5678-uuid",
            "content": "Test entry with tags",
            "recorded_at": "2025-05-16T12:00:00Z",
            "tags": ["rust", "cli"],
            "url": "/api/v1/worklog/entries/efgh-5678-uuid"
        });

        let _m = mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let tags = vec!["rust".to_string(), "cli".to_string()];
        let resp = create_worklog_entry(
            &api_client,
            "Test entry with tags",
            "2025-05-16T12:00:00Z",
            &tags,
            None,
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("efgh-5678-uuid")
        );
        assert_eq!(
            resp.get("content").and_then(Value::as_str),
            Some("Test entry with tags")
        );
        assert_eq!(
            resp.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()),
            Some(vec!["rust", "cli"])
        );
    }

    #[tokio::test]
    async fn test_create_worklog_entry_with_comma_separated_tags() {
        // This simulates what happens when the CLI parses the command line arguments
        let tags_input = ["rust, cli".to_string()];

        // After processing, we expect the tags to be split and trimmed
        let processed_tags: Vec<String> = tags_input
            .iter()
            .flat_map(|s| s.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(processed_tags, vec!["rust", "cli"]);

        let payload = json!({
            "content": "Test entry with comma-separated tags",
            "recorded_at": "2025-05-16T12:00:00Z",
            "tags": ["rust", "cli"]
        });

        let response_body = json!({
            "id": "ijkl-9012-uuid",
            "content": "Test entry with comma-separated tags",
            "recorded_at": "2025-05-16T12:00:00Z",
            "tags": ["rust", "cli"],
            "url": "/api/v1/worklog/entries/ijkl-9012-uuid"
        });

        let _m = mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_worklog_entry(
            &api_client,
            "Test entry with comma-separated tags",
            "2025-05-16T12:00:00Z",
            &processed_tags,
            None,
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("ijkl-9012-uuid")
        );
        assert_eq!(
            resp.get("content").and_then(Value::as_str),
            Some("Test entry with comma-separated tags")
        );
        assert_eq!(
            resp.get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>()),
            Some(vec!["rust", "cli"])
        );
    }

    #[tokio::test]
    async fn test_create_project() {
        let payload = json!({
            "name": "Test Project",
            "description": "A test project",
            "identifier": "tst"
        });

        let response_body = json!({
            "id": "project-uuid-123",
            "name": "Test Project",
            "description": "A test project",
            "identifier": "tst",
            "slug": "test-project",
            "url": "/api/v1/projects/project-uuid-123",
            "inserted_at": "2025-07-07T12:00:00Z",
            "updated_at": "2025-07-07T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/projects")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_project(
            &api_client,
            "Test Project",
            Some("A test project"),
            Some("tst"),
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("project-uuid-123")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("Test Project")
        );
        assert_eq!(resp.get("identifier").and_then(Value::as_str), Some("tst"));
    }

    #[tokio::test]
    async fn test_date_formatting() {
        // Test start of day formatting
        let formatted = format_date_for_api("2025-06-01", false).unwrap();
        assert_eq!(formatted, "2025-06-01T00:00:00Z");

        // Test end of day formatting
        let formatted = format_date_for_api("2025-06-01", true).unwrap();
        assert_eq!(formatted, "2025-06-01T23:59:59Z");

        // Test invalid date format
        let result = format_date_for_api("invalid-date", false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid date format"));
    }

    #[tokio::test]
    async fn test_create_project_minimal() {
        let payload = json!({
            "name": "Minimal Project"
        });

        let response_body = json!({
            "id": "project-uuid-456",
            "name": "Minimal Project",
            "identifier": "min",
            "slug": "minimal-project",
            "url": "/api/v1/projects/project-uuid-456",
            "inserted_at": "2025-07-07T12:00:00Z",
            "updated_at": "2025-07-07T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/projects")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_project(&api_client, "Minimal Project", None, None)
            .await
            .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("project-uuid-456")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("Minimal Project")
        );
        assert_eq!(resp.get("identifier").and_then(Value::as_str), Some("min"));
    }

    #[tokio::test]
    async fn test_create_repo_full() {
        let payload = json!({
            "name": "My Repository",
            "project_id": "project-uuid-123",
            "local_path": "/path/to/repo",
            "remote_url": "https://github.com/user/repo.git",
            "default_branch": "main"
        });

        let response_body = json!({
            "id": "repo-uuid-123",
            "name": "My Repository",
            "project_id": "project-uuid-123",
            "local_path": "/path/to/repo",
            "remote_url": "https://github.com/user/repo.git",
            "default_branch": "main",
            "url": "/api/v1/repositories/repo-uuid-123",
            "inserted_at": "2025-07-09T12:00:00Z",
            "updated_at": "2025-07-09T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/repositories")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_repo(
            &api_client,
            "My Repository",
            "project-uuid-123",
            Some("/path/to/repo"),
            Some("https://github.com/user/repo.git"),
            Some("main"),
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("repo-uuid-123")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("My Repository")
        );
        assert_eq!(
            resp.get("project_id").and_then(Value::as_str),
            Some("project-uuid-123")
        );
        assert_eq!(
            resp.get("local_path").and_then(Value::as_str),
            Some("/path/to/repo")
        );
        assert_eq!(
            resp.get("remote_url").and_then(Value::as_str),
            Some("https://github.com/user/repo.git")
        );
        assert_eq!(
            resp.get("default_branch").and_then(Value::as_str),
            Some("main")
        );
    }

    #[tokio::test]
    async fn test_create_repo_minimal() {
        let payload = json!({
            "name": "Minimal Repo",
            "project_id": "project-uuid-456"
        });

        let response_body = json!({
            "id": "repo-uuid-456",
            "name": "Minimal Repo",
            "project_id": "project-uuid-456",
            "local_path": null,
            "remote_url": null,
            "default_branch": null,
            "url": "/api/v1/repositories/repo-uuid-456",
            "inserted_at": "2025-07-09T12:00:00Z",
            "updated_at": "2025-07-09T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/repositories")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_repo(
            &api_client,
            "Minimal Repo",
            "project-uuid-456",
            None,
            None,
            None,
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("repo-uuid-456")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("Minimal Repo")
        );
        assert_eq!(
            resp.get("project_id").and_then(Value::as_str),
            Some("project-uuid-456")
        );
    }

    #[tokio::test]
    async fn test_create_repo_local_only() {
        let payload = json!({
            "name": "Local Repository",
            "project_id": "project-uuid-789",
            "local_path": "/home/user/my-project"
        });

        let response_body = json!({
            "id": "repo-uuid-789",
            "name": "Local Repository",
            "project_id": "project-uuid-789",
            "local_path": "/home/user/my-project",
            "remote_url": null,
            "default_branch": null,
            "url": "/api/v1/repositories/repo-uuid-789",
            "inserted_at": "2025-07-09T12:00:00Z",
            "updated_at": "2025-07-09T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/repositories")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_repo(
            &api_client,
            "Local Repository",
            "project-uuid-789",
            Some("/home/user/my-project"),
            None,
            None,
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("repo-uuid-789")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("Local Repository")
        );
        assert_eq!(
            resp.get("local_path").and_then(Value::as_str),
            Some("/home/user/my-project")
        );
        assert_eq!(resp.get("remote_url"), Some(&serde_json::Value::Null));
    }

    #[tokio::test]
    async fn test_create_repo_remote_only() {
        let payload = json!({
            "name": "Remote Repository",
            "project_id": "project-uuid-101",
            "remote_url": "git@gitlab.com:group/project.git",
            "default_branch": "develop"
        });

        let response_body = json!({
            "id": "repo-uuid-101",
            "name": "Remote Repository",
            "project_id": "project-uuid-101",
            "local_path": null,
            "remote_url": "git@gitlab.com:group/project.git",
            "default_branch": "develop",
            "url": "/api/v1/repositories/repo-uuid-101",
            "inserted_at": "2025-07-09T12:00:00Z",
            "updated_at": "2025-07-09T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/repositories")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(201)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = create_repo(
            &api_client,
            "Remote Repository",
            "project-uuid-101",
            None,
            Some("git@gitlab.com:group/project.git"),
            Some("develop"),
        )
        .await
        .expect("Expected Ok");

        assert_eq!(
            resp.get("id").and_then(Value::as_str),
            Some("repo-uuid-101")
        );
        assert_eq!(
            resp.get("name").and_then(Value::as_str),
            Some("Remote Repository")
        );
        assert_eq!(
            resp.get("remote_url").and_then(Value::as_str),
            Some("git@gitlab.com:group/project.git")
        );
        assert_eq!(
            resp.get("default_branch").and_then(Value::as_str),
            Some("develop")
        );
        assert_eq!(resp.get("local_path"), Some(&serde_json::Value::Null));
    }

    #[tokio::test]
    async fn test_create_repo_validation_error() {
        let payload = json!({
            "name": "Invalid Repo",
            "project_id": "nonexistent-project"
        });

        let error_response = json!({
            "error": "Validation failed",
            "details": {
                "project_id": ["does not exist"]
            }
        });

        let _m = mock("POST", "/api/v1/repositories")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(422)
            .with_body(error_response.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let result = create_repo(
            &api_client,
            "Invalid Repo",
            "nonexistent-project",
            None,
            None,
            None,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_token_info_active() {
        let payload = json!({
            "token": "test-access-token"
        });

        let response_body = json!({
            "active": true,
            "client_id": "cli-client",
            "username": "testuser",
            "scope": "user:read user:write project:read project:write worklog:read worklog:write repo:read repo:write",
            "exp": 1672531200
        });

        let _m = mock("POST", "/auth/token_info")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(200)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = check_token_info(&api_client, "test-access-token")
            .await
            .expect("Expected Ok");

        assert!(resp.active);
        assert_eq!(resp.client_id, "cli-client");
        assert_eq!(resp.username, Some("testuser".to_string()));
        assert_eq!(resp.scope, "user:read user:write project:read project:write worklog:read worklog:write repo:read repo:write");
    }

    #[tokio::test]
    async fn test_check_token_info_inactive() {
        let payload = json!({
            "token": "expired-token"
        });

        let response_body = json!({
            "active": false
        });

        let _m = mock("POST", "/auth/token_info")
            .match_header("authorization", Matcher::Any)
            .match_body(Matcher::Json(payload))
            .with_status(200)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let result = check_token_info(&api_client, "expired-token").await;

        // The function should return an error for inactive tokens
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_worklog_entries_basic() {
        let response_body = json!({
            "entries": [
                {
                    "id": "entry-uuid-123",
                    "content": "Working on feature X",
                    "recorded_at": "2025-07-09T14:30:00Z",
                    "tags": ["development", "feature"],
                    "project_id": "project-uuid-123",
                    "url": "/api/v1/worklog/entries/entry-uuid-123"
                },
                {
                    "id": "entry-uuid-456",
                    "content": "Code review session",
                    "recorded_at": "2025-07-09T15:45:00Z",
                    "tags": ["review"],
                    "project_id": "project-uuid-123",
                    "url": "/api/v1/worklog/entries/entry-uuid-456"
                }
            ],
            "meta": {
                "result_count": 2,
                "total_count": 5,
                "start_cursor": "entry-uuid-123",
                "end_cursor": "entry-uuid-456",
                "limit": 20
            }
        });

        let _m = mock("GET", "/api/v1/worklog/entries?limit=20")
            .match_header("authorization", Matcher::Any)
            .with_status(200)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = fetch_worklog_entries(&api_client, None, None, None, None, 20, None)
            .await
            .expect("Expected Ok");

        let entries = resp.get("entries").expect("Expected entries array");
        assert!(entries.is_array());
        assert_eq!(entries.as_array().unwrap().len(), 2);

        let first_entry = &entries.as_array().unwrap()[0];
        assert_eq!(first_entry["id"], "entry-uuid-123");
        assert_eq!(first_entry["content"], "Working on feature X");
    }

    #[tokio::test]
    async fn test_fetch_worklog_entries_with_filters() {
        let response_body = json!({
            "entries": [
                {
                    "id": "entry-uuid-789",
                    "content": "Development work",
                    "recorded_at": "2025-07-09T10:00:00Z",
                    "tags": ["development"],
                    "project_id": "specific-project",
                    "url": "/api/v1/worklog/entries/entry-uuid-789"
                }
            ],
            "meta": {
                "result_count": 1,
                "total_count": 1,
                "start_cursor": "entry-uuid-789",
                "end_cursor": "entry-uuid-789",
                "limit": 10
            }
        });

        let expected_params = "limit=10&project_id=specific-project&tags=development,feature&from=2025-07-01T00:00:00Z&to=2025-07-09T23:59:59Z&starting_after=cursor-123";

        let _m = mock(
            "GET",
            format!("/api/v1/worklog/entries?{expected_params}").as_str(),
        )
        .match_header("authorization", Matcher::Any)
        .with_status(200)
        .with_body(response_body.to_string())
        .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let tags = vec!["development".to_string(), "feature".to_string()];
        let resp = fetch_worklog_entries(
            &api_client,
            Some("specific-project"),
            Some(&tags),
            Some("2025-07-01"),
            Some("2025-07-09"),
            10,
            Some("cursor-123"),
        )
        .await
        .expect("Expected Ok");

        let entries = resp.get("entries").expect("Expected entries array");
        assert!(entries.is_array());
        assert_eq!(entries.as_array().unwrap().len(), 1);

        let entry = &entries.as_array().unwrap()[0];
        assert_eq!(entry["id"], "entry-uuid-789");
        assert_eq!(entry["project_id"], "specific-project");
    }

    #[tokio::test]
    async fn test_fetch_worklog_entries_empty() {
        let response_body = json!({
            "entries": [],
            "meta": {
                "result_count": 0,
                "total_count": 0,
                "start_cursor": null,
                "end_cursor": null,
                "limit": 20
            }
        });

        let _m = mock("GET", "/api/v1/worklog/entries?limit=20")
            .match_header("authorization", Matcher::Any)
            .with_status(200)
            .with_body(response_body.to_string())
            .create();

        let mut api_client = ApiClient::new(&mockito::server_url());
        api_client.set_access_token("dummy-token".into());

        let resp = fetch_worklog_entries(&api_client, None, None, None, None, 20, None)
            .await
            .expect("Expected Ok");

        let entries = resp.get("entries").expect("Expected entries array");
        assert!(entries.is_array());
        assert_eq!(entries.as_array().unwrap().len(), 0);

        let meta = resp.get("meta").expect("Expected meta object");
        assert_eq!(meta["result_count"], 0);
        assert_eq!(meta["total_count"], 0);
    }
}
