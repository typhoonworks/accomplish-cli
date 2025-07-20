use crate::api::endpoints::{generate_worklog_recap, get_recap_status};
use crate::auth::AuthService;
use crate::commands::project;
use crate::errors::AppError;
use crate::utils::duration::parse_since_duration;
use crate::utils::spinner::Spinner;
use chrono::{DateTime, Utc};
use colored::*;
use std::io::{self, Write};

pub async fn execute(
    auth_service: &mut AuthService,
    from: Option<&str>,
    to: Option<&str>,
    since: Option<&str>,
    tags: Option<&[String]>,
    project_identifier: Option<&str>,
) -> Result<(), AppError> {
    // Handle date filtering
    let (from_date, to_date) = if let Some(since_duration) = since {
        if from.is_some() || to.is_some() {
            return Err(AppError::Other(
                "Cannot use --since with --from or --to flags".to_string(),
            ));
        }

        let from_iso =
            parse_since_duration(since_duration).map_err(|e| AppError::Other(e.to_string()))?;

        // Default to now for 'to' when using --since
        let to_iso = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        (Some(from_iso), Some(to_iso))
    } else if from.is_none() && to.is_none() {
        // Default behavior: from start of current day to now
        let now = Utc::now();
        let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let start_of_day_utc = DateTime::<Utc>::from_naive_utc_and_offset(start_of_day, Utc);

        let from_iso = start_of_day_utc.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let to_iso = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        (Some(from_iso), Some(to_iso))
    } else {
        (from.map(String::from), to.map(String::from))
    };

    // Convert project identifier to UUID if provided
    let project_ids = if let Some(identifier) = project_identifier {
        let projects = project::get_projects(auth_service).await?;

        let mut found_id = None;
        for p in &projects {
            if p.identifier.to_lowercase() == identifier.to_lowercase() {
                found_id = Some(p.id.clone());
                break;
            }
        }

        if found_id.is_none() {
            println!(
                "⚠️ Warning: No project found with identifier '{}'",
                identifier
            );
        }

        found_id.map(|id| vec![id])
    } else {
        None
    };

    // Show what we're generating a recap for
    let filter_description = build_filter_description(
        from_date.as_deref(),
        to_date.as_deref(),
        since,
        tags,
        project_identifier,
    );

    println!(
        "{}",
        format!("🤖 Generating recap{}", filter_description).bright_blue()
    );
    print!("{}", "Analyzing worklog entries...".bright_black());
    io::stdout().flush().unwrap();

    // Get API client after project resolution to avoid borrowing conflicts
    let api_client = auth_service.api_client();

    // Extract just the date part (YYYY-MM-DD) from ISO format for API
    let from_date_api = from_date
        .as_ref()
        .and_then(|d| d.split('T').next())
        .map(String::from);
    let to_date_api = to_date
        .as_ref()
        .and_then(|d| d.split('T').next())
        .map(String::from);

    // Generate the recap
    let recap_response = generate_worklog_recap(
        api_client,
        from_date_api.as_deref(),
        to_date_api.as_deref(),
        project_ids.as_deref(),
        tags,
    )
    .await
    .map_err(|e| match e {
        crate::api::errors::ApiError::BadRequest(msg) => {
            AppError::Other(format!("No worklog entries found for the specified filters.\n\nTry:\n• Expanding your date range\n• Removing project or tag filters\n• Using 'acc logs' to see available entries\n\nAPI response: {}", msg))
        }
        crate::api::errors::ApiError::Unauthorized(msg) => {
            if msg.contains("not available") {
                AppError::Other("The recap feature is not available on your current plan. Please upgrade to access AI-powered summaries.".to_string())
            } else {
                AppError::Other(format!("Authentication failed: {}", msg))
            }
        }
        crate::api::errors::ApiError::RateLimited => {
            AppError::Other("You've reached your recap generation limit for this billing cycle. Limits reset monthly.".to_string())
        }
        _ => AppError::Other(format!("Failed to generate recap: {}", e)),
    })?;

    // Clear the "Analyzing..." message
    print!("\r{}\r", " ".repeat(50));
    io::stdout().flush().unwrap();

    match recap_response.status.as_str() {
        "completed" => {
            // Cache hit - get the content immediately
            if let Some(_poll_url) = &recap_response.poll_url {
                let recap_id = &recap_response.recap_id;
                let status_response =
                    get_recap_status(api_client, recap_id).await.map_err(|e| {
                        AppError::Other(format!("Failed to fetch recap content: {}", e))
                    })?;

                if let Some(content) = status_response.content {
                    print_recap_result(
                        &content,
                        &status_response.metadata,
                        &status_response.filters,
                    );
                } else {
                    return Err(AppError::Other(
                        "Recap completed but no content was returned".to_string(),
                    ));
                }
            } else {
                return Err(AppError::Other(
                    "Recap completed but no poll URL was provided".to_string(),
                ));
            }
        }
        "processing" => {
            // For now, fall back to polling since SSE implementation is simplified
            // Future enhancement: implement full SSE streaming
            println!("{}", "✨ Generating your recap...".bright_green());

            let recap_id = &recap_response.recap_id;
            return poll_for_completion(api_client, recap_id).await;
        }
        _ => {
            return Err(AppError::Other(format!(
                "Unexpected recap status: {}",
                recap_response.status
            )));
        }
    }

    Ok(())
}

