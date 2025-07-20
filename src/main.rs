mod api;
mod auth;
mod cli;
mod commands;
mod config;
mod errors;
mod storage;
mod user_agent;
mod utils;

use crate::api::errors::ApiError;
use auth::AuthService;
use clap::Parser;
use cli::{Cli, Commands, ProjectCommands};
use commands::{capture, init, log, login, logout, logs, project, recap, status};
use config::Settings;
use errors::AppError;
use serde_json::Value;
use std::env;
use std::process;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 1) Load settings
    let settings = Settings::new()?;

    // 2) Init AuthService
    let mut auth_service = AuthService::new(
        settings.api_base.clone(),
        settings.credentials_dir.clone(),
        &settings.profile,
    );

    // 3) Dispatch commands
    match Cli::parse().command {
        Commands::Version => {
            const VERSION: &str = env!("CARGO_PKG_VERSION");
            const NAME: &str = env!("CARGO_PKG_NAME");
            println!("{NAME} {VERSION}");
        }
        Commands::Login => {
            if let Err(e) = login::execute(&mut auth_service, &settings.client_id).await {
                if let AppError::Api(ApiError::Unauthorized(body)) = &e {
                    let err_code = serde_json::from_str::<Value>(body.as_str())
                        .ok()
                        .and_then(|v| v.get("error").and_then(Value::as_str).map(String::from))
                        .unwrap_or_else(|| "unknown_error".into());

                    let (msg, hint) = match err_code.as_str() {
                        "invalid_client" => (
                            "Invalid client ID".to_string(),
                            "Check your `client_id` in ~/.accomplish/config.toml".to_string(),
                        ),
                        "invalid_request" => (
                            "Malformed request".to_string(),
                            "Ensure `client_id` and `scope` are set".to_string(),
                        ),
                        "authorization_pending" => (
                            "Authorization pending".to_string(),
                            "Approve the request in your browser".to_string(),
                        ),
                        "expired_token" => (
                            "Device code expired".to_string(),
                            "Restart `accomplish login` to get a new code".to_string(),
                        ),
                        other => (
                            format!("Authentication error: {other}"),
                            "See API docs for error codes".to_string(),
                        ),
                    };

                    eprintln!();
                    eprintln!("error: {msg}");
                    eprintln!("hint: {hint}");
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                }
                process::exit(1);
            }
        }
        Commands::Logout => {
            auth_service.clear_tokens();
            logout::execute();
        }
        Commands::Status => {
            status::execute(&mut auth_service).await?;
        }
        Commands::Capture { limit, edit } => {
            if let Err(e) = auth_service.ensure_authenticated().await {
                if matches!(e, AppError::Auth(_)) {
                    eprintln!();
                    eprintln!("You are not authenticated. Run `accomplish login` first.");
                    process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }

            if let Err(e) = capture::execute(&mut auth_service, limit, edit).await {
                eprintln!("\nerror: {e}");
                process::exit(1);
            }
        }
        Commands::Init => {
            if let Err(e) = auth_service.ensure_authenticated().await {
                if matches!(e, AppError::Auth(_)) {
                    eprintln!();
                    eprintln!("You are not authenticated. Run `accomplish login` first.");
                    process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }

            if let Err(e) = init::execute(&mut auth_service).await {
                eprintln!("\nerror: {e}");
                process::exit(1);
            }
        }
        Commands::Log {
            messages,
            tags,
            edit,
            project_identifier,
        } => {
            if let Err(e) = auth_service.ensure_authenticated().await {
                if matches!(e, AppError::Auth(_)) {
                    eprintln!();
                    eprintln!("You are not authenticated. Run `accomplish login` first.");
                    process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }

            let processed_tags: Vec<String> = tags
                .unwrap_or_default()
                .iter()
                .flat_map(|s| s.split(','))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let final_messages = if edit {
                match utils::editor::open_in_editor(Some(utils::editor::DEFAULT_TEMPLATE)) {
                    Ok(content) => {
                        if content.is_empty() {
                            eprintln!("No content provided. Aborting.");
                            process::exit(1);
                        }
                        vec![content]
                    }
                    Err(e) => {
                        eprintln!("\nerror: {e}");
                        process::exit(1);
                    }
                }
            } else {
                messages
            };

            let resolved_project_identifier = project_identifier
                .or_else(|| config::lookup_default_project_for_dir(&env::current_dir().unwrap()))
                .or(settings.default_project.clone());

            if let Err(e) = log::execute(
                &mut auth_service,
                &final_messages,
                &processed_tags,
                resolved_project_identifier.as_deref(),
            )
            .await
            .map(|_| ())
            {
                eprintln!("\nerror: {e}");
                process::exit(1);
            }
        }
        Commands::Project { command } => {
            match command {
                ProjectCommands::Current => {
                    // This command doesn't need authentication - it just reads local config
                    let default = settings.default_project.clone().or_else(|| {
                        config::lookup_default_project_for_dir(&env::current_dir().unwrap())
                    });
                    match default {
                        Some(id) => println!("{id}"),
                        None => println!("(no default project configured)"),
                    }
                }
                ProjectCommands::List | ProjectCommands::New { .. } => {
                    // These commands need authentication
                    if let Err(e) = auth_service.ensure_authenticated().await {
                        if matches!(e, AppError::Auth(_)) {
                            eprintln!();
                            eprintln!("You are not authenticated. Run `accomplish login` first.");
                            process::exit(1);
                        } else {
                            eprintln!();
                            eprintln!("error: {e}");
                            process::exit(1);
                        }
                    }

                    match command {
                        ProjectCommands::List => {
                            if let Err(e) = project::list(&mut auth_service).await {
                                eprintln!("\nerror: {e}");
                                process::exit(1);
                            }
                        }
                        ProjectCommands::New {
                            name,
                            description,
                            identifier,
                        } => {
                            if let Err(e) = project::create_project(
                                &mut auth_service,
                                &name,
                                description.as_deref(),
                                identifier.as_deref(),
                            )
                            .await
                            {
                                eprintln!("\nerror: {e}");
                                process::exit(1);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        Commands::Logs {
            project,
            all,
            tags,
            from,
            to,
            limit,
            verbose,
        } => {
            if let Err(e) = auth_service.ensure_authenticated().await {
                if matches!(e, AppError::Auth(_)) {
                    eprintln!();
                    eprintln!("You are not authenticated. Run `accomplish login` first.");
                    process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }

            let processed_tags: Option<Vec<String>> = tags.map(|t| {
                t.iter()
                    .flat_map(|s| s.split(','))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            // Determine effective project filter:
            // 1. If --all is specified, show all projects (no filter)
            // 2. If -p/--project is specified, use that project
            // 3. Otherwise, use current project if configured
            let effective_project = if all {
                None
            } else {
                project.or_else(|| {
                    config::lookup_default_project_for_dir(&env::current_dir().unwrap())
                        .or(settings.default_project.clone())
                })
            };

            if let Err(e) = logs::execute(
                &mut auth_service,
                effective_project.as_deref(),
                processed_tags.as_deref(),
                from.as_deref(),
                to.as_deref(),
                limit,
                verbose,
            )
            .await
            {
                eprintln!("\nerror: {e}");
                process::exit(1);
            }
        }
        Commands::Recap {
            from,
            to,
            since,
            tags,
            exclude_tags,
            project,
        } => {
            if let Err(e) = auth_service.ensure_authenticated().await {
                if matches!(e, AppError::Auth(_)) {
                    eprintln!();
                    eprintln!("You are not authenticated. Run `accomplish login` first.");
                    process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }

            let processed_tags: Option<Vec<String>> = tags.map(|t| {
                t.iter()
                    .flat_map(|s| s.split_whitespace())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            let processed_exclude_tags: Option<Vec<String>> = exclude_tags.map(|t| {
                t.iter()
                    .flat_map(|s| s.split_whitespace())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            let resolved_project = project
                .or_else(|| config::lookup_default_project_for_dir(&env::current_dir().unwrap()))
                .or(settings.default_project.clone());

            if let Err(e) = recap::execute(
                &mut auth_service,
                from.as_deref(),
                to.as_deref(),
                since.as_deref(),
                processed_tags.as_deref(),
                processed_exclude_tags.as_deref(),
                resolved_project.as_deref(),
            )
            .await
            {
                eprintln!("\nerror: {e}");
                process::exit(1);
            }
        }
    }

    Ok(())
}
