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

    /// Trust mode: off, limited, or full (auto-approve tool calls)
    #[arg(short = 'y', long = "trust")]
    trust: Option<String>,

    /// Continue the most recent session
    #[arg(short = 'c', long = "continue")]
    continue_session: bool,

    /// Resume a specific session by ID prefix or title search
    #[arg(short, long)]
    session: Option<String>,

    /// Join an agent team as lead (sets team context for all tools)
    #[arg(long)]
    team_name: Option<String>,

    /// Display mode for agent teams: in-process or tmux
    #[arg(long, default_value = "in-process")]
    teammate_mode: String,
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
    /// Log in to a provider (OAuth or API key)
    Login {
        /// Provider to log in to (e.g. openai, anthropic, gemini, openrouter)
        provider: Option<String>,
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
    /// List saved sessions
    Sessions {
        /// Optional search query to filter by ID or title
        query: Option<String>,
    },
    /// Export a session to markdown
    Export {
        /// Session ID prefix or title query
        id: String,
        /// Output file path (default: nyzhi-export-<timestamp>.md)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Manage a specific session
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Show session or overall statistics
    Stats,
    /// Show cost report
    Cost {
        /// Period: daily, weekly, monthly (default: daily)
        #[arg(default_value = "daily")]
        period: String,
    },
    /// Deep-initialize project with AGENTS.md and structure analysis
    Deepinit,
    /// Manage agent teams
    Teams {
        #[command(subcommand)]
        action: TeamsAction,
    },
    /// List learned skills
    Skills,
    /// Check rate limit status or wait for rate limits to clear
    Wait,
    /// Replay a session's event timeline
    Replay {
        /// Session ID
        id: String,
        /// Filter by event type (e.g. tool, error)
        #[arg(long)]
        filter: Option<String>,
    },
    /// Check for updates and self-update
    Update {
        /// Force update even if already on latest
        #[arg(long)]
        force: bool,
        /// Rollback to a backup (path or "latest")
        #[arg(long)]
        rollback: Option<String>,
        /// List available backups
        #[arg(long)]
        list_backups: bool,
    },
    /// Completely uninstall nyzhi: remove binary, config, data, backups, and PATH entries
    Uninstall {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Auto-diagnose and fix CI failures. Reads failure logs and runs an agent to fix them.
    CiFix {
        /// Path to CI log file (reads from stdin if not provided)
        #[arg(short, long)]
        log_file: Option<String>,
        /// CI format: auto, junit, tap, plain (default: auto)
        #[arg(long, default_value = "auto")]
        format: String,
        /// Auto-commit the fix
        #[arg(long)]
        commit: bool,
    },
}

#[derive(Subcommand)]
enum TeamsAction {
    /// List all agent teams
    List,
    /// Show team details
    Show {
        /// Team name
        name: String,
    },
    /// Delete a team and its artifacts
    Delete {
        /// Team name
        name: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// Delete a saved session
    Delete {
        /// Session ID prefix or title query
        id: String,
    },
    /// Rename a saved session
    Rename {
        /// Session ID prefix or title query
        id: String,
        /// New title for the session
        title: String,
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

    let provider_name_owned = cli
        .provider
        .clone()
        .unwrap_or_else(|| config.provider.default.clone());
    let provider_name: &str = &provider_name_owned;

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
            let prov = match prov {
                Some(p) => p,
                None => {
                    println!("Select a provider:");
                    for (i, def) in nyzhi_config::BUILT_IN_PROVIDERS.iter().enumerate() {
                        println!("  {}: {}", i + 1, def.name);
                    }
                    print!("Enter number: ");
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    let idx: usize = input.trim().parse().unwrap_or(0);
                    if idx == 0 || idx > nyzhi_config::BUILT_IN_PROVIDERS.len() {
                        eprintln!("Invalid selection.");
                        return Ok(());
                    }
                    nyzhi_config::BUILT_IN_PROVIDERS[idx - 1].id.to_string()
                }
            };
            let def = nyzhi_config::find_provider_def(&prov);
            let supports_oauth = def.map(|d| d.supports_oauth).unwrap_or(false);

            if supports_oauth {
                match nyzhi_auth::oauth::login(&prov).await {
                    Ok(_) => println!("Logged in to {prov} via OAuth."),
                    Err(e) => {
                        eprintln!("OAuth login failed: {e}");
                        eprintln!("You can add an API key instead.");
                        prompt_api_key(&prov)?;
                    }
                }
            } else {
                prompt_api_key(&prov)?;
            }
            return Ok(());
        }
        Some(Commands::Logout { provider: prov }) => {
            nyzhi_auth::token_store::delete_token(&prov)?;
            println!("Logged out from {prov}.");
            return Ok(());
        }
        Some(Commands::Whoami) => {
            println!("Auth status:");
            let mut seen = std::collections::HashSet::new();
            for def in nyzhi_config::BUILT_IN_PROVIDERS {
                seen.insert(def.id.to_string());
                let status = nyzhi_auth::auth_status(def.id);
                let marker = if status != "not connected" { "✓" } else { "✗" };
                let mut line = format!("  {marker} {}: {status}", def.name);
                if let Ok(accounts) = nyzhi_auth::token_store::list_accounts(def.id) {
                    if accounts.len() > 1 {
                        line.push_str(&format!(" ({} accounts)", accounts.len()));
                        for (i, acc) in accounts.iter().enumerate() {
                            let label = acc.label.as_deref().unwrap_or("default");
                            let active = if acc.active { " [active]" } else { "" };
                            let rl = if acc.rate_limited_until.is_some() {
                                " [rate-limited]"
                            } else {
                                ""
                            };
                            println!("      {}. {}{}{}", i + 1, label, active, rl);
                        }
                    }
                }
                println!("{line}");
            }
            for (name, _entry) in &config.provider.providers {
                if seen.contains(name) { continue; }
                let status = nyzhi_auth::auth_status(name);
                let marker = if status != "not connected" { "✓" } else { "✗" };
                println!("  {marker} {name} (custom): {status}");
            }
            return Ok(());
        }
        Some(Commands::Sessions { query }) => {
            let sessions = if let Some(q) = &query {
                nyzhi_core::session::find_sessions(q)?
            } else {
                nyzhi_core::session::list_sessions()?
            };
            if sessions.is_empty() {
                if let Some(q) = &query {
                    println!("No sessions matching '{q}'");
                } else {
                    println!("No saved sessions.");
                }
            } else {
                println!(
                    "{:<10} {:<40} {:>4}  {:<20} UPDATED",
                    "ID", "TITLE", "MSGS", "PROVIDER/MODEL"
                );
                for s in sessions.iter().take(50) {
                    let title = if s.title.len() > 38 {
                        format!("{}…", &s.title[..37])
                    } else {
                        s.title.clone()
                    };
                    let pm = format!("{}/{}", s.provider, s.model);
                    let pm_display = if pm.len() > 20 {
                        format!("{}…", &pm[..19])
                    } else {
                        pm
                    };
                    println!(
                        "{:<10} {:<40} {:>4}  {:<20} {}",
                        &s.id[..s.id.len().min(8)],
                        title,
                        s.message_count,
                        pm_display,
                        s.updated_at.format("%Y-%m-%d %H:%M"),
                    );
                }
                if sessions.len() > 50 {
                    println!("... and {} more", sessions.len() - 50);
                }
            }
            return Ok(());
        }
        Some(Commands::Export { id, output }) => {
            let matches = nyzhi_core::session::find_sessions(&id)?;
            let meta = match matches.len() {
                0 => {
                    eprintln!("No session matching '{id}'");
                    std::process::exit(1);
                }
                1 => &matches[0],
                n => {
                    eprintln!("Ambiguous: {n} sessions match '{id}'. Be more specific.");
                    for s in matches.iter().take(10) {
                        eprintln!("  [{}] {}", &s.id[..8], s.title);
                    }
                    std::process::exit(1);
                }
            };
            let (thread, session_meta) = nyzhi_core::session::load_session(&meta.id)?;
            let export_meta = nyzhi_tui::export::ExportMeta {
                provider: session_meta.provider.clone(),
                model: session_meta.model.clone(),
                usage: nyzhi_core::agent::SessionUsage::default(),
                timestamp: session_meta.updated_at,
            };
            let markdown = nyzhi_tui::export::export_thread_markdown(
                thread.messages(),
                &export_meta,
            );
            let path = output.unwrap_or_else(nyzhi_tui::export::default_export_path);
            std::fs::write(&path, &markdown)?;
            println!(
                "Exported session [{}] \"{}\" ({} messages, {} bytes) to {}",
                &session_meta.id[..8],
                session_meta.title,
                session_meta.message_count,
                markdown.len(),
                path,
            );
            return Ok(());
        }
        Some(Commands::Session { action }) => {
            match action {
                SessionAction::Delete { id } => {
                    let matches = nyzhi_core::session::find_sessions(&id)?;
                    match matches.len() {
                        0 => {
                            eprintln!("No session matching '{id}'");
                            std::process::exit(1);
                        }
                        1 => {
                            let s = &matches[0];
                            nyzhi_core::session::delete_session(&s.id)?;
                            println!("Deleted session [{}] \"{}\"", &s.id[..8], s.title);
                        }
                        n => {
                            eprintln!(
                                "Ambiguous: {n} sessions match '{id}'. Be more specific."
                            );
                            for s in matches.iter().take(10) {
                                eprintln!("  [{}] {}", &s.id[..8], s.title);
                            }
                            std::process::exit(1);
                        }
                    }
                }
                SessionAction::Rename { id, title } => {
                    let matches = nyzhi_core::session::find_sessions(&id)?;
                    match matches.len() {
                        0 => {
                            eprintln!("No session matching '{id}'");
                            std::process::exit(1);
                        }
                        1 => {
                            let s = &matches[0];
                            nyzhi_core::session::rename_session(&s.id, &title)?;
                            println!(
                                "Renamed session [{}] to \"{}\"",
                                &s.id[..8],
                                title,
                            );
                        }
                        n => {
                            eprintln!(
                                "Ambiguous: {n} sessions match '{id}'. Be more specific."
                            );
                            for s in matches.iter().take(10) {
                                eprintln!("  [{}] {}", &s.id[..8], s.title);
                            }
                            std::process::exit(1);
                        }
                    }
                }
            }
            return Ok(());
        }
        Some(Commands::Stats) => {
            let entries = nyzhi_core::analytics::load_entries()?;
            if entries.is_empty() {
                println!("No usage data recorded yet.");
            } else {
                let report = nyzhi_core::analytics::generate_report(
                    &entries, "All-time", 0,
                );
                println!("{}", report.display());
                println!("\nTotal sessions: {}", {
                    let mut ids: Vec<&str> = entries.iter().map(|e| e.session_id.as_str()).collect();
                    ids.sort();
                    ids.dedup();
                    ids.len()
                });
                println!("Total entries:  {}", entries.len());
            }
            return Ok(());
        }
        Some(Commands::Teams { action }) => {
            match action {
                TeamsAction::List => {
                    let teams = nyzhi_core::teams::list_teams();
                    if teams.is_empty() {
                        println!("No agent teams found.");
                    } else {
                        println!("Agent teams:\n");
                        for name in &teams {
                            if let Ok(config) = nyzhi_core::teams::config::TeamConfig::load(name) {
                                let member_names: Vec<&str> = config.members.iter().map(|m| m.name.as_str()).collect();
                                println!("  {} ({} members: {})", name, config.members.len(), member_names.join(", "));
                            } else {
                                println!("  {} (config error)", name);
                            }
                        }
                    }
                }
                TeamsAction::Show { name } => {
                    match nyzhi_core::teams::config::TeamConfig::load(&name) {
                        Ok(config) => {
                            println!("Team: {}\n", config.name);
                            println!("Members:");
                            for m in &config.members {
                                let role = m.role.as_deref().unwrap_or("-");
                                let id = m.agent_id.as_deref().unwrap_or("n/a");
                                println!("  {} [{}] role={} id={}", m.name, m.agent_type, role, id);
                            }
                            let tasks = nyzhi_core::teams::tasks::list_tasks(&name, None).unwrap_or_default();
                            if !tasks.is_empty() {
                                println!("\nTasks ({}):", tasks.len());
                                for t in &tasks {
                                    let owner = t.owner.as_deref().unwrap_or("unassigned");
                                    println!("  #{} [{}] {} ({})", t.id, t.status, t.subject, owner);
                                }
                            }
                        }
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
                TeamsAction::Delete { name } => {
                    match nyzhi_core::teams::config::TeamConfig::delete(&name) {
                        Ok(()) => println!("Team '{name}' deleted."),
                        Err(e) => eprintln!("Error: {e}"),
                    }
                }
            }
            return Ok(());
        }
        Some(Commands::Deepinit) => {
            let path = nyzhi_core::deepinit::write_agents_md(&workspace.project_root)?;
            println!("Generated {}", path.display());
            let scan = nyzhi_core::deepinit::scan_project(&workspace.project_root)?;
            if !scan.languages.is_empty() {
                println!("Languages: {}", scan.languages.join(", "));
            }
            if !scan.frameworks.is_empty() {
                println!("Frameworks: {}", scan.frameworks.join(", "));
            }
            println!("Directories: {}", scan.directories.len());
            return Ok(());
        }
        Some(Commands::Skills) => {
            let skills = nyzhi_core::skills::load_skills(&workspace.project_root)?;
            if skills.is_empty() {
                println!("No learned skills found.");
                println!("Use /learn in the TUI to extract patterns from your session.");
            } else {
                println!("Learned skills ({}):", skills.len());
                for skill in &skills {
                    println!("  - {} ({})", skill.name, skill.path.display());
                }
            }
            return Ok(());
        }
        Some(Commands::Wait) => {
            println!("Rate limit daemon not yet active.");
            println!("If you hit rate limits during a session, nyzhi will auto-retry with backoff.");
            println!("Configure retry in .nyzhi/config.toml:\n");
            println!("  [agent.retry]");
            println!("  max_retries = 3");
            println!("  initial_backoff_ms = 1000");
            println!("  max_backoff_ms = 30000");
            return Ok(());
        }
        Some(Commands::Replay { id, filter }) => {
            let events = nyzhi_core::replay::load_replay(&id)?;
            if events.is_empty() {
                println!("No replay data for session '{id}'");
            } else {
                println!("{}", nyzhi_core::replay::format_replay(
                    &events,
                    filter.as_deref(),
                ));
            }
            return Ok(());
        }
        Some(Commands::Update { force, rollback: rollback_target, list_backups: show_backups }) => {
            if show_backups {
                let backups = nyzhi_core::updater::list_backups();
                if backups.is_empty() {
                    println!("No backups available.");
                } else {
                    println!("Available backups (newest first):");
                    for b in &backups {
                        println!("  {}", b.display());
                    }
                }
                return Ok(());
            }
            if let Some(target) = rollback_target {
                let path = if target == "latest" {
                    let backups = nyzhi_core::updater::list_backups();
                    if backups.is_empty() {
                        eprintln!("No backups available to rollback to.");
                        std::process::exit(1);
                    }
                    backups[0].clone()
                } else {
                    std::path::PathBuf::from(&target)
                };
                println!("Rolling back to {}...", path.display());
                match nyzhi_core::updater::rollback(&path) {
                    Ok(()) => println!("Rollback successful. Restart nyzhi."),
                    Err(e) => {
                        eprintln!("Rollback failed: {e}");
                        std::process::exit(1);
                    }
                }
                return Ok(());
            }
            println!("Checking for updates...");
            let update_config = config.update.clone();
            let result = if force {
                nyzhi_core::updater::check_for_update_force(&update_config).await
            } else {
                let mut cfg = update_config;
                cfg.check_interval_hours = 0;
                nyzhi_core::updater::check_for_update(&cfg).await
            };
            match result {
                Ok(Some(info)) => {
                    println!(
                        "Update available: v{} -> v{}",
                        info.current_version, info.new_version
                    );
                    if let Some(ref cl) = info.changelog {
                        println!("  {cl}");
                    }
                    println!("Backing up current binary and downloading update...");
                    match nyzhi_core::updater::download_and_apply(&info).await {
                        Ok(ur) => {
                            println!(
                                "Updated to v{}! Restart nyzhi to use the new version.",
                                ur.new_version
                            );
                            if let Some(ref bp) = ur.backup_path {
                                println!("  Backup: {}", bp.display());
                            }
                            if ur.verified {
                                println!("  Post-flight verification: passed");
                            }
                            let warnings = nyzhi_core::updater::startup_health_check();
                            for w in &warnings {
                                eprintln!("  Warning: {w}");
                            }
                        }
                        Err(e) => {
                            eprintln!("Update failed: {e:#}");
                            let backups = nyzhi_core::updater::list_backups();
                            if !backups.is_empty() {
                                eprintln!(
                                    "  Rollback available: nyzhi update --rollback {}",
                                    backups[0].display()
                                );
                            }
                            std::process::exit(1);
                        }
                    }
                }
                Ok(None) => {
                    println!(
                        "Already on the latest version (v{}).",
                        nyzhi_core::updater::current_version()
                    );
                }
                Err(e) => {
                    eprintln!("Update check failed: {e}");
                    std::process::exit(1);
                }
            }
            return Ok(());
        }
        Some(Commands::Uninstall { yes }) => {
            let nyzhi_home = std::env::var("NYZHI_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".nyzhi"));
            let config_dir = nyzhi_config::Config::config_dir();
            let data_dir = nyzhi_config::Config::data_dir();

            println!("This will permanently remove:");
            println!("  Binary & backups:  {}", nyzhi_home.display());
            println!("  Configuration:     {}", config_dir.display());
            println!("  Data & sessions:   {}", data_dir.display());
            println!("  OAuth tokens:      OS keyring (nyzhi-*)");
            println!("  Shell PATH entry:  ~/.zshrc / ~/.bashrc / fish conf.d");
            println!();

            if !yes {
                eprint!("Are you sure? Type 'yes' to confirm: ");
                use std::io::Write;
                std::io::stderr().flush()?;
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;
                if answer.trim() != "yes" {
                    println!("Aborted.");
                    return Ok(());
                }
            }

            let mut removed = Vec::new();
            let mut errors = Vec::new();

            for dir in [&nyzhi_home, &config_dir, &data_dir] {
                if dir.exists() {
                    match std::fs::remove_dir_all(dir) {
                        Ok(()) => removed.push(format!("  ✓ {}", dir.display())),
                        Err(e) => errors.push(format!("  ✗ {}: {e}", dir.display())),
                    }
                }
            }

            for provider in ["openai", "anthropic", "gemini", "openrouter"] {
                if let Ok(entry) = keyring::Entry::new("nyzhi", provider) {
                    match entry.delete_credential() {
                        Ok(()) => removed.push(format!("  ✓ keyring: nyzhi/{provider}")),
                        Err(keyring::Error::NoEntry) => {}
                        Err(e) => errors.push(format!("  ✗ keyring nyzhi/{provider}: {e}")),
                    }
                }
                let svc = format!("nyzhi-{provider}");
                if let Ok(entry) = keyring::Entry::new(&svc, "oauth_token") {
                    match entry.delete_credential() {
                        Ok(()) => removed.push(format!("  ✓ keyring: {svc}/oauth_token")),
                        Err(keyring::Error::NoEntry) => {}
                        Err(e) => errors.push(format!("  ✗ keyring {svc}: {e}")),
                    }
                }
            }

            for (shell, profile_path) in [
                ("zsh", dirs::home_dir().map(|h| h.join(".zshrc"))),
                ("bash", dirs::home_dir().map(|h| h.join(".bashrc"))),
                ("bash", dirs::home_dir().map(|h| h.join(".bash_profile"))),
                ("sh", dirs::home_dir().map(|h| h.join(".profile"))),
            ] {
                if let Some(path) = profile_path {
                    if path.exists() {
                        if let Ok(contents) = std::fs::read_to_string(&path) {
                            let filtered: Vec<&str> = contents
                                .lines()
                                .filter(|line| {
                                    !line.contains("nyzhi") || line.trim_start().starts_with('#')
                                })
                                .collect();
                            if filtered.len() < contents.lines().count() {
                                let new_contents = filtered.join("\n") + "\n";
                                match std::fs::write(&path, new_contents) {
                                    Ok(()) => removed.push(format!(
                                        "  ✓ cleaned PATH from {} ({})",
                                        path.display(),
                                        shell
                                    )),
                                    Err(e) => errors.push(format!(
                                        "  ✗ {}: {e}",
                                        path.display()
                                    )),
                                }
                            }
                        }
                    }
                }
            }
            if let Some(fish_conf) = dirs::config_dir().map(|c| c.join("fish/conf.d/nyzhi.fish")) {
                if fish_conf.exists() {
                    match std::fs::remove_file(&fish_conf) {
                        Ok(()) => removed.push(format!("  ✓ {}", fish_conf.display())),
                        Err(e) => errors.push(format!("  ✗ {}: {e}", fish_conf.display())),
                    }
                }
            }

            println!();
            if !removed.is_empty() {
                println!("Removed:");
                for r in &removed {
                    println!("{r}");
                }
            }
            if !errors.is_empty() {
                println!("\nErrors:");
                for e in &errors {
                    eprintln!("{e}");
                }
            }
            println!("\nnyzhi has been uninstalled. Restart your shell to update PATH.");
            return Ok(());
        }
        Some(Commands::Cost { period }) => {
            let entries = nyzhi_core::analytics::load_entries()?;
            if entries.is_empty() {
                println!("No usage data recorded yet.");
                return Ok(());
            }
            let now = nyzhi_core::analytics::now_ts();
            let (label, since) = match period.as_str() {
                "daily" | "day" => ("Daily", now.saturating_sub(86_400)),
                "weekly" | "week" => ("Weekly", now.saturating_sub(86_400 * 7)),
                "monthly" | "month" => ("Monthly", now.saturating_sub(86_400 * 30)),
                other => {
                    eprintln!("Unknown period: {other} (use daily, weekly, monthly)");
                    std::process::exit(1);
                }
            };
            let report = nyzhi_core::analytics::generate_report(&entries, label, since);
            println!("{}", report.display());
            return Ok(());
        }
        _ => {}
    }

    let provider: std::sync::Arc<dyn nyzhi_provider::Provider> =
        nyzhi_provider::create_provider_async(provider_name, &config)
            .await?
            .into();
    let bundle = nyzhi_core::tools::default_registry();
    let mut registry = bundle.registry;
    let _todo_store = bundle.todo_store;
    let deferred_index = bundle.deferred_index;

    let mut all_mcp_servers = config.mcp.servers.clone();
    let mcp_json_servers = nyzhi_core::mcp::load_mcp_json(&workspace.project_root);
    all_mcp_servers.extend(mcp_json_servers);

    let mcp_manager = if !all_mcp_servers.is_empty() {
        match nyzhi_core::mcp::McpManager::start_all(&all_mcp_servers).await {
            Ok(mgr) => {
                let all_tools = mgr.all_tools().await;
                let defer_mcp = all_tools.len() > 15;
                for (server_name, tool_def) in &all_tools {
                    let desc = tool_def
                        .description
                        .as_deref()
                        .unwrap_or("MCP tool")
                        .to_string();
                    let schema: serde_json::Value =
                        serde_json::to_value(&*tool_def.input_schema).unwrap_or_default();
                    let tool = Box::new(
                        nyzhi_core::mcp::tool_adapter::McpTool::new(
                            server_name,
                            &tool_def.name,
                            &desc,
                            schema,
                            mgr.clone(),
                        ),
                    );
                    if defer_mcp {
                        registry.register_deferred(tool);
                    } else {
                        registry.register(tool);
                    }
                }

                if defer_mcp {
                    if let Ok(mut idx) = deferred_index.write() {
                        *idx = registry.deferred_index();
                    }
                    let index_dir = workspace.project_root.join(".nyzhi").join("context").join("tools");
                    std::fs::create_dir_all(&index_dir).ok();
                    let mut index_content = String::from("# MCP Tool Index\n\n");
                    for (server_name, tool_def) in &all_tools {
                        let desc = tool_def.description.as_deref().unwrap_or("MCP tool");
                        index_content.push_str(&format!(
                            "- `mcp__{}__{}`  {}\n",
                            server_name, tool_def.name, desc
                        ));
                    }
                    std::fs::write(index_dir.join("mcp-index.md"), &index_content).ok();
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

    // Multi-agent tools will be registered per-session with access to the event_tx.
    // The old single-shot `task` tool is replaced by spawn_agent/send_input/wait/close_agent/resume_agent.

    let mut config = config;
    if let Some(trust_str) = &cli.trust {
        match trust_str.parse::<nyzhi_config::TrustMode>() {
            Ok(mode) => config.agent.trust.mode = mode,
            Err(e) => {
                eprintln!("Invalid --trust value: {e}");
                std::process::exit(1);
            }
        }
    }

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
                cli.team_name.as_deref(),
            )
            .await?;
        }
        None => {
            if cli.continue_session && cli.session.is_some() {
                eprintln!("Cannot use both --continue and --session");
                std::process::exit(1);
            }

            let initial_session = if cli.continue_session {
                match nyzhi_core::session::latest_session()? {
                    Some(meta) => {
                        let (thread, meta) = nyzhi_core::session::load_session(&meta.id)?;
                        Some((thread, meta))
                    }
                    None => {
                        eprintln!("No sessions to continue.");
                        std::process::exit(1);
                    }
                }
            } else if let Some(ref query) = cli.session {
                let matches = nyzhi_core::session::find_sessions(query)?;
                match matches.len() {
                    0 => {
                        eprintln!("No session matching '{query}'");
                        std::process::exit(1);
                    }
                    1 => {
                        let (thread, meta) =
                            nyzhi_core::session::load_session(&matches[0].id)?;
                        Some((thread, meta))
                    }
                    n => {
                        eprintln!("Ambiguous: {n} sessions match '{query}'. Be more specific.");
                        for s in matches.iter().take(10) {
                            eprintln!("  [{}] {}", &s.id[..8], s.title);
                        }
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            let model_name = cli.model.clone().unwrap_or_else(|| {
                provider
                    .supported_models()
                    .first()
                    .map(|m| m.id.as_str())
                    .unwrap_or("default")
                    .to_string()
            });

            let mut app =
                nyzhi_tui::App::new(provider_name, &model_name, &config.tui, workspace.clone());
            app.mcp_manager = mcp_manager.clone();
            app.initial_session = initial_session;
            app.run(provider.clone(), registry, &config).await?;
        }
        Some(Commands::CiFix { log_file, format, commit }) => {
            let ci_log = if let Some(path) = &log_file {
                std::fs::read_to_string(path)?
            } else {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            };

            if ci_log.trim().is_empty() {
                eprintln!("No CI log content provided. Pass --log-file or pipe via stdin.");
                std::process::exit(1);
            }

            let prompt = format!(
                "CI failure log (format: {format}):\n\n```\n{ci_log}\n```\n\n\
                 Analyze this CI failure. Identify the root cause, fix the code, and verify \
                 the fix passes. Be surgical - only change what's needed to make CI green."
            );

            run_once(
                &*provider,
                &prompt,
                &[],
                &registry,
                &workspace,
                &config,
                &mcp_summaries,
                None,
            )
            .await?;

            if commit {
                let output = tokio::process::Command::new("git")
                    .args(["add", "-A"])
                    .current_dir(&workspace.project_root)
                    .output()
                    .await?;
                if output.status.success() {
                    let _ = tokio::process::Command::new("git")
                        .args(["commit", "-m", "fix: auto-fix CI failure (nyzhi ci-fix)"])
                        .current_dir(&workspace.project_root)
                        .output()
                        .await;
                }
            }
        }
        _ => unreachable!(),
    }

    if let Some(mgr) = &mcp_manager {
        mgr.stop_all().await;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_once(
    provider: &dyn nyzhi_provider::Provider,
    prompt: &str,
    image_paths: &[String],
    registry: &nyzhi_core::tools::ToolRegistry,
    workspace: &nyzhi_core::workspace::WorkspaceContext,
    config: &nyzhi_config::Config,
    mcp_tools: &[nyzhi_core::prompt::McpToolSummary],
    team_name: Option<&str>,
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
        trust: config.agent.trust.clone(),
        retry: config.agent.retry.clone(),
        routing: config.agent.routing.clone(),
        auto_compact_threshold: config.agent.auto_compact_threshold,
        team_name: team_name.map(String::from),
        agent_name: team_name.map(|_| "team-lead".to_string()),
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
        change_tracker: std::sync::Arc::new(tokio::sync::Mutex::new(
            nyzhi_core::tools::change_tracker::ChangeTracker::new(),
        )),
        allowed_tool_names: None,
        team_name: team_name.map(String::from),
        agent_name: team_name.map(|_| "team-lead".to_string()),
        is_team_lead: team_name.is_some(),
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
                AgentEvent::Retrying {
                    attempt,
                    max_retries,
                    wait_ms,
                    reason,
                } => {
                    eprintln!(
                        "\n[retry {attempt}/{max_retries}] waiting {wait_ms}ms: {reason}"
                    );
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
    let turn_start = std::time::Instant::now();

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

    let turn_elapsed = turn_start.elapsed();
    let notify = &config.tui.notify;
    if turn_elapsed.as_millis() as u64 >= notify.min_duration_ms {
        if notify.bell {
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::style::Print("\x07")
            );
        }
        if notify.desktop {
            let elapsed_str = format!("{:.1}s", turn_elapsed.as_secs_f64());
            let _ = notify_rust::Notification::new()
                .summary("nyzhi code")
                .body(&format!("Turn complete ({elapsed_str})"))
                .show();
        }
    }

    if !config.agent.hooks.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let results =
            nyzhi_core::hooks::run_after_turn_hooks(&config.agent.hooks, &cwd).await;
        for r in results {
            eprintln!("{}", r.summary());
        }
    }

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

fn prompt_api_key(provider: &str) -> anyhow::Result<()> {
    let display = nyzhi_config::find_provider_def(provider)
        .map(|d| d.name)
        .unwrap_or(provider);
    print!("Enter API key for {display}: ");
    use std::io::Write;
    std::io::stdout().flush()?;
    let mut key = String::new();
    std::io::stdin().read_line(&mut key)?;
    let key = key.trim();
    if key.is_empty() {
        eprintln!("No key entered.");
        return Ok(());
    }
    nyzhi_auth::token_store::store_api_key(provider, key)?;
    println!("API key saved for {display}.");
    Ok(())
}
