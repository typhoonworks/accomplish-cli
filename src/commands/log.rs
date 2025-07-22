// src/commands/log.rs
use crate::api::endpoints::create_worklog_entry;
use crate::auth::AuthService;
use crate::commands::project;
use crate::errors::AppError;
use chrono::Utc;
use regex::Regex;
use serde_json::to_string_pretty;

/// Converts bare URLs in text to markdown links.
/// URLs that are already in markdown link format are left unchanged.
fn convert_urls_to_markdown(text: &str) -> String {
    // Simple approach: find URLs that aren't already in markdown links
    let url_regex = Regex::new(r"https?://[^\s\]]+").unwrap();

    url_regex
        .replace_all(text, |caps: &regex::Captures| {
            let url = caps.get(0).unwrap().as_str();
            let start = caps.get(0).unwrap().start();

            // Check if this URL is already part of a markdown link
            // Look for "](" before the URL
            let text_before_url = &text[..start];
            if text_before_url.ends_with("](") {
                // This URL is already in a markdown link, don't convert
                url.to_string()
            } else {
                // Convert to markdown link
                format!("[{url}]({url})")
            }
        })
        .to_string()
}

/// Adds a new worklog entry with the given messages, optional tags, and optional project identifier.
/// Requires an authenticated AuthService.
pub async fn execute(
    auth_service: &mut AuthService,
    messages: &[String],
    tags: &[String],
    project_identifier: Option<&str>,
) -> Result<String, AppError> {
    let recorded_at = Utc::now().to_rfc3339();
    let content = convert_urls_to_markdown(&messages.join("\n\n"));

    let (project_id, project_info) = if let Some(identifier) = project_identifier {
        let projects = project::get_projects(auth_service).await?;

        let mut project_id = None;
        let mut project_info = None;

        for p in &projects {
            if p.identifier.to_lowercase() == identifier.to_lowercase() {
                project_id = Some(p.id.clone());
                project_info = Some((p.name.clone(), p.identifier.to_uppercase()));
                break;
            }
        }

        if project_id.is_none() {
            println!("⚠️ Warning: No project found with identifier '{identifier}'");
        }

        (project_id, project_info)
    } else {
        (None, None)
    };

    let resp = create_worklog_entry(
        auth_service.api_client(),
        &content,
        &recorded_at,
        tags,
        project_id.as_deref(),
    )
    .await
    .map_err(AppError::Api)?;

    if let Some(id) = resp.get("id").and_then(|v| v.as_str()) {
        println!("✅ Created entry with id {id}");
        if !tags.is_empty() {
            println!("Tags: {}", tags.join(", "));
        }
        if let Some(identifier) = project_identifier {
            if let Some((name, uppercase_identifier)) = project_info {
                println!("Project: {name} ({uppercase_identifier})");
            } else {
                println!("Project: {}", identifier.to_uppercase());
            }
        }
        Ok(id.to_string())
    } else {
        println!("{}", to_string_pretty(&resp)?);
        Err(AppError::ParseError(
            "Failed to get entry ID from response".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};
    use serde_json::json;

    fn setup_mock_auth_service(server_url: &str) -> AuthService {
        let mut auth =
            AuthService::new(server_url.to_string(), std::env::temp_dir(), "test-profile");
        auth.save_access_token("test-token").unwrap();
        auth
    }

    #[tokio::test]
    async fn test_execute_success() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());

        let response = json!({
            "id": "id-123",
            "content": "Test message",
            "recorded_at": "2025-05-17T12:00:00Z"
        });

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({ "content": "Test message" })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let result = execute(&mut auth, &["Test message".into()], &[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_multiple_messages() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());
        let messages = vec!["Line 1".into(), "Line 2".into()];
        let joined = "Line 1\n\nLine 2";

        let response = json!({
            "id": "id-456",
            "content": joined,
            "recorded_at": "2025-05-17T12:00:00Z"
        });

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({ "content": joined })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let result = execute(&mut auth, &messages, &[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_tags() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());
        let tags = vec!["rust".into(), "cli".into()];
        let response = json!({
            "id": "id-789",
            "content": "Message with tags",
            "recorded_at": "2025-05-17T12:00:00Z",
            "tags": tags
        });

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "content": "Message with tags",
                "tags": tags
            })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let result = execute(&mut auth, &["Message with tags".into()], &tags, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_api_error() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({ "content": "Err message" })))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"bad_request"}"#)
            .create();

        let result = execute(&mut auth, &["Err message".into()], &[], None).await;
        assert!(matches!(result, Err(AppError::Api(_))));
    }

    #[tokio::test]
    async fn test_execute_with_multiline_content() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());
        // Note: In this test we're directly passing a single string with embedded newlines
        // rather than multiple message strings that get joined
        let content = "This is a multiline entry\n\nwith multiple paragraphs\n\nAnd some spacing.";

        let response = json!({
            "id": "id-multiline",
            "content": content,
            "recorded_at": "2025-05-17T12:00:00Z"
        });

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({ "content": content })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        // Test with a single message containing newlines
        let result = execute(&mut auth, &[content.to_string()], &[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_project() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());
        let project_id = "website";
        let project_identifier = "web";

        let projects_response = json!({
            "projects": [
                {
                    "id": project_id,
                    "name": "Website Project",
                    "identifier": project_identifier
                }
            ]
        });

        let _projects_mock = server
            .mock("GET", "/api/v1/projects")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(projects_response.to_string())
            .create();

        let entry_response = json!({
            "id": "id-project",
            "content": "Entry with project",
            "recorded_at": "2025-05-17T12:00:00Z",
            "project_id": project_id,
            "project_url": "/api/v1/projects/website"
        });

        let _entry_mock = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({
                "content": "Entry with project",
                "project_id": project_id
            })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(entry_response.to_string())
            .create();

        let result = execute(
            &mut auth,
            &["Entry with project".into()],
            &[],
            Some(project_identifier),
        )
        .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_urls_to_markdown_basic_url() {
        let input = "Check out https://example.com for more info";
        let expected = "Check out [https://example.com](https://example.com) for more info";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_multiple_urls() {
        let input = "Visit https://example.com and https://test.org";
        let expected = "Visit [https://example.com](https://example.com) and [https://test.org](https://test.org)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_url_at_beginning() {
        let input = "https://example.com is a good site";
        let expected = "[https://example.com](https://example.com) is a good site";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_url_at_end() {
        let input = "Check this out: https://example.com";
        let expected = "Check this out: [https://example.com](https://example.com)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_already_markdown_link() {
        let input = "This is [already a link](https://example.com) and should not change";
        let expected = "This is [already a link](https://example.com) and should not change";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_mixed_content() {
        let input = "Check [this link](https://example.com) and also https://test.org";
        let expected =
            "Check [this link](https://example.com) and also [https://test.org](https://test.org)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_gitlab_issue_url() {
        let input = "Planning approach for https://gitlab.silverfin.com/development/silverfin/-/issues/26766";
        let expected = "Planning approach for [https://gitlab.silverfin.com/development/silverfin/-/issues/26766](https://gitlab.silverfin.com/development/silverfin/-/issues/26766)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_http_url() {
        let input = "Visit http://example.com for more";
        let expected = "Visit [http://example.com](http://example.com) for more";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_no_urls() {
        let input = "This text has no URLs in it";
        let expected = "This text has no URLs in it";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_multiline() {
        let input = "Line 1 with https://example.com\n\nLine 2 with https://test.org";
        let expected = "Line 1 with [https://example.com](https://example.com)\n\nLine 2 with [https://test.org](https://test.org)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_url_with_query_params() {
        let input = "Search at https://example.com/search?q=rust&type=code";
        let expected = "Search at [https://example.com/search?q=rust&type=code](https://example.com/search?q=rust&type=code)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[test]
    fn test_convert_urls_to_markdown_url_with_fragment() {
        let input = "Go to https://example.com/docs#section1";
        let expected =
            "Go to [https://example.com/docs#section1](https://example.com/docs#section1)";
        assert_eq!(convert_urls_to_markdown(input), expected);
    }

    #[tokio::test]
    async fn test_execute_with_url_conversion() {
        let mut server = Server::new_async().await;
        let mut auth = setup_mock_auth_service(&server.url());
        let messages = vec!["Planning approach for https://gitlab.silverfin.com/development/silverfin/-/issues/26766".into()];
        let expected_content = "Planning approach for [https://gitlab.silverfin.com/development/silverfin/-/issues/26766](https://gitlab.silverfin.com/development/silverfin/-/issues/26766)";

        let response = json!({
            "id": "id-url-test",
            "content": expected_content,
            "recorded_at": "2025-05-17T12:00:00Z"
        });

        let _m = server
            .mock("POST", "/api/v1/worklog/entries")
            .match_header("authorization", "Bearer test-token")
            .match_body(Matcher::PartialJson(json!({ "content": expected_content })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let result = execute(&mut auth, &messages, &[], None).await;
        assert!(result.is_ok());
    }
}
