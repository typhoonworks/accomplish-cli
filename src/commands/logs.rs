use crate::api::endpoints::fetch_worklog_entries;
use crate::auth::AuthService;
use crate::commands::project;
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use colored::*;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use serde_json::Value;
use std::io::{self, Write};

pub async fn execute(
    auth_service: &mut AuthService,
    project_identifier: Option<&str>,
    tags: Option<&[String]>,
    from: Option<&str>,
    to: Option<&str>,
    limit: u32,
    verbose: bool,
) -> Result<(), AppError> {
    // Convert project identifier to project UUID if provided
    let project_id = if let Some(identifier) = project_identifier {
        let projects = project::get_projects(auth_service).await?;

        let mut found_id = None;
        for p in &projects {
            if p.identifier.to_lowercase() == identifier.to_lowercase() {
                found_id = Some(p.id.clone());
                break;
            }
        }

        if found_id.is_none() {
            println!("⚠️ Warning: No project found with identifier '{identifier}'");
        }

        found_id
    } else {
        None
    };

    let api_client = auth_service.api_client();
    let mut cursor: Option<String> = None;
    let mut total_entries_shown = 0;
    let mut all_entries_loaded = false;

    // Load first page
    let response = fetch_worklog_entries(
        api_client,
        project_id.as_deref(),
        tags,
        from,
        to,
        limit,
        cursor.as_deref(),
    )
    .await?;

    if let Some(entries) = response.get("entries").and_then(Value::as_array) {
        if entries.is_empty() {
            println!("No entries found.");
            return Ok(());
        }

        // Show first page entries
        for entry in entries {
            print_entry(entry, verbose)?;
        }
        total_entries_shown += entries.len();

        // Check if we have more pages
        let meta = response.get("meta");
        if let Some(end_cursor) = meta.and_then(|m| m.get("end_cursor").and_then(Value::as_str)) {
            cursor = Some(end_cursor.to_string());
        } else {
            all_entries_loaded = true;
        }

        // If we have more entries, start interactive pagination
        if !all_entries_loaded {
            interactive_pagination(
                auth_service,
                project_id.as_deref(),
                tags,
                from,
                to,
                limit,
                verbose,
                &mut cursor,
                &mut total_entries_shown,
            )
            .await?;
        }
    } else {
        println!("No entries found.");
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn interactive_pagination(
    auth_service: &mut AuthService,
    project_id: Option<&str>,
    tags: Option<&[String]>,
    from: Option<&str>,
    to: Option<&str>,
    limit: u32,
    verbose: bool,
    cursor: &mut Option<String>,
    total_entries_shown: &mut usize,
) -> Result<(), AppError> {
    let api_client = auth_service.api_client();

    loop {
        // Show pagination prompt
        print!("{}", "Press ".bright_black());
        print!("{}", "SPACE".bright_white());
        print!("{}", " for more, ".bright_black());
        print!("{}", "q".bright_white());
        print!("{}", " to quit: ".bright_black());
        io::stdout().flush().unwrap();

        // Enable raw mode for single key input
        enable_raw_mode()
            .map_err(|e| AppError::Other(format!("Failed to enable raw mode: {e}")))?;

        let key_result = read();

        // Always disable raw mode before continuing
        disable_raw_mode()
            .map_err(|e| AppError::Other(format!("Failed to disable raw mode: {e}")))?;

        match key_result {
            Ok(Event::Key(KeyEvent { code, .. })) => {
                match code {
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        // Clear the prompt line
                        print!("\r{}\r", " ".repeat(50));
                        io::stdout().flush().unwrap();

                        // Load next page
                        let response = fetch_worklog_entries(
                            api_client,
                            project_id,
                            tags,
                            from,
                            to,
                            limit,
                            cursor.as_deref(),
                        )
                        .await?;

                        if let Some(entries) = response.get("entries").and_then(Value::as_array) {
                            if entries.is_empty() {
                                println!("No more entries.");
                                break;
                            }

                            for entry in entries {
                                print_entry(entry, verbose)?;
                            }
                            *total_entries_shown += entries.len();

                            // Update cursor for next page
                            let meta = response.get("meta");
                            if let Some(end_cursor) =
                                meta.and_then(|m| m.get("end_cursor").and_then(Value::as_str))
                            {
                                *cursor = Some(end_cursor.to_string());
                            } else {
                                println!("No more entries.");
                                break;
                            }
                        } else {
                            println!("No more entries.");
                            break;
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                        // Clear the prompt line
                        print!("\r{}\r", " ".repeat(50));
                        io::stdout().flush().unwrap();
                        break;
                    }
                    _ => {
                        // Clear the prompt line and show it again
                        print!("\r{}\r", " ".repeat(50));
                        io::stdout().flush().unwrap();
                        continue;
                    }
                }
            }
            Ok(_) => continue,
            Err(e) => {
                return Err(AppError::Other(format!("Error reading key: {e}")));
            }
        }
    }

    Ok(())
}

fn print_entry(entry: &Value, verbose: bool) -> Result<(), AppError> {
    let id = entry.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let content = entry.get("content").and_then(Value::as_str).unwrap_or("");
    let recorded_at = entry
        .get("recorded_at")
        .and_then(Value::as_str)
        .unwrap_or("");

    // Parse and format the date
    let formatted_date = if !recorded_at.is_empty() {
        match recorded_at.parse::<DateTime<Utc>>() {
            Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            Err(_) => recorded_at.to_string(),
        }
    } else {
        "unknown".to_string()
    };

    // Get tags
    let tags = entry
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    // Get project info
    let project_info = entry
        .get("project")
        .and_then(|p| p.get("identifier"))
        .and_then(Value::as_str)
        .map(|id| format!(" [{id}]"))
        .unwrap_or_default();

    // Format the header with colors
    let header = format!(
        "{} ({}){}",
        formatted_date.bright_blue(),
        &id[..8].bright_black(),
        project_info.bright_green()
    );

    // Print the entry
    println!("{header}");

    if verbose {
        // In verbose mode, show full content
        println!("  {}", content.white());
        if !tags.is_empty() {
            println!("  Tags: {}", tags.bright_yellow());
        }
        println!();
    } else {
        // In non-verbose mode, show truncated first line
        let first_line = content.lines().next().unwrap_or("");
        let truncated = if first_line.len() > 80 {
            format!("{}...", &first_line[..77])
        } else {
            first_line.to_string()
        };

        if !truncated.is_empty() {
            println!("  {}", truncated.white());
        }

        // Show tags on the same line or next line if present
        if !tags.is_empty() {
            println!("  Tags: {}", tags.bright_yellow());
        }
        println!();
    }

    Ok(())
}
