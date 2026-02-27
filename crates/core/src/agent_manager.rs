use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::Result;
use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot, watch, Mutex};
use tokio::task::JoinHandle;

use crate::agent::{run_turn, AgentConfig, AgentEvent, SessionUsage};
use crate::conversation::Thread;
use crate::tools::{ToolContext, ToolRegistry};

pub type AgentId = String;

const AGENT_NAMES: &[&str] = &[
    "Pikachu",
    "Charizard",
    "Bulbasaur",
    "Squirtle",
    "Eevee",
    "Gengar",
    "Mewtwo",
    "Snorlax",
    "Dragonite",
    "Alakazam",
    "Gyarados",
    "Arcanine",
    "Lucario",
    "Gardevoir",
    "Blaziken",
    "Greninja",
    "Umbreon",
    "Espeon",
    "Jolteon",
    "Vaporeon",
    "Flareon",
    "Leafeon",
    "Glaceon",
    "Sylveon",
    "Typhlosion",
    "Feraligatr",
    "Meganium",
    "Scizor",
    "Tyranitar",
    "Heracross",
    "Ampharos",
    "Togekiss",
    "Salamence",
    "Metagross",
    "Absol",
    "Flygon",
    "Milotic",
    "Aggron",
    "Swampert",
    "Sceptile",
    "Luxray",
    "Staraptor",
    "Garchomp",
    "Gallade",
    "Weavile",
    "Electivire",
    "Magmortar",
    "Infernape",
    "Empoleon",
    "Torterra",
    "Zoroark",
    "Hydreigon",
    "Volcarona",
    "Haxorus",
    "Krookodile",
    "Chandelure",
    "Excadrill",
    "Bisharp",
    "Braviary",
    "Golurk",
    "Serperior",
    "Samurott",
    "Emboar",
    "Noivern",
    "Talonflame",
    "Hawlucha",
    "Goodra",
    "Aegislash",
    "Dragalge",
    "Pangoro",
    "Decidueye",
    "Incineroar",
    "Primarina",
    "Mimikyu",
    "Toxapex",
    "Golisopod",
    "Kommo",
    "Lycanroc",
    "Corviknight",
    "Dragapult",
    "Grimmsnarl",
    "Cinderace",
    "Rillaboom",
    "Toxtricity",
    "Urshifu",
    "Ceruledge",
    "Kingambit",
    "Baxcalibur",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    PendingInit,
    Running,
    Completed(Option<String>),
    Errored(String),
    Shutdown,
    NotFound,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::PendingInit => write!(f, "pending_init"),
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Completed(msg) => {
                if let Some(m) = msg {
                    let preview = if m.len() > 100 { &m[..100] } else { m };
                    write!(f, "completed: {preview}")
                } else {
                    write!(f, "completed")
                }
            }
            AgentStatus::Errored(e) => write!(f, "errored: {e}"),
            AgentStatus::Shutdown => write!(f, "shutdown"),
            AgentStatus::NotFound => write!(f, "not_found"),
        }
    }
}

impl AgentStatus {
    pub fn is_final(&self) -> bool {
        !matches!(self, AgentStatus::PendingInit | AgentStatus::Running)
    }
}

struct AgentHandle {
    pub nickname: String,
    pub role: Option<String>,
    #[allow(dead_code)]
    pub depth: u32,
    pub status_tx: watch::Sender<AgentStatus>,
    pub status_rx: watch::Receiver<AgentStatus>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
    pub join_handle: Option<JoinHandle<()>>,
    pub thread: Arc<Mutex<Thread>>,
}

#[derive(Default)]
struct NicknamePool {
    used: HashSet<String>,
}

impl NicknamePool {
    fn reserve(&mut self) -> String {
        let available: Vec<&&str> = AGENT_NAMES
            .iter()
            .filter(|n| !self.used.contains(**n))
            .collect();
        let name = if let Some(&&chosen) = available.choose(&mut rand::rng()) {
            chosen.to_string()
        } else {
            self.used.clear();
            AGENT_NAMES
                .choose(&mut rand::rng())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Agent".to_string())
        };
        self.used.insert(name.clone());
        name
    }

