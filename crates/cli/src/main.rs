use anyhow::Result;
use base64::Engine;
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
        /// Attach image file(s) to the prompt
        #[arg(short = 'i', long = "image")]
        images: Vec<String>,
    },
    /// Log in to a provider via OAuth
    Login {
        /// Provider to log in to (gemini, openai)
        provider: String,
    },
    /// Log out from a provider (delete stored OAuth token)
    Logout {
        /// Provider to log out from
        provider: String,
    },
    /// Show current auth status for the active provider
    Whoami,
    /// Show current configuration
    Config,
    /// Initialize a .nyzhi/ project directory
    Init,
    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Add an MCP server
    Add {
        /// Server name
        name: String,
        /// HTTP URL (for remote servers)
        #[arg(long)]
        url: Option<String>,
        /// Scope: "global" or "project" (default: project)
        #[arg(long, default_value = "project")]
        scope: String,
        /// Command and arguments for stdio transport
        #[arg(last = true)]
        command_args: Vec<String>,
    },
    /// List configured MCP servers
    List,
    /// Remove an MCP server
    Remove {
        /// Server name to remove
        name: String,
        /// Scope: "global" or "project" (default: project)
        #[arg(long, default_value = "project")]
        scope: String,
    },
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
    let global_config = nyzhi_config::Config::load()?;
    nyzhi_config::Config::ensure_dirs()?;

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace = nyzhi_core::workspace::detect_workspace(&cwd);

    let config = if workspace.has_nyzhi_config {
        match nyzhi_config::Config::load_project(&workspace.project_root)? {
            Some(project_config) => nyzhi_config::Config::merge(&global_config, &project_config),
            None => global_config,
        }
    } else {
        global_config
    };

    let provider_name = cli
        .provider
        .as_deref()
        .unwrap_or(&config.provider.default);

    match cli.command {
        Some(Commands::Init) => {
            match nyzhi_core::workspace::scaffold_nyzhi_dir(&workspace.project_root) {
                Ok(created) => {
                    if created.is_empty() {
                        println!(
                            ".nyzhi/ already exists in {}",
                            workspace.project_root.display()
                        );
                    } else {
                        println!(
                            "Initialized .nyzhi/ in {}",
                            workspace.project_root.display()
                        );
                        for p in &created {
                            println!("  created {}", p.display());
                        }
                    }
                }
                Err(e) => eprintln!("Failed to initialize: {e}"),
            }
            return Ok(());
        }
        Some(Commands::Config) => {
            let path = nyzhi_config::Config::config_path();
            println!("Config path: {}", path.display());
            if workspace.has_nyzhi_config {
                println!(
                    "Project config: {}",
                    workspace.project_root.join(".nyzhi/config.toml").display()
                );
            }
            println!("{}", toml::to_string_pretty(&config)?);
            return Ok(());
        }
        Some(Commands::Mcp { action }) => {
            handle_mcp_command(action, &workspace, &config).await?;
            return Ok(());
        }
        Some(Commands::Login { provider: prov }) => {
            match nyzhi_auth::oauth::login(&prov).await {
                Ok(_) => println!("Logged in to {prov}."),
                Err(e) => eprintln!("Login failed: {e}"),
            }
            return Ok(());
        }
        Some(Commands::Logout { provider: prov }) => {
            nyzhi_auth::token_store::delete_token(&prov)?;
            println!("Logged out from {prov}.");
            return Ok(());
        }
        Some(Commands::Whoami) => {
            let providers = ["openai", "anthropic", "gemini"];
            println!("Auth status:");
            for prov in &providers {
                let conf_entry = config.provider.entry(prov);
                let has_api_key = conf_entry
                    .and_then(|e| e.api_key.as_deref())
                    .is_some();
                let env_var = nyzhi_auth::api_key::env_var_name(prov);
                let has_env = std::env::var(env_var).is_ok();
                let has_token = nyzhi_auth::token_store::load_token(prov)
                    .ok()
                    .flatten()
                    .is_some();

                let method = if has_api_key {
                    "config api_key".to_string()
                } else if has_env {
                    format!("env ({env_var})")
                } else if has_token {
                    "OAuth token".to_string()
                } else {
                    "none".to_string()
                };
                let marker = if has_api_key || has_env || has_token {
                    "✓"
                } else {
                    "✗"
                };
                println!("  {marker} {prov}: {method}");
            }
            return Ok(());
        }
        _ => {}
    }

    let provider: std::sync::Arc<dyn nyzhi_provider::Provider> =
        nyzhi_provider::create_provider_async(provider_name, &config)
            .await?
            .into();
    let mut registry = nyzhi_core::tools::default_registry();

    let mut all_mcp_servers = config.mcp.servers.clone();
    let mcp_json_servers = nyzhi_core::mcp::load_mcp_json(&workspace.project_root);
    all_mcp_servers.extend(mcp_json_servers);

    let mcp_manager = if !all_mcp_servers.is_empty() {
        match nyzhi_core::mcp::McpManager::start_all(&all_mcp_servers).await {
            Ok(mgr) => {
                for (server_name, tool_def) in mgr.all_tools().await {
                    let desc = tool_def
                        .description
                        .as_deref()
                        .unwrap_or("MCP tool")
                        .to_string();
                    let schema: serde_json::Value =
                        serde_json::to_value(&*tool_def.input_schema).unwrap_or_default();
                    registry.register(Box::new(
                        nyzhi_core::mcp::tool_adapter::McpTool::new(
                            &server_name,
                            &tool_def.name,
                            &desc,
                            schema,
                            mgr.clone(),
                        ),
                    ));
                }
                Some(mgr)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to start MCP servers");
                None
            }
        }
    } else {
        None
    };

    let mcp_summaries: Vec<nyzhi_core::prompt::McpToolSummary> =
        if let Some(mgr) = &mcp_manager {
            let mut s = Vec::new();
            for (server, td) in mgr.all_tools().await {
                s.push(nyzhi_core::prompt::McpToolSummary {
                    server_name: server,
                    tool_name: td.name.to_string(),
                    description: td
                        .description
                        .as_deref()
                        .unwrap_or("MCP tool")
                        .to_string(),
                });
            }
            s
        } else {
            Vec::new()
        };

    registry.register(Box::new(nyzhi_core::tools::task::TaskTool::new(
        provider.clone(),
        std::sync::Arc::new(nyzhi_core::tools::default_registry()),
        2,
    )));

    match cli.command {
        Some(Commands::Run { prompt, images }) => {
            run_once(
                &*provider,
                &prompt,
                &images,
                &registry,
                &workspace,
                &config,
                &mcp_summaries,
            )
            .await?;
        }
        None => {
            let model_name = cli.model.as_deref().unwrap_or(
                provider
                    .supported_models()
                    .first()
                    .map(|m| m.id)
                    .unwrap_or("default"),
            );

            let mut app =
                nyzhi_tui::App::new(provider_name, model_name, &config.tui, workspace.clone());
            app.mcp_manager = mcp_manager.clone();
            app.run(&*provider, &registry, &config).await?;
        }
        _ => unreachable!(),
    }

    if let Some(mgr) = &mcp_manager {
        mgr.stop_all().await;
    }

    Ok(())
}

