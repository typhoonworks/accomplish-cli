# Changelog

All notable changes to the Accomplish CLI will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/typhoonworks/accomplish/compare/cli-v0.1.2...HEAD
[0.1.2]: https://github.com/typhoonworks/accomplish/compare/cli-v0.1.1...cli-v0.1.2
[0.1.1]: https://github.com/typhoonworks/accomplish/compare/cli-v0.1.0...cli-v0.1.1
[0.1.0]: https://github.com/typhoonworks/accomplish/releases/tag/cli-v0.1.0
