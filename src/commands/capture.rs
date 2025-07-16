use crate::api::endpoints::{
    associate_commits_with_entry, create_commits, fetch_projects, fetch_uncaptured_commits,
    CommitData,
};
use crate::auth::AuthService;
use crate::commands::log;
use crate::config;
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use git2::{Commit, Repository};
use inquire::{Confirm, MultiSelect};
use std::env;
use std::path::Path;

/// Represents a git commit with its metadata
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub sha: String,
    pub message: String,
    pub committed_at: DateTime<Utc>,
    pub short_sha: String,
    pub summary: String,
}

impl GitCommit {
    /// Creates a new GitCommit from a git2::Commit
    pub fn from_git2_commit(commit: &Commit) -> Result<Self, AppError> {
        let sha = commit.id().to_string();
        let short_sha = sha.chars().take(7).collect();
        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();

        let timestamp = commit.time().seconds();
        let committed_at = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| AppError::ParseError("Invalid commit timestamp".to_string()))?;

        Ok(GitCommit {
            sha,
            message,
            committed_at,
            short_sha,
            summary,
        })
    }
}

/// Executes the capture command
pub async fn execute(
    auth_service: &mut AuthService,
    limit: u32,
    edit: bool,
) -> Result<(), AppError> {
    // Check if current directory is a git repository
    let current_dir = env::current_dir()
        .map_err(|e| AppError::ParseError(format!("Failed to get current directory: {e}")))?;

    if !is_git_repository(&current_dir) {
        return Err(AppError::Other(
            "This command must be run in a git repository".to_string(),
        ));
    }

    // Check if directory is initialized (has a project configured)
    let project_identifier =
        config::lookup_default_project_for_dir(&current_dir).ok_or_else(|| {
            AppError::ParseError("Directory not initialized. Run 'acc init' first".to_string())
        })?;

    // Get the repository from the backend
    let repo_id =
        get_repository_id_for_project(auth_service, &project_identifier, &current_dir).await?;

    // Get recent commits from git
    let commits = get_recent_commits(&current_dir, limit)?;

    if commits.is_empty() {
        println!("No commits found in the repository.");
        return Ok(());
    }

    // Get uncaptured commits from the backend
    let commit_shas: Vec<String> = commits.iter().map(|c| c.sha.clone()).collect();
    let uncaptured_shas = get_uncaptured_commits(auth_service, &repo_id, &commit_shas).await?;

    if uncaptured_shas.is_empty() {
        println!("No new commits to capture.");
        return Ok(());
    }

    // Filter commits to only show uncaptured ones
    let uncaptured_commits: Vec<GitCommit> = commits
        .into_iter()
        .filter(|c| uncaptured_shas.contains(&c.sha))
        .collect();

    // Present interactive selection
    let options: Vec<String> = uncaptured_commits
        .iter()
        .map(|c| format!("{} {}", c.short_sha, c.summary))
        .collect();

    let selected_options = MultiSelect::new("Select commits to capture:", options.clone())
        .with_help_message("Use space to select, arrow keys to navigate, enter to confirm")
        .prompt()
        .map_err(|e| AppError::ParseError(format!("Selection failed: {e}")))?;

    if selected_options.is_empty() {
        println!("No commits selected.");
        return Ok(());
    }

    // Get the selected commits
    let selected_commits: Vec<&GitCommit> = selected_options
        .iter()
        .map(|selected_option| {
            // Find the index of the selected option in the uncaptured_commits
            let index = options
                .iter()
                .position(|opt| opt == selected_option)
                .unwrap();
            &uncaptured_commits[index]
        })
        .collect();

    // Create commits in the backend
    let commit_data: Vec<CommitData> = selected_commits
        .iter()
        .map(|c| CommitData {
            sha: c.sha.clone(),
            message: Some(c.message.clone()),
            committed_at: Some(c.committed_at.to_rfc3339()),
        })
        .collect();

    let created_commits = capture_commits(auth_service, &repo_id, &commit_data).await?;

    println!("✅ Captured {} commits", selected_commits.len());

    // Ask if user wants to create a worklog entry
    let create_worklog = Confirm::new("Create worklog entry from selected commits?")
        .with_default(true)
        .prompt()
        .map_err(|e| AppError::ParseError(format!("Confirmation failed: {e}")))?;

    if create_worklog {
        // Extract commit IDs from the API response
        let commit_ids: Vec<String> = created_commits
            .get("commits")
            .and_then(|commits| commits.as_array())
            .map(|commits| {
                commits
                    .iter()
                    .filter_map(|commit| commit.get("id").and_then(|id| id.as_str()))
                    .map(|id| id.to_string())
                    .collect()
            })
            .unwrap_or_default();

        create_worklog_entry_from_commits(
            auth_service,
            &selected_commits,
            &commit_ids,
            &project_identifier,
            edit,
        )
        .await?;
    }

    Ok(())
}