async fn run_once(
    provider: &dyn nyzhi_provider::Provider,
    prompt: &str,
    image_paths: &[String],
    registry: &nyzhi_core::tools::ToolRegistry,
    workspace: &nyzhi_core::workspace::WorkspaceContext,
    config: &nyzhi_config::Config,
    mcp_tools: &[nyzhi_core::prompt::McpToolSummary],
) -> Result<()> {
    use nyzhi_core::agent::{AgentConfig, AgentEvent};
    use nyzhi_core::conversation::Thread;
    use nyzhi_core::tools::ToolContext;
    use nyzhi_provider::{ContentPart, MessageContent};

    let mut thread = Thread::new();
    let agent_config = AgentConfig {
        system_prompt: nyzhi_core::prompt::build_system_prompt_with_mcp(
            Some(workspace),
            config.agent.custom_instructions.as_deref(),
            mcp_tools,
        ),
        max_steps: config.agent.max_steps.unwrap_or(100),
        max_tokens: config.agent.max_tokens,
        ..AgentConfig::default()
    };
    let (event_tx, mut event_rx) = tokio::sync::broadcast::channel::<AgentEvent>(256);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tool_ctx = ToolContext {
        session_id: thread.id.clone(),
        cwd,
        project_root: workspace.project_root.clone(),
        depth: 0,
        event_tx: Some(event_tx.clone()),
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

    if image_paths.is_empty() {
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
    } else {
        let mut parts = Vec::new();
        for path_str in image_paths {
            let path = std::path::Path::new(path_str);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let media_type = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => anyhow::bail!("Unsupported image format: .{ext}"),
            };
            let bytes = std::fs::read(path)?;
            let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
            parts.push(ContentPart::Image {
                media_type: media_type.to_string(),
                data,
            });
        }
        parts.push(ContentPart::Text {
            text: prompt.to_string(),
        });
        nyzhi_core::agent::run_turn_with_content(
            provider,
            &mut thread,
            MessageContent::Parts(parts),
            &agent_config,
            &tx,
            registry,
            &tool_ctx,
            None,
            &mut session_usage,
        )
        .await?;
    }

    let _ = handle.await;
    println!();
    Ok(())
}

