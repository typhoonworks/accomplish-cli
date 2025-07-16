use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "accomplish",
    about = "Accomplish CLI for managing tasks",
    version,
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show version information
    Version,

    /// Log in to your account
    Login,

    /// Log out from your account
    Logout,

    /// Check the current authentication status
    Status,

    /// Initialize a project in the current directory
    Init,

    /// Add a new worklog entry
    Log {
        /// The text of the entry (can be specified multiple times, one per line)
        #[arg(short = 'm', long = "message", required_unless_present = "edit")]
        messages: Vec<String>,

        /// Optional tags to associate with the entry (comma-separated)
        #[arg(short = 't', long = "tags", value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Open editor to write the entry
        #[arg(long)]
        edit: bool,

        /// Associate with a project by its 3-letter identifier
        #[arg(short = 'p', long = "project")]
        project_identifier: Option<String>,
    },

    /// Manage projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Capture git commits and optionally create worklog entries
    Capture {
        /// Maximum number of commits to display (default: 25)
        #[arg(short = 'n', long = "limit", default_value = "25")]
        limit: u32,

        /// Open editor to write the entry with pre-filled commit messages
        #[arg(long)]
        edit: bool,
    },

    /// List existing worklog entries (defaults to current project if configured)
    #[command(alias = "ls")]
    Logs {
        /// Filter by project identifier
        #[arg(short = 'p', long = "project")]
        project: Option<String>,

        /// Show entries from all projects (overrides current project default)
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Filter by comma-separated tags
        #[arg(short = 't', long = "tags", value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Start date (inclusive, YYYY-MM-DD format)
        #[arg(long = "from")]
        from: Option<String>,

        /// End date (inclusive, YYYY-MM-DD format)
        #[arg(long = "to")]
        to: Option<String>,

        /// Maximum number of entries to return
        #[arg(short = 'n', long = "limit", default_value = "20")]
        limit: u32,

        /// Show full entry content instead of truncated preview
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// List all projects
    List,
    /// Show which project identifier will be used by default
    Current,
    /// Create a new project
    New {
        /// The name of the project
        name: String,

        /// Optional description of the project
        #[arg(short = 'd', long = "description")]
        description: Option<String>,

        /// Optional 3-letter identifier (auto-generated if not provided)
        #[arg(short = 'i', long = "identifier")]
        identifier: Option<String>,
    },
}
