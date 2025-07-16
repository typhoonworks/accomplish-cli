# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

The Accomplish CLI (`acc`) is a Rust-based command-line tool for interacting with the Accomplish platform. It enables users to log work entries, manage projects, and track productivity from the terminal.

## Development Commands

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test
```

### Code Formatting
```bash
cargo fmt --check  # Check formatting
cargo fmt          # Apply formatting
```

### Cross-platform Builds
```bash
# macOS (Intel)
cargo build --release --target x86_64-apple-darwin

# macOS (Apple Silicon)
cargo build --release --target aarch64-apple-darwin

# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## Architecture

### Core Components

- **CLI Interface** (`src/cli.rs`): Defines command-line interface using clap with subcommands for login, logging, project management, etc.
- **Main Entry Point** (`src/main.rs`): Handles command dispatch, authentication state, and error handling
- **Commands** (`src/commands/`): Individual command implementations:
  - `login.rs` - OAuth authentication flow
  - `log.rs` - Create work entries
  - `logs.rs` - List/filter work entries
  - `project.rs` - Project management (list, create, current)
  - `capture.rs` - Git commit capture for work entries
  - `init.rs` - Project initialization
  - `status.rs` - Authentication status

### Authentication System

- **AuthService** (`src/auth/`): Handles OAuth device flow authentication
- **Callback Server** (`src/auth/callback_server.rs`): Local server for OAuth callback handling
- Token storage in `~/.accomplish/` directory

### API Integration

- **Client** (`src/api/client.rs`): HTTP client for Accomplish API
- **Endpoints** (`src/api/endpoints.rs`): API endpoint definitions
- **Models** (`src/api/models.rs`): Data structures for API responses
- **Errors** (`src/api/errors.rs`): API-specific error handling

### Configuration Management

- **Settings** (`src/config.rs`): Configuration loading from `~/.accomplish/config.toml`
- **Project Resolution**: Supports both global and local project configuration
  - Local: `.accomplish.toml` in project directory
  - Global: `~/.accomplish/directories.toml` for directory-project mappings

### Key Features

1. **OAuth Authentication**: Device flow with automatic browser opening
2. **Work Entry Logging**: Text-based with tags and project association
3. **Project Management**: Create, list, and set default projects
4. **Git Integration**: Capture commits as work entries
5. **Flexible Configuration**: Environment-based profiles and directory-specific projects

### Error Handling

The codebase uses a comprehensive error handling system with:
- `AppError` for application-level errors
- `ApiError` for API-specific errors
- Detailed error messages with hints for common issues

### Binary Output

The project builds to a single binary named `acc` as defined in `Cargo.toml`.