use anyhow::Result;
use clap::{Parser, Subcommand};

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

    match cli.command {
        Some(Commands::Run { prompt }) => {
            run_once(&*provider, &prompt).await?;
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

            let mut app = nyzhi_tui::App::new(provider_name, model_name);
            app.run(&*provider).await?;
        }
    }

    Ok(())
}

async fn run_once(provider: &dyn nyzhi_provider::Provider, prompt: &str) -> Result<()> {
    use futures::StreamExt;
    use nyzhi_provider::*;

    let request = ChatRequest {
        model: String::new(),
        messages: vec![Message {
            role: Role::User,
            content: MessageContent::Text(prompt.to_string()),
        }],
        tools: Vec::new(),
        max_tokens: Some(4096),
        temperature: None,
        system: Some(nyzhi_core::prompt::default_system_prompt()),
        stream: true,
    };

    let mut stream = provider.chat_stream(&request).await?;

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::TextDelta(text) => {
                print!("{text}");
            }
            StreamEvent::Done => break,
            StreamEvent::Error(e) => {
                eprintln!("\nError: {e}");
                break;
            }
            _ => {}
        }
    }

    println!();
    Ok(())
}