async fn handle_mcp_command(
    action: McpAction,
    workspace: &nyzhi_core::workspace::WorkspaceContext,
    config: &nyzhi_config::Config,
) -> Result<()> {
    use std::collections::HashMap;

    match action {
        McpAction::Add {
            name,
            url,
            scope,
            command_args,
        } => {
            let server_config = if let Some(url) = url {
                nyzhi_config::McpServerConfig::Http {
                    url,
                    headers: HashMap::new(),
                }
            } else if !command_args.is_empty() {
                let command = command_args[0].clone();
                let args = command_args[1..].to_vec();
                nyzhi_config::McpServerConfig::Stdio {
                    command,
                    args,
                    env: HashMap::new(),
                }
            } else {
                eprintln!("Provide either --url or a command after --");
                return Ok(());
            };

            let config_path = if scope == "global" {
                nyzhi_config::Config::config_dir().join("config.toml")
            } else {
                workspace.project_root.join(".nyzhi").join("config.toml")
            };

            let mut existing = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                toml::from_str::<nyzhi_config::Config>(&content).unwrap_or_default()
            } else {
                nyzhi_config::Config::default()
            };

            existing.mcp.servers.insert(name.clone(), server_config);

            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&config_path, toml::to_string_pretty(&existing)?)?;
            println!("Added MCP server '{name}' to {}", config_path.display());
        }
        McpAction::List => {
            let mut all_servers = config.mcp.servers.clone();
            let mcp_json = nyzhi_core::mcp::load_mcp_json(&workspace.project_root);
            all_servers.extend(mcp_json);

            if all_servers.is_empty() {
                println!("No MCP servers configured.");
                println!("  Add one: nyzhi mcp add <name> -- <command> [args...]");
            } else {
                println!("MCP servers ({}):", all_servers.len());
                for (name, cfg) in &all_servers {
                    match cfg {
                        nyzhi_config::McpServerConfig::Stdio {
                            command, args, ..
                        } => {
                            println!("  {name}  stdio  {command} {}", args.join(" "));
                        }
                        nyzhi_config::McpServerConfig::Http { url, .. } => {
                            println!("  {name}  http   {url}");
                        }
                    }
                }
            }
        }
        McpAction::Remove { name, scope } => {
            let config_path = if scope == "global" {
                nyzhi_config::Config::config_dir().join("config.toml")
            } else {
                workspace.project_root.join(".nyzhi").join("config.toml")
            };

            if !config_path.exists() {
                eprintln!("Config file not found: {}", config_path.display());
                return Ok(());
            }

            let content = std::fs::read_to_string(&config_path)?;
            let mut existing: nyzhi_config::Config = toml::from_str(&content)?;

            if existing.mcp.servers.remove(&name).is_some() {
                std::fs::write(&config_path, toml::to_string_pretty(&existing)?)?;
                println!("Removed MCP server '{name}' from {}", config_path.display());
            } else {
                eprintln!("MCP server '{name}' not found in {}", config_path.display());
            }
        }
    }

    Ok(())
}