async fn poll_for_completion(
    api_client: &crate::api::client::ApiClient,
    recap_id: &str,
) -> Result<(), AppError> {
    let mut spinner = Spinner::new();

    spinner
        .spin_with_callback(|| async {
            match get_recap_status(api_client, recap_id).await {
                Ok(status_response) => match status_response.status.as_str() {
                    "completed" => {
                        if let Some(content) = status_response.content {
                            print_recap_result(
                                &content,
                                &status_response.metadata,
                                &status_response.filters,
                            );
                            Some(Ok(()))
                        } else {
                            Some(Err(AppError::Other(
                                "Recap completed but no content was returned".to_string(),
                            )))
                        }
                    }
                    "failed" => Some(Err(AppError::Other(
                        "Recap generation failed. Please try again.".to_string(),
                    ))),
                    "processing" => None, // Continue spinning
                    _ => Some(Err(AppError::Other(format!(
                        "Unexpected recap status: {}",
                        status_response.status
                    )))),
                },
                Err(e) => Some(Err(AppError::Other(format!(
                    "Failed to check recap status: {}",
                    e
                )))),
            }
        })
        .await
}

fn print_recap_result(
    content: &str,
    metadata: &Option<crate::api::models::RecapMetadata>,
    filters: &Option<crate::api::models::RecapFilters>,
) {
    println!("{}", content.white());
    println!();

    if let Some(meta) = metadata {
        // Show entry count
        println!(
            "{}",
            format!("📊 Processed {} worklog entries", meta.entry_count).purple()
        );

        // Show projects found in the data (if any)
        if !meta.projects.is_empty() {
            println!(
                "{}",
                format!("📁 Projects: {}", meta.projects.join(", ")).purple()
            );
        }

        // Show tags found in the data (if any)
        if !meta.tags.is_empty() {
            println!("{}", format!("🏷️  Tags: {}", meta.tags.join(", ")).purple());
        }

        // Show applied filters (if any)
        if let Some(filters) = filters {
            let mut filter_parts = Vec::new();

            if !filters.project_ids.is_empty() {
                filter_parts.push(format!("projects: {}", filters.project_ids.join(", ")));
            }

            if !filters.tags.is_empty() {
                filter_parts.push(format!("tags: {}", filters.tags.join(", ")));
            }

            if !filter_parts.is_empty() {
                println!(
                    "{}",
                    format!("🔍 Filtered by: {}", filter_parts.join(", ")).purple()
                );
            }
        }
    }

    println!("{}", "✅ Recap complete!".bright_green());
}

fn build_filter_description(
    from: Option<&str>,
    to: Option<&str>,
    since: Option<&str>,
    tags: Option<&[String]>,
    project: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(since_duration) = since {
        parts.push(format!("from last {}", since_duration));
    } else if let (Some(from_date), Some(to_date)) = (from, to) {
        // Try to parse and format dates nicely
        let from_formatted = from_date.split('T').next().unwrap_or(from_date);
        let to_formatted = to_date.split('T').next().unwrap_or(to_date);

        if from_formatted == to_formatted {
            parts.push(format!("for {}", from_formatted));
        } else {
            parts.push(format!("from {} to {}", from_formatted, to_formatted));
        }
    } else if let Some(from_date) = from {
        parts.push(format!(
            "from {}",
            from_date.split('T').next().unwrap_or(from_date)
        ));
    } else if let Some(to_date) = to {
        parts.push(format!(
            "until {}",
            to_date.split('T').next().unwrap_or(to_date)
        ));
    }

    if let Some(project_id) = project {
        parts.push(format!("for project {}", project_id.to_uppercase()));
    }

    if let Some(tag_list) = tags {
        if !tag_list.is_empty() {
            parts.push(format!("tagged with {}", tag_list.join(", ")));
        }
    }

    if parts.is_empty() {
        " for today".to_string()
    } else {
        format!(" {}", parts.join(", "))
    }
}
