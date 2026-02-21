use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nyzhi", about = "AI coding agent for the terminal", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Provider to use (openai, anthropic, gemini)
    #[arg(short, long)]
    provider: Option<String>,

    /// Model to use (e.g. gpt-4.1, claude-sonnet-4, gemini-2.5-flash)
    #[arg(short, long)]
    model: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a non-interactive run with a prompt
    Run {
        /// The prompt to send
        prompt: String,
    },
    /// Log in to a provider via OAuth
    Login {
        /// Provider to log in to
        provider: String,
    },
    /// Show current configuration
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("nyzhi=info".parse()?),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let config = nyzhi_config::Config::load()?;
    nyzhi_config::Config::ensure_dirs()?;

    let provider_name = cli
        .provider
        .as_deref()
        .unwrap_or(&config.provider.default);

    let provider = nyzhi_provider::create_provider(provider_name, &config)?;
    let registry = nyzhi_core::tools::default_registry();

    match cli.command {
        Some(Commands::Run { prompt }) => {
            run_once(&*provider, &prompt, &registry).await?;
        }
        Some(Commands::Login { provider: prov }) => {
            eprintln!("OAuth login for '{prov}' is not yet implemented.");
        }
        Some(Commands::Config) => {
            let path = nyzhi_config::Config::config_path();
            println!("Config path: {}", path.display());
            println!("{}", toml::to_string_pretty(&config)?);
        }
        None => {
            let model_name = cli.model.as_deref().unwrap_or(
                provider
                    .supported_models()
                    .first()
                    .map(|m| m.id)
                    .unwrap_or("default"),
            );

            let mut app = nyzhi_tui::App::new(provider_name, model_name, &config.tui);
            app.run(&*provider, &registry).await?;
        }
    }

    Ok(())
}

async fn run_once(
    provider: &dyn nyzhi_provider::Provider,
    prompt: &str,
    registry: &nyzhi_core::tools::ToolRegistry,
) -> Result<()> {
    use nyzhi_core::agent::{AgentConfig, AgentEvent};
    use nyzhi_core::conversation::Thread;
    use nyzhi_core::tools::ToolContext;

    let mut thread = Thread::new();
    let agent_config = AgentConfig::default();
    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel::<AgentEvent>(256);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tool_ctx = ToolContext {
        session_id: thread.id.clone(),
        cwd,
    };

    let tx = event_tx.clone();
    let handle = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match event {
                AgentEvent::TextDelta(text) => print!("{text}"),
                AgentEvent::ToolCallStart { name, .. } => {
                    eprint!("\n[tool: {name}] ");
                }
                AgentEvent::ToolCallDone { name, output, .. } => {
                    let preview = if output.len() > 200 {
                        format!("{}...", &output[..197])
                    } else {
                        output
                    };
                    eprintln!("{name} done: {preview}");
                }
                AgentEvent::ApprovalRequest {
                    tool_name, respond, ..
                } => {
                    // Auto-approve in non-interactive mode
                    let mut guard = respond.lock().await;
                    if let Some(sender) = guard.take() {
                        eprintln!("[auto-approved: {tool_name}]");
                        let _ = sender.send(true);
                    }
                }
                AgentEvent::TurnComplete => break,
                AgentEvent::Error(e) => {
                    eprintln!("\nError: {e}");
                    break;
                }
                _ => {}
            }
        }
    });

    let mut session_usage = nyzhi_core::agent::SessionUsage::default();
    nyzhi_core::agent::run_turn(
        provider,
        &mut thread,
        prompt,
        &agent_config,
        &tx,
        registry,
        &tool_ctx,
        None,
        &mut session_usage,
    )
    .await?;

    let _ = handle.await;
    println!();
    Ok(())
}
