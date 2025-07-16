use crate::api::endpoints;
use crate::auth::AuthService;
use crate::commands::project::{get_projects, Project};
use crate::errors::AppError;
use dirs_next::home_dir;
use inquire::{Confirm, Select, Text};
use std::fs;
use std::path::Path;

pub async fn execute(auth_service: &mut AuthService) -> Result<(), AppError> {
    let current_dir = std::env::current_dir()
        .map_err(|e| AppError::ParseError(format!("Failed to get current directory: {}", e)))?;

    // Check if directory is already initialized locally
    let accomplish_config_path = current_dir.join(".accomplish.toml");
    let has_local_config = accomplish_config_path.exists();

    // Check if directory is already tracked globally
    let is_tracked_globally = is_globally_tracked(&current_dir)?;

    if has_local_config || is_tracked_globally {
        let config_type = if has_local_config { "local" } else { "global" };
        println!(
            "Directory is already initialized with a project ({} config).",
            config_type
        );

        let proceed = Confirm::new("Do you want to reinitialize this directory?")
            .with_help_message("This will replace the existing configuration")
            .with_default(false)
            .prompt()
            .map_err(|e| AppError::ParseError(format!("Confirmation failed: {}", e)))?;

        if !proceed {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Detect if it's a git repository
    let is_git_repo = current_dir.join(".git").exists();
    let repo_type = if is_git_repo {
        "git repository"
    } else {
        "folder"
    };

    println!("Initializing {} in: {}", repo_type, current_dir.display());

    // Fetch available projects
    let projects = get_projects(auth_service).await?;

    if projects.is_empty() {
        println!("No projects found. Please create a project first using 'acc project new'.");
        return Ok(());
    }

    // Create selection options
    let mut options: Vec<String> = projects
        .iter()
        .map(|p| format!("{} - {}", p.identifier.to_uppercase(), p.name))
        .collect();
    options.push("Cancel".to_string());

    // Interactive selection
    let selected = Select::new(
        "Select a project to associate with this directory:",
        options,
    )
    .with_help_message("Use arrow keys to navigate, Enter to select")
    .prompt()
    .map_err(|e| AppError::ParseError(format!("Selection failed: {}", e)))?;

    // Handle cancellation
    if selected == "Cancel" {
        println!("Operation cancelled.");
        return Ok(());
    }

    // Find the selected project
    let selected_project = projects
        .iter()
        .find(|p| selected.starts_with(&p.identifier.to_uppercase()))
        .ok_or_else(|| AppError::ParseError("Selected project not found".to_string()))?;

    // Create repository if it's a git repo
    if is_git_repo {
        let git_remote = get_git_remote(&current_dir);
        let default_branch = get_default_branch(&current_dir);

        // Check if a repository with the same remote URL already exists
        let mut existing_repo = None;
        if let Some(ref remote_url) = git_remote {
            match endpoints::fetch_repositories(auth_service.api_client()).await {
                Ok(response) => {
                    if let Some(repositories) =
                        response.get("repositories").and_then(|v| v.as_array())
                    {
                        existing_repo = repositories
                            .iter()
                            .find(|repo| {
                                // Filter by project_id and remote_url
                                let same_project = repo
                                    .get("project_id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id == selected_project.id)
                                    .unwrap_or(false);
                                let same_remote = repo
                                    .get("remote_url")
                                    .and_then(|v| v.as_str())
                                    .map(|url| url == remote_url)
                                    .unwrap_or(false);
                                same_project && same_remote
                            })
                            .cloned();
                    }
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Warning: Could not check for existing repositories: {}",
                        e
                    );
                }
            }
        }

        if let Some(repo) = existing_repo {
            // Repository already exists
            println!("✓ Repository already exists in project");
            if let Some(repo_name) = repo.get("name").and_then(|v| v.as_str()) {
                println!("  Repository name: {}", repo_name);
            }
            if let Some(repo_id) = repo.get("id").and_then(|v| v.as_str()) {
                println!("  Repository ID: {}", repo_id);
            }
        } else {
            // Create new repository
            let default_repo_name = derive_repo_name(&current_dir, git_remote.as_deref());
            let repo_name = Text::new("Repository name:")
                .with_default(&default_repo_name)
                .with_help_message("This will be the name of the repository in Accomplish")
                .prompt()
                .map_err(|e| AppError::ParseError(format!("Input failed: {}", e)))?;

            let local_path = current_dir.to_string_lossy().to_string();

            match endpoints::create_repo(
                auth_service.api_client(),
                &repo_name,
                &selected_project.id,
                Some(&local_path),
                git_remote.as_deref(),
                default_branch.as_deref(),
            )
            .await
            {
                Ok(repo_response) => {
                    println!("✓ Repository '{}' created successfully", repo_name);
                    if let Some(repo_id) = repo_response.get("id").and_then(|v| v.as_str()) {
                        println!("  Repository ID: {}", repo_id);
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to create repository: {}", e);
                    eprintln!("   Project will still be configured locally/globally");
                }
            }
        }
    }

    // Ask user where to store the configuration
    let use_local = if is_git_repo {
        Confirm::new("Store configuration locally in .accomplish.toml? (No = store globally)")
            .with_help_message("Local: adds .accomplish.toml to repo (remember to add to .gitignore)\nGlobal: stores in ~/.accomplish/directories.toml")
            .with_default(false)
            .prompt()
            .map_err(|e| AppError::ParseError(format!("Confirmation failed: {}", e)))?
    } else {
        // For non-git folders, default to local but still give option
        Confirm::new("Store configuration locally in .accomplish.toml? (No = store globally)")
            .with_help_message("Local: creates .accomplish.toml in this folder\nGlobal: stores in ~/.accomplish/directories.toml")
            .with_default(true)
            .prompt()
            .map_err(|e| AppError::ParseError(format!("Confirmation failed: {}", e)))?
    };

    // Clean up existing configuration before creating new one
    if has_local_config || is_tracked_globally {
        cleanup_existing_config(&current_dir, has_local_config, is_tracked_globally)?;
    }

    // Create configuration
    if use_local {
        create_local_config(&current_dir, selected_project, is_git_repo)?;
        println!(
            "✓ Local configuration created for project '{}' ({})",
            selected_project.name,
            selected_project.identifier.to_uppercase()
        );
        if is_git_repo {
            println!("⚠️  Remember to add .accomplish.toml to your .gitignore file!");
        }
    } else {
        create_global_config(&current_dir, selected_project, is_git_repo)?;
        println!(
            "✓ Directory globally tracked with project '{}' ({})",
            selected_project.name,
            selected_project.identifier.to_uppercase()
        );
    }

    if is_git_repo {
        println!("Git repository detected. Project will be associated with this repo.");
    }

    Ok(())
}

fn create_local_config(dir: &Path, project: &Project, is_git_repo: bool) -> Result<(), AppError> {
    let config_path = dir.join(".accomplish.toml");

    let config_content = if is_git_repo {
        let git_remote = get_git_remote(dir).unwrap_or_else(|| "unknown".to_string());
        format!(
            r#"# Accomplish project configuration
# This file associates this directory with an Accomplish project
# Remember to add this file to your .gitignore!

[project]
default_project = "{}"
type = "git"
remote = "{}"

# Generated by: acc init
"#,
            project.identifier, git_remote
        )
    } else {
        format!(
            r#"# Accomplish project configuration
# This file associates this directory with an Accomplish project

[project]
default_project = "{}"
type = "folder"

# Generated by: acc init
"#,
            project.identifier
        )
    };

    fs::write(&config_path, config_content)
        .map_err(|e| AppError::ParseError(format!("Failed to write local config file: {}", e)))?;

    Ok(())
}

fn create_global_config(dir: &Path, project: &Project, is_git_repo: bool) -> Result<(), AppError> {
    let home = home_dir()
        .ok_or_else(|| AppError::ParseError("Could not find home directory".to_string()))?;

    let accomplish_dir = home.join(".accomplish");
    if !accomplish_dir.exists() {
        fs::create_dir_all(&accomplish_dir).map_err(|e| {
            AppError::ParseError(format!("Failed to create .accomplish directory: {}", e))
        })?;
    }

    let global_config_path = accomplish_dir.join("directories.toml");

    // Load existing config or create new one
    let mut config = if global_config_path.exists() {
        let content = fs::read_to_string(&global_config_path)
            .map_err(|e| AppError::ParseError(format!("Failed to read global config: {}", e)))?;
        toml::from_str(&content)
            .map_err(|e| AppError::ParseError(format!("Failed to parse global config: {}", e)))?
    } else {
        GlobalConfig::default()
    };

    // Add new directory entry
    let dir_key = dir.to_string_lossy().to_string();
    let entry = DirectoryEntry {
        project_identifier: project.identifier.clone(),
        directory_type: if is_git_repo {
            "git".to_string()
        } else {
            "folder".to_string()
        },
        git_remote: if is_git_repo {
            get_git_remote(dir)
        } else {
            None
        },
    };

    config.directories.insert(dir_key, entry);

    // Write updated config
    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| AppError::ParseError(format!("Failed to serialize global config: {}", e)))?;

    fs::write(&global_config_path, config_content)
        .map_err(|e| AppError::ParseError(format!("Failed to write global config file: {}", e)))?;

    Ok(())
}

fn is_globally_tracked(dir: &Path) -> Result<bool, AppError> {
    let home = home_dir()
        .ok_or_else(|| AppError::ParseError("Could not find home directory".to_string()))?;

    let global_config_path = home.join(".accomplish/directories.toml");
    if !global_config_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&global_config_path)
        .map_err(|e| AppError::ParseError(format!("Failed to read global config: {}", e)))?;

    let config: GlobalConfig = toml::from_str(&content)
        .map_err(|e| AppError::ParseError(format!("Failed to parse global config: {}", e)))?;

    let dir_key = dir.to_string_lossy().to_string();
    Ok(config.directories.contains_key(&dir_key))
}

fn get_git_remote(dir: &Path) -> Option<String> {
    let git_config_path = dir.join(".git/config");
    if !git_config_path.exists() {
        return None;
    }

    let config_content = fs::read_to_string(&git_config_path).ok()?;

    for line in config_content.lines() {
        if line.trim().starts_with("url = ") {
            let url = line.trim().strip_prefix("url = ")?;
            return Some(url.to_string());
        }
    }

    None
}

fn get_default_branch(dir: &Path) -> Option<String> {
    use std::process::Command;

    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(dir)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout).ok()?;
        Some(branch.trim().to_string())
    } else {
        None
    }
}