    fn release(&mut self, name: &str) {
        self.used.remove(name);
    }
}

struct AgentGuards {
    active_count: AtomicUsize,
    max_threads: usize,
    max_depth: u32,
}

impl AgentGuards {
    fn new(max_threads: usize, max_depth: u32) -> Self {
        Self {
            active_count: AtomicUsize::new(0),
            max_threads,
            max_depth,
        }
    }

    fn try_reserve(&self) -> bool {
        let mut current = self.active_count.load(Ordering::Acquire);
        loop {
            if current >= self.max_threads {
                return false;
            }
            match self.active_count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(updated) => current = updated,
            }
        }
    }

    fn release(&self) {
        self.active_count.fetch_sub(1, Ordering::AcqRel);
    }

    fn exceeds_depth(&self, depth: u32) -> bool {
        depth > self.max_depth
    }
}

pub struct AgentManager {
    agents: Arc<Mutex<HashMap<AgentId, AgentHandle>>>,
    guards: Arc<AgentGuards>,
    nicknames: Arc<Mutex<NicknamePool>>,
    provider: Arc<dyn nyzhi_provider::Provider>,
    registry: Arc<ToolRegistry>,
    parent_event_tx: broadcast::Sender<AgentEvent>,
}

impl AgentManager {
    pub fn new(
        provider: Arc<dyn nyzhi_provider::Provider>,
        registry: Arc<ToolRegistry>,
        parent_event_tx: broadcast::Sender<AgentEvent>,
        max_threads: usize,
        max_depth: u32,
    ) -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            guards: Arc::new(AgentGuards::new(max_threads, max_depth)),
            nicknames: Arc::new(Mutex::new(NicknamePool::default())),
            provider,
            registry,
            parent_event_tx,
        }
    }

    pub async fn spawn_agent(
        &self,
        prompt: String,
        role: Option<String>,
        parent_depth: u32,
        parent_ctx: &ToolContext,
        agent_config: AgentConfig,
        tool_filter: Option<Vec<String>>,
    ) -> Result<(AgentId, String)> {
        let child_depth = parent_depth + 1;
        if self.guards.exceeds_depth(child_depth) {
            anyhow::bail!(
                "Agent depth limit ({}) reached. Solve the task yourself.",
                self.guards.max_depth
            );
        }
        if !self.guards.try_reserve() {
            anyhow::bail!(
                "Agent limit ({}) reached. Wait for existing agents to complete or close them first.",
                self.guards.max_threads
            );
        }

        let agent_id = uuid::Uuid::new_v4().to_string();
        let nickname = self.nicknames.lock().await.reserve();

        let (status_tx, status_rx) = watch::channel(AgentStatus::PendingInit);
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        let thread = Arc::new(Mutex::new(Thread::new()));

        let handle = AgentHandle {
            nickname: nickname.clone(),
            role: role.clone(),
            depth: child_depth,
            status_tx: status_tx.clone(),
            status_rx: status_rx.clone(),
            cancel_tx: Some(cancel_tx),
            join_handle: None,
            thread: thread.clone(),
        };

        self.agents.lock().await.insert(agent_id.clone(), handle);

        let _ = self.parent_event_tx.send(AgentEvent::SubAgentSpawned {
            id: agent_id.clone(),
            nickname: nickname.clone(),
            role: role.clone(),
        });

        let provider = self.provider.clone();
        let registry = self.registry.clone();
        let guards = self.guards.clone();
        let parent_event_tx = self.parent_event_tx.clone();
        let id_clone = agent_id.clone();
        let nick_clone = nickname.clone();

        let child_ctx = ToolContext {
            session_id: parent_ctx.session_id.clone(),
            cwd: parent_ctx.cwd.clone(),
            project_root: parent_ctx.project_root.clone(),
            depth: child_depth,
            event_tx: None,
            change_tracker: parent_ctx.change_tracker.clone(),
            allowed_tool_names: tool_filter,
            team_name: agent_config.team_name.clone(),
            agent_name: agent_config.agent_name.clone(),
            is_team_lead: false,
            todo_store: parent_ctx.todo_store.clone(),
            index: parent_ctx.index.clone(),
            sandbox_level: parent_ctx.sandbox_level,
            subagent_model_overrides: parent_ctx.subagent_model_overrides.clone(),
            shared_context: parent_ctx.shared_context.clone(),
        };

        let join_handle = tokio::spawn(async move {
            let (child_event_tx, mut child_event_rx) = broadcast::channel::<AgentEvent>(256);

            let child_ctx = ToolContext {
                event_tx: Some(child_event_tx.clone()),
                ..child_ctx
            };

            let _ = status_tx.send(AgentStatus::Running);
            let _ = parent_event_tx.send(AgentEvent::SubAgentStatusChanged {
                id: id_clone.clone(),
                nickname: nick_clone.clone(),
                status: "running".to_string(),
            });

            let fwd_parent_tx = parent_event_tx.clone();
            let fwd_nick = nick_clone.clone();
            let fwd_id = id_clone.clone();
            let forward_handle = tokio::spawn(async move {
                loop {
                    match child_event_rx.recv().await {
                        Ok(event) => {
                            let forwarded = match event {
                                AgentEvent::TextDelta(text) => {
                                    AgentEvent::TextDelta(format!("[{}] {}", fwd_nick, text))
                                }
                                AgentEvent::ToolCallStart { id, name } => {
                                    AgentEvent::ToolCallStart {
                                        id,
                                        name: format!("[{}] {}", fwd_nick, name),
                                    }
                                }
                                AgentEvent::ToolCallDone {
                                    id,
                                    name,
                                    output,
                                    elapsed_ms,
                                } => AgentEvent::ToolCallDone {
                                    id,
                                    name: format!("[{}] {}", fwd_nick, name),
                                    output,
                                    elapsed_ms,
                                },
                                AgentEvent::TurnComplete => break,
                                other => other,
                            };
                            let _ = fwd_parent_tx.send(forwarded);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            });

            let mut child_thread = thread.lock().await;
            let mut session_usage = SessionUsage::default();

            let result = tokio::select! {
                r = run_turn(
                    &*provider,
                    &mut child_thread,
                    &prompt,
                    &agent_config,
                    &child_event_tx,
                    &registry,
                    &child_ctx,
                    None,
                    &mut session_usage,
                ) => r,
                _ = cancel_rx => {
                    Err(anyhow::anyhow!("Agent cancelled"))
                }
            };

            let final_status = match &result {
                Ok(()) => {
                    let final_text = child_thread
                        .messages()
                        .iter()
                        .rev()
                        .find(|m| m.role == nyzhi_provider::Role::Assistant)
                        .map(|m| m.content.as_text().to_string());
                    AgentStatus::Completed(final_text)
                }
                Err(e) => AgentStatus::Errored(e.to_string()),
            };

            let _ = child_event_tx.send(AgentEvent::TurnComplete);
            let _ = forward_handle.await;

            let _ = status_tx.send(final_status.clone());
            let _ = parent_event_tx.send(AgentEvent::SubAgentCompleted {
                id: fwd_id.clone(),
                nickname: nick_clone.clone(),
                final_message: match &final_status {
                    AgentStatus::Completed(msg) => msg.clone(),
                    AgentStatus::Errored(e) => Some(format!("Error: {e}")),
                    _ => None,
                },
            });
            let _ = parent_event_tx.send(AgentEvent::SubAgentStatusChanged {
                id: fwd_id,
                nickname: nick_clone,
                status: final_status.to_string(),
            });

            guards.release();
        });

        if let Some(handle) = self.agents.lock().await.get_mut(&agent_id) {
            handle.join_handle = Some(join_handle);
        }

        Ok((agent_id, nickname))
    }

    pub async fn send_input(&self, agent_id: &str, message: String) -> Result<()> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent with id {agent_id} not found"))?;

        let status = handle.status_rx.borrow().clone();
        if status.is_final() {
            anyhow::bail!(
                "Agent {agent_id} ({}) is in final state: {status}",
                handle.nickname
            );
        }

        let mut thread = handle.thread.lock().await;
        thread.push_message(nyzhi_provider::Message {
            role: nyzhi_provider::Role::User,
            content: nyzhi_provider::MessageContent::Text(message),
        });

        Ok(())
    }

    pub async fn get_status(&self, agent_id: &str) -> AgentStatus {
        let agents = self.agents.lock().await;
        match agents.get(agent_id) {
            Some(handle) => handle.status_rx.borrow().clone(),
            None => AgentStatus::NotFound,
        }
    }

    pub async fn subscribe_status(&self, agent_id: &str) -> Result<watch::Receiver<AgentStatus>> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent with id {agent_id} not found"))?;
        Ok(handle.status_rx.clone())
    }

    pub async fn shutdown_agent(&self, agent_id: &str) -> Result<AgentStatus> {
        let mut agents = self.agents.lock().await;
        let handle = agents
            .get_mut(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent with id {agent_id} not found"))?;

        if let Some(cancel_tx) = handle.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        let _ = handle.status_tx.send(AgentStatus::Shutdown);
        let status = handle.status_rx.borrow().clone();

        let nickname = handle.nickname.clone();
        self.nicknames.lock().await.release(&nickname);
        self.guards.release();

        Ok(status)
    }

    pub async fn wait_any(
        &self,
        agent_ids: &[String],
        timeout_ms: i64,
    ) -> Result<(HashMap<String, AgentStatus>, bool)> {
        if agent_ids.is_empty() {
            anyhow::bail!("ids must be non-empty");
        }

        let timeout_ms = timeout_ms.clamp(10_000, 300_000);

        let mut status_receivers: Vec<(String, watch::Receiver<AgentStatus>)> = Vec::new();
        let mut immediate_finals: HashMap<String, AgentStatus> = HashMap::new();

        {
            let agents = self.agents.lock().await;
            for id in agent_ids {
                match agents.get(id) {
                    Some(handle) => {
                        let status = handle.status_rx.borrow().clone();
                        if status.is_final() {
                            immediate_finals.insert(id.clone(), status);
                        } else {
                            status_receivers.push((id.clone(), handle.status_rx.clone()));
                        }
                    }
                    None => {
                        immediate_finals.insert(id.clone(), AgentStatus::NotFound);
                    }
                }
            }
        }

        if !immediate_finals.is_empty() {
            return Ok((immediate_finals, false));
        }

        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64);

        let mut results: HashMap<String, AgentStatus> = HashMap::new();

        let mut futures: futures::stream::FuturesUnordered<_> = status_receivers
            .into_iter()
            .map(move |(id, mut rx)| async move {
                loop {
                    let status = rx.borrow().clone();
                    if status.is_final() {
                        return Some((id, status));
                    }
                    match tokio::time::timeout_at(deadline, rx.changed()).await {
                        Ok(Ok(())) => {
                            let s = rx.borrow().clone();
                            if s.is_final() {
                                return Some((id, s));
                            }
                        }
                        Ok(Err(_)) => {
                            return Some((id, rx.borrow().clone()));
                        }
                        Err(_) => return None,
                    }
                }
            })
            .collect();

        use futures::StreamExt;
        while let Some(result) = futures.next().await {
            if let Some((id, status)) = result {
                results.insert(id, status);
                break;
            }
        }

        let timed_out = results.is_empty();
        Ok((results, timed_out))
    }

    pub async fn get_agent_info(&self, agent_id: &str) -> Option<(String, Option<String>)> {
        let agents = self.agents.lock().await;
        agents
            .get(agent_id)
            .map(|h| (h.nickname.clone(), h.role.clone()))
    }

    pub async fn resume_agent(&self, agent_id: &str) -> Result<AgentStatus> {
        let agents = self.agents.lock().await;
        let handle = agents
            .get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent with id {agent_id} not found"))?;

        let status = handle.status_rx.borrow().clone();
        if !status.is_final() {
            return Ok(status);
        }

        let _ = handle.status_tx.send(AgentStatus::Running);
        Ok(AgentStatus::Running)
    }
}
