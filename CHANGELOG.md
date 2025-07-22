# Changelog

All notable changes to the Accomplish CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Updated `rand` crate from 0.8.5 to 0.9.2
- Updated spinner utility to use new rand 0.9 API (`thread_rng()` → `rng()`, updated imports)

## [0.4.0] - 2025-07-20

### Added
- New `acc recap` command for generating AI-powered worklog summaries with real-time progress updates via SSE streaming
  - Automatically uses the current project (like `acc log` and `acc logs`) when no project is explicitly specified
  - Supports filtering by tags with `-t, --tags` flag
  - Supports excluding entries with specific tags using `-x, --exclude-tags` flag
  - Supports date filtering with `--from`, `--to`, and `--since` options
- Server-Sent Events (SSE) support with automatic fallback to polling for robust recap generation
- Enhanced duration parsing with human-friendly expressions: `yesterday`, `today`, `this-week`, `last-week`, `this-month`, `last-month`
- Dependabot configuration for automated dependency updates

## [0.3.0] - 2025-07-18

### Added
- Rate limiting support for API requests (HTTP 429 responses)
- User-friendly error message when rate limits are exceeded: "Consider spacing out your requests to avoid hitting the rate limit"

## [0.2.0] - 2025-01-16

### Changed
- **BREAKING**: Renamed internal `auth` module to `auth_service` to resolve module naming conflicts
- Comprehensive code quality improvements based on strict Clippy linting rules
- Updated all format strings to use inline variable syntax for better readability and performance
- Replaced manual string manipulation with idiomatic Rust methods (`strip_prefix`, `strip_suffix`)
- Improved loop patterns using `enumerate()` instead of manual counters
- Enhanced test assertions to use more appropriate comparison methods

### Fixed
- Fixed over 100 Clippy warnings and errors across the entire codebase
- Resolved module inception issues in the authentication module structure
- Corrected inefficient vector allocations in tests

## [0.1.2] - 2025-01-16

### Fixed
- Completely eliminated OpenSSL dependencies by configuring git2 with no default features
- Fixed macOS cross-compilation build issues by switching from OpenSSL to rustls-tls for all HTTP requests
- Fixed Linux build issues by removing unused keyring dependency that required D-Bus
- Resolved GitHub Actions release workflow failures for all platforms

## [0.1.1] - 2025-01-16

### Fixed
- Fixed macOS cross-compilation build issues by switching from OpenSSL to rustls-tls for HTTP requests
- Fixed Linux build issues by removing unused keyring dependency that required D-Bus
- Resolved GitHub Actions release workflow failures for all platforms

## [0.1.0] - 2025-01-16

### Added
- Initial pre-release version of the Accomplish CLI
- Core authentication flow with OAuth device flow
- Work logging functionality with `acc log` command
- Project management commands (`acc project list`, `acc project new`)
- Git commit capture with `acc capture`
- Worklog listing with `acc logs`
- Configuration management with TOML files
- Multi-profile support for different environments
- Secure credential storage in user directory
- Version command (`acc version`) to display CLI version information

### Changed
- Migrated from version 0.2.0 to 0.1.0 following SemVer conventions for pre-release software

[Unreleased]: https://github.com/typhoonworks/accomplish-cli/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/typhoonworks/accomplish-cli/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/typhoonworks/accomplish-cli/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/typhoonworks/accomplish-cli/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/typhoonworks/accomplish/compare/cli-v0.1.1...cli-v0.1.2
[0.1.1]: https://github.com/typhoonworks/accomplish/compare/cli-v0.1.0...cli-v0.1.1
[0.1.0]: https://github.com/typhoonworks/accomplish/releases/tag/cli-v0.1.0