fn derive_repo_name(dir: &Path, git_remote: Option<&str>) -> String {
    // First try to derive from git remote URL
    if let Some(remote) = git_remote {
        if let Some(name) = extract_repo_name_from_url(remote) {
            return name;
        }
    }

    // Fall back to directory name
    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
        return name.to_string();
    }

    // Last resort
    "unknown".to_string()
}

fn extract_repo_name_from_url(url: &str) -> Option<String> {
    // Handle GitHub/GitLab style URLs: https://github.com/user/repo.git or git@github.com:user/repo.git
    if url.ends_with(".git") {
        let without_git = &url[..url.len() - 4];
        if let Some(last_slash) = without_git.rfind('/') {
            let repo_part = &without_git[last_slash + 1..];
            if !repo_part.is_empty() {
                return Some(repo_part.to_string());
            }
        }
        if let Some(last_colon) = without_git.rfind(':') {
            let repo_part = &without_git[last_colon + 1..];
            if let Some(slash_pos) = repo_part.find('/') {
                let repo_name = &repo_part[slash_pos + 1..];
                if !repo_name.is_empty() {
                    return Some(repo_name.to_string());
                }
            }
        }
    }

    None
}

fn cleanup_existing_config(dir: &Path, has_local: bool, has_global: bool) -> Result<(), AppError> {
    if has_local {
        let local_config_path = dir.join(".accomplish.toml");
        if local_config_path.exists() {
            fs::remove_file(&local_config_path).map_err(|e| {
                AppError::ParseError(format!("Failed to remove local config: {}", e))
            })?;
        }
    }

    if has_global {
        remove_from_global_config(dir)?;
    }

    Ok(())
}

