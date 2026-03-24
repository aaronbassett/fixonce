use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod output;
mod tui;

use commands::create::CreateArgs;
use commands::feedback::FeedbackRatingArg;
use commands::query::QueryArgs;
use commands::update::UpdateArgs;
use output::OutputFormat;

// Re-export for clarity in match arms below.
use commands::hook::{run_hook, HookSubcommand};
use commands::{analyze::run_analyze, context::run_context, detect::run_detect};

/// Default Supabase URL.  Override with the `FIXONCE_API_URL` env-var.
const DEFAULT_API_URL: &str = "https://fixonce.supabase.co";

/// `FixOnce` — persistent memory for Claude Code agents
#[derive(Debug, Parser)]
#[command(name = "fixonce", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Override the backend API URL
    #[arg(long, env = "FIXONCE_API_URL", global = true)]
    api_url: Option<String>,

    /// Supabase anon (public) key for auth operations
    #[arg(long, env = "FIXONCE_ANON_KEY", global = true)]
    anon_key: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Log in with GitHub OAuth
    Login,
    /// Authenticate this machine via challenge-response
    Auth,
    /// Manage signing keys
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Create a new memory
    Create {
        #[command(flatten)]
        args: CreateArgs,
    },
    /// Get a memory by ID
    Get {
        /// Memory UUID
        id: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Update an existing memory
    Update {
        /// Memory UUID
        id: String,
        #[command(flatten)]
        args: UpdateArgs,
    },
    /// Soft-delete a memory
    Delete {
        /// Memory UUID
        id: String,
    },
    /// Submit feedback on a memory
    Feedback {
        /// Memory UUID
        id: String,
        /// Rating
        rating: FeedbackRatingArg,
        /// Optional free-text context
        #[arg(long)]
        context: Option<String>,
    },
    /// Query memories with the read pipeline
    Query {
        #[command(flatten)]
        args: QueryArgs,
    },
    /// View the lineage chain for a memory
    Lineage {
        /// Memory UUID
        id: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Detect Midnight ecosystem versions in the current project
    Detect {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Gather full project context (versions + git + file structure)
    Context {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Analyse a Claude Code session transcript and extract memory candidates
    Analyze {
        /// Path to the session log file
        session_log: String,
        /// Output format (non-TTY only)
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Manage CLI configuration
    Config,
    /// Launch the interactive Terminal User Interface
    Tui,
    /// Dispatch a Claude Code lifecycle hook event (called by shell scripts)
    Hook {
        #[command(subcommand)]
        event: HookSubcommand,
    },
}

#[derive(Debug, Subcommand)]
enum KeysAction {
    /// Generate and register a new signing key
    Add,
    /// List registered signing keys
    List,
    /// Revoke a signing key by ID
    Revoke {
        /// Key ID to revoke
        key_id: String,
    },
}

/// Derive a short transaction name from the [`Commands`] variant.
fn transaction_name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Login => "cli.login",
        Commands::Auth => "cli.auth",
        Commands::Keys { .. } => "cli.keys",
        Commands::Create { .. } => "cli.create",
        Commands::Get { .. } => "cli.get",
        Commands::Update { .. } => "cli.update",
        Commands::Delete { .. } => "cli.delete",
        Commands::Feedback { .. } => "cli.feedback",
        Commands::Query { .. } => "cli.query",
        Commands::Lineage { .. } => "cli.lineage",
        Commands::Detect { .. } => "cli.detect",
        Commands::Context { .. } => "cli.context",
        Commands::Analyze { .. } => "cli.analyze",
        Commands::Config => "cli.config",
        Commands::Tui => "cli.tui",
        Commands::Hook { .. } => "cli.hook",
    }
}

/// Initialise Sentry when `SENTRY_DSN` is set.
///
/// Returns the guard that must be held for the lifetime of the program. When
/// `SENTRY_DSN` is absent the guard is a no-op and Sentry adds zero overhead.
fn init_sentry() -> sentry::ClientInitGuard {
    let dsn = std::env::var("SENTRY_DSN").unwrap_or_default();
    let environment = std::env::var("SENTRY_ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_owned());

    sentry::init(sentry::ClientOptions {
        dsn: dsn.parse().ok(),
        release: Some(env!("CARGO_PKG_VERSION").into()),
        environment: Some(environment.into()),
        traces_sample_rate: 1.0,
        ..Default::default()
    })
}

/// Set up the `tracing` subscriber with the Sentry layer so that spans are
/// reported as Sentry transactions/spans and events as breadcrumbs.
fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(sentry::integrations::tracing::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    // Sentry guard — must outlive the entire program.
    let _sentry_guard = init_sentry();
    init_tracing();

    let cli = Cli::parse();

    let api_url = cli.api_url.as_deref().unwrap_or(DEFAULT_API_URL);
    let anon_key = cli.anon_key.as_deref().unwrap_or("");

    let Some(cmd) = cli.command else {
        // No subcommand — clap will show help via --help; nothing else to do.
        return Ok(());
    };

    let tx_name = transaction_name(&cmd);
    let tx_ctx = sentry::TransactionContext::new(tx_name, "cli.command");
    let tx = sentry::start_transaction(tx_ctx);
    sentry::configure_scope(|scope| scope.set_span(Some(tx.clone().into())));

    let result = run_command(cmd, api_url, anon_key).await;

    tx.set_status(if result.is_ok() {
        sentry::protocol::SpanStatus::Ok
    } else {
        sentry::protocol::SpanStatus::InternalError
    });
    tx.finish();

    result
}

async fn run_command(cmd: Commands, api_url: &str, anon_key: &str) -> Result<()> {
    match cmd {
        Commands::Login => commands::login::run_login(api_url, anon_key).await?,
        Commands::Auth => commands::auth::run_auth(api_url).await?,
        Commands::Keys { action } => match action {
            KeysAction::Add => commands::keys::run_keys_add(api_url).await?,
            KeysAction::List => commands::keys::run_keys_list(api_url).await?,
            KeysAction::Revoke { key_id } => {
                commands::keys::run_keys_revoke(api_url, &key_id).await?;
            }
        },
        Commands::Create { args } => commands::create::run_create(api_url, args).await?,
        Commands::Get { id, format } => commands::get::run_get(api_url, &id, format).await?,
        Commands::Update { id, args } => commands::update::run_update(api_url, &id, args).await?,
        Commands::Delete { id } => commands::delete::run_delete(api_url, &id).await?,
        Commands::Feedback {
            id,
            rating,
            context,
        } => commands::feedback::run_feedback(api_url, &id, rating, context).await?,
        Commands::Query { args } => commands::query::run_query(api_url, args).await?,
        Commands::Lineage { id, format } => {
            commands::lineage::run_lineage(api_url, &id, format).await?;
        }
        Commands::Detect { format } => run_detect(format).await?,
        Commands::Context { format } => run_context(format).await?,
        Commands::Analyze {
            session_log,
            format,
        } => {
            run_analyze(api_url, &session_log, format).await?;
        }
        Commands::Config => commands::config::run_config()?,
        Commands::Tui => tui::app::run_tui(api_url).await?,
        Commands::Hook { event } => run_hook(api_url, event).await?,
    }

    Ok(())
}
