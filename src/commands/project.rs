use crate::api::endpoints;
use crate::auth::AuthService;
use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use tabled::settings::Style;
use tabled::{Table, Tabled};

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub identifier: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProjectsResponse {
    projects: Vec<Project>,
}

/// Lists all projects for the authenticated user.
/// Requires an authenticated AuthService.
pub async fn list(auth_service: &mut AuthService) -> Result<(), AppError> {
    let projects = get_projects(auth_service).await?;

    if projects.is_empty() {
        println!("No projects found.");
        return Ok(());
    }

    let table_data: Vec<ProjectTableRow> = projects
        .into_iter()
        .map(|project| ProjectTableRow {
            name: project.name,
            identifier: project.identifier.to_uppercase(),
        })
        .collect();

    let table = Table::new(table_data).with(Style::modern()).to_string();

    println!("{table}");
    Ok(())
}

#[derive(Tabled)]
struct ProjectTableRow {
    #[tabled(rename = "Identifier")]
    identifier: String,
    #[tabled(rename = "Name")]
    name: String,
}

/// Gets projects from the API and parses the response.
pub async fn get_projects(auth_service: &mut AuthService) -> Result<Vec<Project>, AppError> {
    let response = endpoints::fetch_projects(auth_service.api_client())
        .await
        .map_err(AppError::Api)?;

    let projects_response: ProjectsResponse = serde_json::from_value(response)
        .map_err(|e| AppError::ParseError(format!("Failed to parse projects response: {e}")))?;

    Ok(projects_response.projects)
}

/// Creates a new project with the given name, description, and identifier.
/// If identifier is None, the backend will auto-generate one.
/// Requires an authenticated AuthService.
pub async fn create_project(
    auth_service: &mut AuthService,
    name: &str,
    description: Option<&str>,
    identifier: Option<&str>,
) -> Result<(), AppError> {
    // Validate project name
    if name.trim().is_empty() {
        return Err(AppError::ParseError(
            "Project name cannot be empty".to_string(),
        ));
    }

    // Validate identifier if provided
    if let Some(id) = identifier {
        if id.trim().is_empty() {
            return Err(AppError::ParseError(
                "Identifier cannot be empty".to_string(),
            ));
        }
        if id.trim().len() > 3 {
            return Err(AppError::ParseError(
                "Identifier must be 3 characters or less".to_string(),
            ));
        }
        if !id.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(AppError::ParseError(
                "Identifier must contain only letters".to_string(),
            ));
        }
    }

    let response =
        endpoints::create_project(auth_service.api_client(), name, description, identifier)
            .await
            .map_err(AppError::Api)?;

    // Extract project details from response
    let project_name = response
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let project_id = response
        .get("identifier")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    println!(
        "✓ Project '{project_name}' created successfully with identifier '{project_id}'"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};
    use serde_json::json;

    fn setup_mock_auth_service() -> AuthService {
        let mut auth = AuthService::new(server_url(), std::env::temp_dir(), "test-profile");
        auth.save_access_token("test-token").unwrap();
        auth
    }

    #[tokio::test]
    async fn test_get_projects_success() {
        let mut auth = setup_mock_auth_service();

        let response = json!({
            "projects": [
                {
                    "id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
                    "name": "website",
                    "identifier": "web"
                },
                {
                    "id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
                    "name": "internal-ops",
                    "identifier": "ops"
                }
            ]
        });

        let _m = mock("GET", "/api/v1/projects")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let projects = get_projects(&mut auth).await;
        assert!(projects.is_ok());

        let projects = projects.unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].id, "3fa85f64-5717-4562-b3fc-2c963f66afa6");
        assert_eq!(projects[0].name, "website");
        assert_eq!(projects[0].identifier, "web");
        assert_eq!(projects[1].id, "7c9e6679-7425-40de-944b-e07fc1f90ae7");
        assert_eq!(projects[1].name, "internal-ops");
        assert_eq!(projects[1].identifier, "ops");
    }

    #[tokio::test]
    async fn test_get_projects_empty() {
        let mut auth = setup_mock_auth_service();

        let response = json!({
            "projects": []
        });

        let _m = mock("GET", "/api/v1/projects")
            .match_header("authorization", "Bearer test-token")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response.to_string())
            .create();

        let projects = get_projects(&mut auth).await;
        assert!(projects.is_ok());
        assert_eq!(projects.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_get_projects_unauthorized() {
        let mut auth = setup_mock_auth_service();

        let _m = mock("GET", "/api/v1/projects")
            .match_header("authorization", "Bearer test-token")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":"unauthorized"}"#)
            .create();

        let result = get_projects(&mut auth).await;
        assert!(matches!(result, Err(AppError::Api(_))));
    }

    #[tokio::test]
    async fn test_create_project_success() {
        let mut auth = setup_mock_auth_service();

        let response = json!({
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
            .match_header("authorization", "Bearer test-token")
            .with_status(201)
            .with_body(response.to_string())
            .create();

        let result = create_project(
            &mut auth,
            "Test Project",
            Some("A test project"),
            Some("tst"),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_project_minimal() {
        let mut auth = setup_mock_auth_service();

        let response = json!({
            "id": "project-uuid-456",
            "name": "Minimal Project",
            "identifier": "min",
            "slug": "minimal-project",
            "url": "/api/v1/projects/project-uuid-456",
            "inserted_at": "2025-07-07T12:00:00Z",
            "updated_at": "2025-07-07T12:00:00Z"
        });

        let _m = mock("POST", "/api/v1/projects")
            .match_header("authorization", "Bearer test-token")
            .with_status(201)
            .with_body(response.to_string())
            .create();

        let result = create_project(&mut auth, "Minimal Project", None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_project_validation_errors() {
        let mut auth = setup_mock_auth_service();

        // Test empty name
        let result = create_project(&mut auth, "", None, None).await;
        assert!(matches!(result, Err(AppError::ParseError(_))));

        // Test empty identifier
        let result = create_project(&mut auth, "Test", None, Some("")).await;
        assert!(matches!(result, Err(AppError::ParseError(_))));

        // Test identifier too long
        let result = create_project(&mut auth, "Test", None, Some("toolong")).await;
        assert!(matches!(result, Err(AppError::ParseError(_))));

        // Test identifier with non-letters
        let result = create_project(&mut auth, "Test", None, Some("t3t")).await;
        assert!(matches!(result, Err(AppError::ParseError(_))));
    }
}
