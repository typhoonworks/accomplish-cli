use crate::api::endpoints::{generate_worklog_recap, get_recap_status};
use crate::auth::AuthService;
use crate::commands::project;
use crate::errors::AppError;
use crate::utils::duration::parse_since_duration;
use crate::utils::spinner::Spinner;
use chrono::{DateTime, Utc};
use colored::*;
use futures::StreamExt;
use std::io::{self, Write};
use tokio::time::{timeout, Duration};
use url::Url;

pub async fn execute(
    auth_service: &mut AuthService,
    from: Option<&str>,
    to: Option<&str>,
    since: Option<&str>,
    tags: Option<&[String]>,
    exclude_tags: Option<&[String]>,
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
            println!("⚠️ Warning: No project found with identifier '{identifier}");
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
        exclude_tags,
        project_identifier,
    );

    println!(
        "{}",
        format!("🤖 Generating recap{filter_description}").bright_blue()
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
        exclude_tags,
    )
    .await
    .map_err(|e| match e {
        crate::api::errors::ApiError::BadRequest(msg) => {
            AppError::Other(format!("No worklog entries found for the specified filters.\n\nTry:\n• Expanding your date range\n• Removing project or tag filters\n• Using 'acc logs' to see available entries\n\nAPI response: {msg}"))
        }
        crate::api::errors::ApiError::Unauthorized(msg) => {
            if msg.contains("not available") {
                AppError::Other("The recap feature is not available on your current plan. Please upgrade to access AI-powered summaries.".to_string())
            } else {
                AppError::Other(format!("Authentication failed: {msg}"))
            }
        }
        crate::api::errors::ApiError::RateLimited => {
            AppError::Other("You've reached your recap generation limit for this billing cycle. Limits reset monthly.".to_string())
        }
        _ => AppError::Other(format!("Failed to generate recap: {e}")),
    })?;

    // Clear the "Analyzing..." message
    print!("\r{}\r", " ".repeat(50));
    io::stdout().flush().unwrap();

    match recap_response.status.as_str() {
        "completed" => {
            // Cache hit - get the content immediately
            if let Some(_poll_url) = &recap_response.poll_url {
                let recap_id = &recap_response.recap_id;
                let status_response = get_recap_status(api_client, recap_id)
                    .await
                    .map_err(|e| AppError::Other(format!("Failed to fetch recap content: {e}")))?;

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
            println!("{}", "✨ Generating your recap...".bright_green());

            let recap_id = &recap_response.recap_id;

            // Try SSE first if available, otherwise fall back to polling
            if let Some(sse_url) = &recap_response.sse_url {
                match try_sse_completion(api_client, sse_url, recap_id).await {
                    Ok(result) => return result,
                    Err(_) => {
                        // SSE failed, fall back to polling
                        return poll_for_completion(api_client, recap_id).await;
                    }
                }
            } else {
                // No SSE URL provided, use polling
                return poll_for_completion(api_client, recap_id).await;
            }
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

async fn try_sse_completion(
    api_client: &crate::api::client::ApiClient,
    sse_url: &str,
    recap_id: &str,
) -> Result<Result<(), AppError>, AppError> {
    // Extract the endpoint from the full SSE URL
    // The sse_url comes as a full URL like "http://localhost:4000/api/v1/worklog/recaps/sse?recap_id=123"
    // We need to extract the path portion for the API client
    let endpoint = if let Ok(url) = Url::parse(sse_url) {
        let path_and_query = if let Some(query) = url.query() {
            format!("{}?{}", &url.path()[1..], query) // Remove leading slash and add query
        } else {
            url.path()[1..].to_string() // Remove leading slash
        };
        path_and_query
    } else {
        // If parsing fails, try to use as-is
        sse_url.to_string()
    };

    // Try to establish SSE connection with timeout
    let mut sse_stream =
        match timeout(Duration::from_secs(5), api_client.stream_sse(&endpoint)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                // Handle specific error cases
                return match e {
                    crate::api::errors::ApiError::NotFound(_) => {
                        // Stream not found - this is the case where recap completed too quickly
                        // Fall back to polling to get the final result
                        Err(e.into())
                    }
                    _ => Err(e.into()),
                };
            }
            Err(_) => {
                // Timeout - fall back to polling
                return Err(AppError::Other("SSE connection timeout".to_string()));
            }
        };

    use std::time::Instant;
    let start_time = Instant::now();
    let mut spinner_index = 0;
    const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    loop {
        // Display spinner
        let elapsed = start_time.elapsed();
        let seconds = elapsed.as_secs();
        let spinner_char = SPINNER_CHARS[spinner_index % SPINNER_CHARS.len()];

        print!(
            "\r{} {}... ({}s)",
            spinner_char.to_string().bright_red(),
            "Generating your recap".bright_red(),
            seconds
        );
        io::stdout().flush().unwrap();

        // Check for SSE events
        match timeout(Duration::from_millis(100), sse_stream.next()).await {
            Ok(Some(Ok(event))) => {
                match event.status.as_str() {
                    "completed" => {
                        // Clear spinner
                        print!("\r{}\r", " ".repeat(80));
                        io::stdout().flush().unwrap();

                        // Get the final content from the polling endpoint
                        // Retry a couple times to ensure backend has fully populated metadata
                        for attempt in 0..3 {
                            if attempt > 0 {
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }

                            match get_recap_status(api_client, recap_id).await {
                                Ok(status_response) => {
                                    if let Some(content) = status_response.content {
                                        // Check if we have reasonable metadata, or if this is the last attempt
                                        let has_metadata = status_response
                                            .metadata
                                            .as_ref()
                                            .map(|m| m.entry_count > 0)
                                            .unwrap_or(false);

                                        if has_metadata || attempt == 2 {
                                            print_recap_result(
                                                &content,
                                                &status_response.metadata,
                                                &status_response.filters,
                                            );
                                            return Ok(Ok(()));
                                        }
                                        // If no metadata yet and not last attempt, continue retrying
                                    } else {
                                        return Ok(Err(AppError::Other(
                                            "Recap completed but no content was returned"
                                                .to_string(),
                                        )));
                                    }
                                }
                                Err(e) => {
                                    if attempt == 2 {
                                        return Ok(Err(AppError::Other(format!(
                                            "Failed to fetch recap content: {e}"
                                        ))));
                                    }
                                    // Continue retrying on non-final attempts
                                }
                            }
                        }

                        // This shouldn't be reached, but just in case
                        return Ok(Err(AppError::Other(
                            "Failed to get complete recap data after retries".to_string(),
                        )));
                    }
                    "failed" => {
                        print!("\r{}\r", " ".repeat(80));
                        io::stdout().flush().unwrap();
                        return Ok(Err(AppError::Other(
                            "Recap generation failed. Please try again.".to_string(),
                        )));
                    }
                    "processing" => {
                        // Continue listening
                    }
                    _ => {
                        print!("\r{}\r", " ".repeat(80));
                        io::stdout().flush().unwrap();
                        return Ok(Err(AppError::Other(format!(
                            "Unexpected recap status: {}",
                            event.status
                        ))));
                    }
                }
            }
            Ok(Some(Err(e))) => {
                // SSE stream error - fall back to polling
                print!("\r{}\r", " ".repeat(80));
                io::stdout().flush().unwrap();
                return Err(AppError::Other(format!("SSE stream error: {e}")));
            }
            Ok(None) => {
                // Stream ended unexpectedly - fall back to polling
                print!("\r{}\r", " ".repeat(80));
                io::stdout().flush().unwrap();
                return Err(AppError::Other("SSE stream ended unexpectedly".to_string()));
            }
            Err(_) => {
                // Timeout - continue with next spinner frame
                spinner_index += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
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
                    "Failed to check recap status: {e}"
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
    exclude_tags: Option<&[String]>,
    project: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(since_duration) = since {
        parts.push(format!("from last {since_duration}"));
    } else if let (Some(from_date), Some(to_date)) = (from, to) {
        // Try to parse and format dates nicely
        let from_formatted = from_date.split('T').next().unwrap_or(from_date);
        let to_formatted = to_date.split('T').next().unwrap_or(to_date);

        if from_formatted == to_formatted {
            parts.push(format!("for {from_formatted}"));
        } else {
            parts.push(format!("from {from_formatted} to {to_formatted}"));
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

    if let Some(exclude_tag_list) = exclude_tags {
        if !exclude_tag_list.is_empty() {
            parts.push(format!("excluding tags {}", exclude_tag_list.join(", ")));
        }
    }

    if parts.is_empty() {
        " for today".to_string()
    } else {
        format!(" {}", parts.join(", "))
    }
}
