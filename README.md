# Accomplish CLI

The Accomplish CLI (`acc`) is a command-line tool for interacting with the [Accomplish](https://accomplish.dev) platform. It allows you to log work entries, manage projects, and track your productivity directly from your terminal.

## Features

- **Work Logging**: Log work entries with timestamps, tags, and project associations
- **Project Management**: Create and manage projects
- **Authentication**: Secure OAuth-based authentication with the Accomplish platform
- **Cross-Platform**: Works on macOS, Linux, and Windows

## Installation

### Install from pre-built binaries (recommended)

```bash
curl -sSL https://raw.githubusercontent.com/typhoonworks/accomplish-cli/main/install.sh | bash
```

### Download from releases

Download the latest binary for your platform from the [releases page](https://github.com/typhoonworks/accomplish-cli/releases).

### Build from source

```bash
git clone https://github.com/typhoonworks/accomplish-cli.git
cd accomplish-cli
cargo build --release
sudo cp target/release/acc /usr/local/bin/
```

## Quick Start

1. **Login to your Accomplish account:**
   ```bash
   acc login
   ```

2. **Log a work entry:**
   ```bash
   acc log -m "Fixed authentication bug" -t bug,backend
   ```

3. **View your recent entries:**
   ```bash
   acc logs
   ```

4. **Set up a project:**
   ```bash
   acc project set my-project
   ```

For detailed usage instructions, see the [User Guide](USER_GUIDE.md).

## Commands

- `acc login` - Authenticate with Accomplish
- `acc logout` - Sign out of your account
- `acc log` - Record a work entry
- `acc logs` - View work entries
- `acc project` - Manage projects
- `acc status` - Show current status
- `acc init` - Initialize configuration

## Configuration

The CLI stores configuration in `~/.accomplish/config.toml`. You can manually edit this file or use the `acc init` command to set up your preferences.

## Versioning

This project follows [Semantic Versioning](https://semver.org/). Given a version number MAJOR.MINOR.PATCH:

- MAJOR version: Incompatible API changes
- MINOR version: Backwards-compatible functionality additions
- PATCH version: Backwards-compatible bug fixes

As this is pre-1.0 software, the API is considered unstable and breaking changes may occur in minor version updates.

## Development

### Prerequisites

- Rust 1.70+
- Cargo

### Building

```bash
cargo build --release
```

### Running tests

```bash
cargo test
```

### Code formatting

```bash
cargo fmt --check  # Check formatting
cargo fmt          # Apply formatting
```

### Building for different platforms

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

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with release notes
3. Commit changes: `git commit -m "chore: release v0.x.x"`
4. Create and push tag: `git tag v0.x.x && git push origin v0.x.x`
5. GitHub Actions will automatically build and publish binaries

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Support

- Report bugs and request features on [GitHub Issues](https://github.com/typhoonworks/accomplish-cli/issues)
- Check out the [User Guide](USER_GUIDE.md) for detailed usage instructions