/// Checks if the given directory is a git repository
fn is_git_repository(dir: &Path) -> bool {
    Repository::open(dir).is_ok()
}

/// Gets recent commits from the git repository
fn get_recent_commits(dir: &Path, limit: u32) -> Result<Vec<GitCommit>, AppError> {
    let repo = Repository::open(dir)
        .map_err(|e| AppError::ParseError(format!("Failed to open git repository: {e}")))?;

    let mut revwalk = repo
        .revwalk()
        .map_err(|e| AppError::ParseError(format!("Failed to create revision walker: {e}")))?;

    revwalk
        .push_head()
        .map_err(|e| AppError::ParseError(format!("Failed to push HEAD: {e}")))?;

    let mut commits = Vec::new();

    for (count, oid) in revwalk.enumerate() {
        if count >= limit as usize {
            break;
        }

        let oid =
            oid.map_err(|e| AppError::ParseError(format!("Failed to get commit OID: {e}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| AppError::ParseError(format!("Failed to find commit: {e}")))?;

        commits.push(GitCommit::from_git2_commit(&commit)?);
    }

    Ok(commits)
}

/// Gets the repository ID for the given project from the backend
async fn get_repository_id_for_project(
    auth_service: &mut AuthService,
    project_identifier: &str,
    current_dir: &Path,
) -> Result<String, AppError> {
    // Get all projects to find the one with the given identifier
    let projects_response = fetch_projects(auth_service.api_client())
        .await
        .map_err(AppError::Api)?;

    let projects = projects_response
        .get("projects")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::ParseError("Invalid projects response format".to_string()))?;

    // Find the project with the matching identifier
    let target_project = projects
        .iter()
        .find(|p| {
            p.get("identifier")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
                == Some(project_identifier.to_lowercase())
        })
        .ok_or_else(|| AppError::ParseError(format!("Project '{project_identifier}' not found")))?;

    let project_id = target_project
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::ParseError("Project ID not found".to_string()))?;

    // Get repositories for this project
    let repos_response = crate::api::endpoints::fetch_repositories(auth_service.api_client())
        .await
        .map_err(AppError::Api)?;

    let repositories = repos_response
        .get("repositories")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::ParseError("Invalid repositories response format".to_string()))?;

    // Filter repositories for this project
    let project_repos: Vec<_> = repositories
        .iter()
        .filter(|repo| repo.get("project_id").and_then(|v| v.as_str()) == Some(project_id))
        .collect();

    if project_repos.is_empty() {
        return Err(AppError::ParseError(format!(
            "No repositories found for project '{project_identifier}'"
        )));
    }

    // Get current directory path as string for matching
    let current_path = current_dir.to_string_lossy().to_string();

    // Get current git remote URL for matching
    let current_remote = get_git_remote_url(current_dir);

    // Try to match by local_path first
    if let Some(repo) = project_repos.iter().find(|repo| {
        repo.get("local_path")
            .and_then(|v| v.as_str())
            .map(|path| path == current_path)
            .unwrap_or(false)
    }) {
        return repo
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::ParseError("Repository ID not found".to_string()));
    }

    // Try to match by remote_url if local_path didn't match
    if let Some(ref remote_url) = current_remote {
        if let Some(repo) = project_repos.iter().find(|repo| {
            repo.get("remote_url")
                .and_then(|v| v.as_str())
                .map(|url| normalize_git_url(url) == normalize_git_url(remote_url))
                .unwrap_or(false)
        }) {
            return repo
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| AppError::ParseError("Repository ID not found".to_string()));
        }
    }

    // If no exact match found, return error with helpful message
    Err(AppError::ParseError(format!(
        "No repository found for project '{}' matching current directory '{}' or remote URL '{}'",
        project_identifier,
        current_path,
        current_remote.unwrap_or_else(|| "none".to_string())
    )))
}

/// Gets the git remote URL for the current repository
fn get_git_remote_url(dir: &Path) -> Option<String> {
    let repo = Repository::open(dir).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    remote.url().map(|s| s.to_string())
}