fn remove_from_global_config(dir: &Path) -> Result<(), AppError> {
    let home = home_dir()
        .ok_or_else(|| AppError::ParseError("Could not find home directory".to_string()))?;

    let global_config_path = home.join(".accomplish/directories.toml");
    if !global_config_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&global_config_path)
        .map_err(|e| AppError::ParseError(format!("Failed to read global config: {}", e)))?;

    let mut config: GlobalConfig = toml::from_str(&content)
        .map_err(|e| AppError::ParseError(format!("Failed to parse global config: {}", e)))?;

    let dir_key = dir.to_string_lossy().to_string();
    config.directories.remove(&dir_key);

    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| AppError::ParseError(format!("Failed to serialize global config: {}", e)))?;

    fs::write(&global_config_path, config_content)
        .map_err(|e| AppError::ParseError(format!("Failed to write global config file: {}", e)))?;

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct GlobalConfig {
    directories: std::collections::HashMap<String, DirectoryEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct DirectoryEntry {
    project_identifier: String,
    directory_type: String,
    git_remote: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir_with_git() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        let config_content = r#"[core]
    repositoryformatversion = 0
    filemode = true
    bare = false
    logallrefupdates = true
[remote "origin"]
    url = https://github.com/user/repo.git
    fetch = +refs/heads/*:refs/remotes/origin/*
"#;
        fs::write(git_dir.join("config"), config_content).unwrap();
        temp_dir
    }

    #[test]
    fn test_get_git_remote() {
        let temp_dir = create_test_dir_with_git();
        let remote = get_git_remote(temp_dir.path());
        assert_eq!(remote, Some("https://github.com/user/repo.git".to_string()));
    }

    #[test]
    fn test_get_git_remote_no_git() {
        let temp_dir = TempDir::new().unwrap();
        let remote = get_git_remote(temp_dir.path());
        assert_eq!(remote, None);
    }

    #[test]
    fn test_create_local_config_git() {
        let temp_dir = TempDir::new().unwrap();
        let project = Project {
            id: "test-id".to_string(),
            name: "Test Project".to_string(),
            identifier: "tst".to_string(),
        };

        create_local_config(temp_dir.path(), &project, true).unwrap();

        let config_path = temp_dir.path().join(".accomplish.toml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("default_project = \"tst\""));
        assert!(content.contains("type = \"git\""));
        assert!(content.contains("remote = \"unknown\""));
    }

    #[test]
    fn test_create_local_config_folder() {
        let temp_dir = TempDir::new().unwrap();
        let project = Project {
            id: "test-id".to_string(),
            name: "Test Project".to_string(),
            identifier: "tst".to_string(),
        };

        create_local_config(temp_dir.path(), &project, false).unwrap();

        let config_path = temp_dir.path().join(".accomplish.toml");
        assert!(config_path.exists());

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("default_project = \"tst\""));
        assert!(content.contains("type = \"folder\""));
        assert!(!content.contains("remote"));
    }

    #[test]
    fn test_derive_repo_name_from_https_url() {
        let temp_dir = TempDir::new().unwrap();
        let remote = "https://github.com/user/my-repo.git";
        let name = derive_repo_name(temp_dir.path(), Some(remote));
        assert_eq!(name, "my-repo");
    }

    #[test]
    fn test_derive_repo_name_from_ssh_url() {
        let temp_dir = TempDir::new().unwrap();
        let remote = "git@github.com:user/my-repo.git";
        let name = derive_repo_name(temp_dir.path(), Some(remote));
        assert_eq!(name, "my-repo");
    }

    #[test]
    fn test_derive_repo_name_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let name = derive_repo_name(temp_dir.path(), None);
        // Should fallback to directory name
        assert!(!name.is_empty());
        assert_ne!(name, "unknown");
    }

    #[test]
    fn test_extract_repo_name_from_url() {
        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            extract_repo_name_from_url("git@github.com:user/repo.git"),
            Some("repo".to_string())
        );
        assert_eq!(
            extract_repo_name_from_url("https://gitlab.com/group/subgroup/project.git"),
            Some("project".to_string())
        );
        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/repo"),
            None
        );
        assert_eq!(extract_repo_name_from_url("invalid-url"), None);
    }

    #[test]
    fn test_cleanup_existing_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".accomplish.toml");

        // Create a local config file
        fs::write(&config_path, "test content").unwrap();
        assert!(config_path.exists());

        // Clean up
        cleanup_existing_config(temp_dir.path(), true, false).unwrap();
        assert!(!config_path.exists());
    }
}
