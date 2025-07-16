# Accomplish CLI User Guide

The Accomplish CLI (`acc`) is a command-line tool for logging work, managing projects, and interacting with the Accomplish platform directly from your terminal.

## Installation

### Quick Install (Recommended)

```bash
curl -sSL https://raw.githubusercontent.com/typhoonworks/accomplish-cli/main/install.sh | bash
```

This will:
- Download the latest pre-built binary for your platform
- Install it to `/usr/local/bin/acc`
- Set up the default configuration automatically

### Manual Installation

1. Download the latest release from [GitHub Releases](https://github.com/typhoonworks/accomplish-cli/releases)
2. Extract the binary for your platform:
   - **Linux**: `acc-x86_64-unknown-linux-gnu`
   - **macOS Intel**: `acc-x86_64-apple-darwin`
   - **macOS Apple Silicon**: `acc-aarch64-apple-darwin`
   - **Windows**: `acc-x86_64-pc-windows-msvc.exe`
3. Move the binary to your PATH (e.g., `/usr/local/bin/acc`)
4. Make it executable: `chmod +x /usr/local/bin/acc`

## Getting Started

### 1. Authentication

Before using the CLI, you need to authenticate with your Accomplish account:

```bash
acc login
```

This will:
- Open your browser to authenticate
- Store your access token securely in `~/.accomplish/default/token`
- Return to the terminal once authentication is complete

### 2. Check Status

Verify your authentication status:

```bash
acc status
```

### 3. Log Your First Work Entry

```bash
acc log -m "Implemented user authentication feature" -t backend,security
```

## Getting Help

The CLI has built-in help documentation for all commands:

```bash
# General help
acc --help

# Help for specific commands
acc log --help
acc project --help
acc logs --help
```

Each command shows:
- Usage syntax
- Available options and flags
- Examples
- Default values

## Core Commands

### Authentication

#### `acc login`
Authenticate with your Accomplish account using OAuth device flow.

#### `acc logout`
Remove stored credentials and log out.

#### `acc status`
Check your current authentication status.

### Work Logging

#### `acc log`
Create a new work log entry.

**Options:**
- `-m, --message <TEXT>`: Entry content (can be used multiple times for multi-line entries)
- `-t, --tags <TAGS>`: Comma-separated tags (e.g., `backend,api,bugfix`)
- `-p, --project <PROJECT>`: Associate with a specific project by identifier
- `--edit`: Open your default editor to write the entry

**Examples:**
```bash
# Simple entry
acc log -m "Fixed authentication bug"

# Multi-line entry
acc log -m "Added new feature:" -m "- User profile management" -m "- Avatar upload support"

# With tags and project
acc log -m "Implemented API endpoint" -t backend,api -p ABC

# Open editor
acc log --edit
```

#### `acc logs` (alias: `acc ls`)
List your work log entries.

**Options:**
- `-p, --project <PROJECT>`: Filter by project identifier
- `-a, --all`: Show entries from all projects
- `-t, --tags <TAGS>`: Filter by comma-separated tags
- `--from <DATE>`: Start date (YYYY-MM-DD format)
- `--to <DATE>`: End date (YYYY-MM-DD format)
- `-n, --limit <NUMBER>`: Maximum number of entries (default: 20)
- `-v, --verbose`: Show full entry content instead of truncated preview

**Examples:**
```bash
# Recent entries (uses current project if configured)
acc logs

# All entries from all projects
acc logs --all

# Entries from specific project
acc logs -p ABC

# Entries with specific tags
acc logs -t backend,api

# Entries from last week
acc logs --from 2025-01-09 --to 2025-01-16

# Full content view
acc logs -v
```

### Project Management

#### `acc project list`
List all your projects.

#### `acc project current`
Show the current default project identifier.

#### `acc project new`
Create a new project.

**Options:**
- `<NAME>`: Project name (required)
- `-d, --description <TEXT>`: Optional project description
- `-i, --identifier <ID>`: Optional 3-letter identifier (auto-generated if not provided)

**Examples:**
```bash
# Simple project
acc project new "My Website"

# With description and custom identifier
acc project new "E-commerce Platform" -d "Online store with payment integration" -i ECP
```

### Git Integration

#### `acc capture`
Capture recent git commits and optionally create work log entries from them.

**Options:**
- `-n, --limit <NUMBER>`: Maximum number of commits to display (default: 25)
- `--edit`: Open editor to write entry with pre-filled commit messages

**Examples:**
```bash
# Show recent commits
acc capture

# Show last 10 commits
acc capture -n 10

# Create work log entry from commits
acc capture --edit
```

#### `acc init`
Initialize project configuration in the current directory.

This command:
- Detects git repository information
- Creates or updates local project configuration
- Links the directory to an Accomplish project

### Utility Commands

#### `acc version`
Display the CLI version information.

## Configuration

The CLI stores its configuration in `~/.accomplish/config.toml`. On first run, it automatically creates a default configuration:

```toml
[default]
api_base = "https://accomplish.dev"
client_id = "90w0AXnlNgnh2XBJdexYjw"
credentials_dir = "~/.accomplish"
```

### Environment Variables

You can override configuration using environment variables:

```bash
export ACCOMPLISH__DEFAULT__API_BASE="https://custom.accomplish.dev"
export ACCOMPLISH__DEFAULT__CLIENT_ID="your-client-id"
```

## Project Configuration

### Local Project Setup

For directory-specific project defaults, use `acc init` or create `.accomplish.toml` in your project directory:

```toml
[project]
default_project = "ABC"
```

### Global Project Mapping

The CLI can automatically detect which project to use based on your current directory by maintaining a global mapping in `~/.accomplish/directories.toml`.

## Tips and Best Practices

### 1. Efficient Logging
- Use tags consistently to categorize your work
- Set up project defaults with `acc init` in your project directories
- Use `acc capture` to convert git commits into work logs

### 2. Workflow Integration
```bash
# After implementing a feature
git commit -m "Add user authentication"
acc log -m "Implemented user authentication system" -t backend,security

# Review your work for the day
acc logs --from $(date -d "today" +%Y-%m-%d)

# Track work on specific project
cd /path/to/project
acc init  # Set up project mapping
acc log -m "Fixed critical bug in payment processing"
```

### 3. Editor Integration
Set your preferred editor for multi-line entries:
```bash
export EDITOR="code"  # VS Code
export EDITOR="vim"   # Vim
export EDITOR="nano"  # Nano
```

## Troubleshooting

### Authentication Issues
- Run `acc logout` then `acc login` to refresh your credentials
- Check that your browser allows the authentication redirect
- Ensure you have network access to `https://accomplish.dev`

### Configuration Issues
- Delete `~/.accomplish/config.toml` to regenerate default configuration
- Check file permissions on the `~/.accomplish` directory
- Use `acc status` to verify your configuration is working

### Project Issues
- Use `acc project list` to see available projects
- Check that your project identifier is correct (3-letter code)
- Use `acc project current` to see what project the CLI will use by default

## Getting Help

- Run any command with `--help` for detailed usage information
- Check the CLI version with `acc version`
- Report issues at: https://github.com/typhoonworks/accomplish-cli/issues

## Version Information

This guide is for Accomplish CLI v0.1.0. The CLI follows semantic versioning and is currently in pre-release.

---

*Happy logging! 🚀*
