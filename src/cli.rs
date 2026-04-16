use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "vicraft",
    about = "Spec-driven AI workflow CLI — Vitooler series",
    version,
    author
)]
pub struct Cli {
    /// Show full error chain for debugging
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize vicraft structure in current project
    Init,

    /// Analyze codebase and update .aider/context/
    Scan,

    /// Create a new issue file from template
    #[command(name = "new-issue")]
    NewIssue {
        /// Issue name / slug (e.g. "user-authentication")
        name: String,
        /// Open the file in $EDITOR after creation
        #[arg(long)]
        open: bool,
    },

    /// Generate spec from issue file or Linear issue [STEP 1]
    Spec {
        /// Path to .issues/ file or Linear issue ID (e.g. LINEAR-123)
        input: String,
    },

    /// Generate implementation plan from spec [STEP 2]
    Plan {
        /// Path to spec file or review file (for iteration)
        input: String,
    },

    /// Run implementation from plan [STEP 3]
    #[command(name = "impl")]
    Impl {
        /// Path to plan file
        plan: String,
    },

    /// Review implementation against spec and plan [STEP 4]
    Review,

    /// Create conventional commit (amends WIP commit)
    Commit {
        /// Generate message only for staged files
        #[arg(long)]
        staged: bool,
    },

    /// Create a pull request
    Pr,

    /// Clear Aider conversation history
    #[command(name = "clear-context")]
    ClearContext,

    /// Manage project knowledge base (skills)
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
}

#[derive(Subcommand)]
pub enum SkillsAction {
    /// List all skill files
    List,
    /// Open a skill file in $EDITOR
    Edit {
        /// Skill name (e.g. "database", "api")
        name: String,
    },
    /// Create a new skill file
    New {
        /// Skill name (e.g. "caching")
        name: String,
    },
    /// Re-sync skills from codebase via Aider
    Sync,
}
