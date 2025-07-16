use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

use crate::errors::AppError;

/// Default template for worklog entries when opening the editor
pub const DEFAULT_TEMPLATE: &str = r#"# Enter your worklog entry below
# Lines starting with # will be ignored

"#;

/// Opens the user's preferred editor to edit a temporary file.
///
/// This function will:
/// 1. Create a temporary file with optional initial content
/// 2. Open the file in the user's preferred editor ($EDITOR or vi as fallback)
/// 3. Wait for the editor to close
/// 4. Read and return the edited content
///
/// # Arguments
/// * `initial_content` - Optional content to pre-populate the file with
///
/// # Returns
/// * `Result<String, AppError>` - The edited content or an error
pub fn open_in_editor(initial_content: Option<&str>) -> Result<String, AppError> {
    // Create a temporary file
    let temp_dir = env::temp_dir();
    let file_path = temp_dir.join("accomplish_entry.md");

    // Write initial content if provided
    if let Some(content) = initial_content {
        let mut file = File::create(&file_path)?;
        file.write_all(content.as_bytes())?;
    } else {
        File::create(&file_path)?;
    }

    // Try to find the best editor to use
    let editor = get_preferred_editor();

    // Open the editor with appropriate arguments
    let status = if editor == "code" || editor == "code-insiders" {
        // VSCode needs special handling - it returns immediately unless we use --wait
        Command::new(&editor)
            .arg("--wait")
            .arg(&file_path)
            .status()
            .map_err(|e| AppError::Other(format!("Failed to open editor '{editor}': {e}")))?
    } else {
        Command::new(&editor)
            .arg(&file_path)
            .status()
            .map_err(|e| AppError::Other(format!("Failed to open editor '{editor}': {e}")))?
    };

    if !status.success() {
        return Err(AppError::Other(format!(
            "Editor '{editor}' exited with non-zero status"
        )));
    }

    // Read the edited content
    let content = read_file_content(&file_path)?;

    // Clean up the temporary file
    if let Err(e) = fs::remove_file(&file_path) {
        eprintln!("Warning: Failed to remove temporary file: {e}");
    }

    // Filter out comment lines (lines starting with #)
    let filtered_content = content
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<&str>>()
        .join("\n");

    Ok(filtered_content)
}

/// Determines the best editor to use based on environment variables and common editors
/// Returns the command to use for editing
fn get_preferred_editor() -> String {
    // First check VISUAL and EDITOR environment variables
    if let Ok(editor) = env::var("VISUAL") {
        return editor;
    }

    if let Ok(editor) = env::var("EDITOR") {
        return editor;
    }

    // Check for common editors on macOS
    let common_editors = [
        "code",          // VSCode
        "code-insiders", // VSCode Insiders
        "subl",          // Sublime Text
        "atom",          // Atom
        "nano",          // Nano (simpler than vi)
        "vim",           // Vim
        "vi",            // Vi (last resort)
    ];

    for editor in common_editors.iter() {
        if Command::new(editor).arg("--version").output().is_ok() {
            return editor.to_string();
        }
    }

    // Default fallback
    "vi".to_string()
}

/// Reads the content of a file and returns it as a String.
fn read_file_content(path: &PathBuf) -> Result<String, AppError> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Trim trailing whitespace
    let content = content.trim_end().to_string();

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_read_file_content() {
        // Create a temporary file with some content
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("test_read_content.txt");
        let test_content = "Test content\nLine 2\n";

        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(test_content.as_bytes()).unwrap();
        }

        // Read the content
        let content = read_file_content(&file_path).unwrap();

        // Clean up
        fs::remove_file(&file_path).unwrap();

        // Verify
        assert_eq!(content, "Test content\nLine 2");
    }
}
