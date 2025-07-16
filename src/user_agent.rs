use std::env;

/// Generate a User-Agent string for the CLI
/// Format: accomplish-cli/0.1.0 (linux; x86_64)
pub fn generate_user_agent() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let os = get_os_name();
    let arch = get_arch_name();

    format!("accomplish-cli/{} ({}; {})", version, os, arch)
}

/// Get normalized OS name for User-Agent
fn get_os_name() -> &'static str {
    match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "unknown",
    }
}

/// Get normalized architecture name for User-Agent
fn get_arch_name() -> &'static str {
    match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "arm",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_agent() {
        let user_agent = generate_user_agent();

        // Should start with accomplish-cli/
        assert!(user_agent.starts_with("accomplish-cli/"));

        // Should contain version
        assert!(user_agent.contains(env!("CARGO_PKG_VERSION")));

        // Should contain OS and architecture in parentheses
        assert!(user_agent.contains("("));
        assert!(user_agent.contains(")"));
        assert!(user_agent.contains(";"));

        // Print the actual user agent for verification
        println!("Generated User-Agent: {}", user_agent);
    }

    #[test]
    fn test_os_name() {
        let os = get_os_name();
        assert!(matches!(os, "linux" | "macos" | "windows" | "unknown"));
    }

    #[test]
    fn test_arch_name() {
        let arch = get_arch_name();
        assert!(matches!(arch, "x86_64" | "aarch64" | "arm" | "unknown"));
    }
}