/// Normalizes git URLs for comparison (handles differences like .git suffix, SSH vs HTTPS)
fn normalize_git_url(url: &str) -> String {
    let mut normalized = url.to_string();

    // Remove .git suffix if present
    if normalized.ends_with(".git") {
        normalized = normalized[..normalized.len() - 4].to_string();
    }

    // Convert SSH URLs to HTTPS-like format for comparison
    if normalized.starts_with("git@") {
        // Convert git@github.com:user/repo to github.com/user/repo
        normalized = normalized.replace("git@", "").replace(":", "/");
    }

    // Remove protocol prefixes for comparison
    if let Some(pos) = normalized.find("://") {
        normalized = normalized[pos + 3..].to_string();
    }

    normalized.to_lowercase()
}

/// Gets uncaptured commits from the backend API
async fn get_uncaptured_commits(
    auth_service: &mut AuthService,
    repo_id: &str,
    commit_shas: &[String],
) -> Result<Vec<String>, AppError> {
    let response = fetch_uncaptured_commits(auth_service.api_client(), repo_id, commit_shas)
        .await
        .map_err(AppError::Api)?;

    let uncaptured_shas = response
        .get("uncaptured_shas")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::ParseError("Invalid response format".to_string()))?;

    let shas: Vec<String> = uncaptured_shas
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    Ok(shas)
}

/// Captures the selected commits to the backend
async fn capture_commits(
    auth_service: &mut AuthService,
    repo_id: &str,
    commit_data: &[CommitData],
) -> Result<serde_json::Value, AppError> {
    let response = create_commits(auth_service.api_client(), repo_id, commit_data)
        .await
        .map_err(AppError::Api)?;

    Ok(response)
}

/// Creates a worklog entry from the selected commits
async fn create_worklog_entry_from_commits(
    auth_service: &mut AuthService,
    commits: &[&GitCommit],
    commit_ids: &[String],
    project_identifier: &str,
    edit: bool,
) -> Result<(), AppError> {
    // Create content from commit messages
    let messages: Vec<String> = if edit {
        // Pre-fill the editor with commit messages
        let prefilled_content = commits
            .iter()
            .map(|c| c.message.trim())
            .collect::<Vec<&str>>()
            .join("\n\n");

        // Create template with commit messages
        let template = format!(
            "# Enter your worklog entry below\n\
             # Lines starting with # will be ignored\n\
             # Pre-filled with commit messages from selected commits:\n\
             #\n\
             {prefilled_content}\n"
        );

        match crate::utils::editor::open_in_editor(Some(&template)) {
            Ok(content) => {
                if content.is_empty() {
                    return Err(AppError::Other(
                        "No content provided. Aborting.".to_string(),
                    ));
                }
                vec![content]
            }
            Err(e) => {
                return Err(AppError::Other(format!("Editor error: {e}")));
            }
        }
    } else {
        commits
            .iter()
            .map(|c| c.message.trim().to_string())
            .collect()
    };

    // Create the worklog entry first
    let entry_id = log::execute(auth_service, &messages, &[], Some(project_identifier)).await?;

    // Associate the commits with the worklog entry
    if !commit_ids.is_empty() {
        associate_commits_with_entry(auth_service.api_client(), &entry_id, commit_ids)
            .await
            .map_err(AppError::Api)?;

        println!(
            "🔗 Associated {} commits with worklog entry",
            commit_ids.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_git_repository_true() {
        let temp_dir = TempDir::new().unwrap();

        // Initialize a real git repository
        Repository::init(temp_dir.path()).unwrap();

        assert!(is_git_repository(temp_dir.path()));
    }

    #[test]
    fn test_is_git_repository_false() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_git_repository(temp_dir.path()));
    }

    #[test]
    fn test_normalize_git_url() {
        // Test .git suffix removal
        assert_eq!(
            normalize_git_url("https://github.com/user/repo.git"),
            "github.com/user/repo"
        );

        // Test SSH to HTTPS conversion
        assert_eq!(
            normalize_git_url("git@github.com:user/repo.git"),
            "github.com/user/repo"
        );

        // Test protocol removal
        assert_eq!(
            normalize_git_url("https://gitlab.com/user/repo"),
            "gitlab.com/user/repo"
        );

        // Test case insensitive
        assert_eq!(
            normalize_git_url("HTTPS://GitHub.com/User/Repo"),
            "github.com/user/repo"
        );
    }
}
