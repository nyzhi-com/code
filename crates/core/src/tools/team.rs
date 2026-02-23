use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

pub struct TeamCreateTool;

#[async_trait]
impl Tool for TeamCreateTool {
    fn name(&self) -> &str {
        "team_create"
    }
    fn description(&self) -> &str {
        "Create a new agent team. Sets up config, inboxes, and shared task board. \
         The first member is automatically the team lead."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Team name (e.g. 'refactor-team')." },
                "members": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "role": { "type": "string" }
                        },
                        "required": ["name"]
                    },
                    "description": "Team members. First member is the lead."
                }
            },
            "required": ["name", "members"]
        })
    }
    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: name"))?;
        let members_val = args
            .get("members")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing: members"))?;

        let mut members = Vec::new();
        for (i, m) in members_val.iter().enumerate() {
            let mname = m.get("name").and_then(|v| v.as_str()).unwrap_or("agent");
            let role = m
                .get("role")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let agent_type = if i == 0 { "leader" } else { "general-purpose" };
            members.push(crate::teams::config::TeamMemberConfig {
                name: mname.to_string(),
                agent_id: None,
                agent_type: agent_type.to_string(),
                color: crate::teams::config::assign_color(i),
                model: None,
                role,
                worktree_path: None,
            });
        }

        let config = crate::teams::config::TeamConfig::create(name, members)?;

        Ok(ToolResult {
            output: format!(
                "Team '{}' created with {} members. Lead: '{}'.",
                name,
                config.members.len(),
                config.lead_name()
            ),
            title: format!("team_create({name})"),
            metadata: serde_json::to_value(&config).unwrap_or_default(),
        })
    }
}

pub struct TeamDeleteTool;

#[async_trait]
impl Tool for TeamDeleteTool {
    fn name(&self) -> &str {
        "team_delete"
    }
    fn description(&self) -> &str {
        "Delete a team and all its artifacts (config, inboxes, tasks)."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        })
    }
    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: name"))?;
        crate::teams::config::TeamConfig::delete(name)?;
        Ok(ToolResult {
            output: format!("Team '{name}' deleted."),
            title: format!("team_delete({name})"),
            metadata: serde_json::json!({"deleted": true}),
        })
    }
}

pub struct SendMessageTool;

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "send_team_message"
    }
    fn description(&self) -> &str {
        "Send a message to a teammate or broadcast to the entire team. \
         Uses the current agent's identity as the sender."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string" },
                "to": { "type": "string", "description": "Recipient name, or 'all' for broadcast." },
                "text": { "type": "string" }
            },
            "required": ["team", "to", "text"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let to = args
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: to"))?;
        let text = args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: text"))?;

        let from = ctx.agent_name.as_deref().unwrap_or("team-lead");
        let config = crate::teams::config::TeamConfig::load(team)?;
        let color = config
            .members
            .iter()
            .find(|m| m.name == from)
            .map(|m| m.color.as_str());

        if to == "all" {
            crate::teams::mailbox::broadcast(team, from, text, color)?;
            Ok(ToolResult {
                output: format!("Broadcast sent to team '{team}' from '{from}'."),
                title: format!("broadcast({team})"),
                metadata: serde_json::json!({"broadcast": true, "from": from}),
            })
        } else {
            let msg = crate::teams::mailbox::TeamMessage::new(from, text, color);
            crate::teams::mailbox::send_message(team, to, msg)?;
            Ok(ToolResult {
                output: format!("Message sent to '{to}' in team '{team}' from '{from}'."),
                title: format!("send_message({team}, {to})"),
                metadata: serde_json::json!({"to": to, "from": from}),
            })
        }
    }
}

pub struct TaskCreateTool;

#[async_trait]
impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "task_create"
    }
    fn description(&self) -> &str {
        "Create a task on the team's shared task board. Uses auto-incrementing IDs."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string" },
                "subject": { "type": "string", "description": "Imperative-form title (e.g. 'Run tests')." },
                "description": { "type": "string" },
                "active_form": { "type": "string", "description": "Present-continuous form for display (e.g. 'Running tests')." },
                "blocked_by": { "type": "array", "items": { "type": "string" }, "description": "Task IDs this task depends on." }
            },
            "required": ["team", "subject"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let subject = args
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: subject"))?;
        let description = args.get("description").and_then(|v| v.as_str());
        let active_form = args.get("active_form").and_then(|v| v.as_str());
        let blocked_by: Vec<String> = args
            .get("blocked_by")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let task = crate::teams::tasks::TeamTask::create(
            team,
            subject,
            description,
            active_form,
            blocked_by,
        )?;
        Ok(ToolResult {
            output: format!("Task #{} created: '{}'.", task.id, task.subject),
            title: format!("task_create({team}, #{})", task.id),
            metadata: serde_json::to_value(&task).unwrap_or_default(),
        })
    }
}

pub struct TaskUpdateTool;

#[async_trait]
impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "task_update"
    }
    fn description(&self) -> &str {
        "Update a task's status or owner. Completing a task auto-unblocks dependents."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string" },
                "task_id": { "type": "string" },
                "status": { "type": "string", "enum": ["pending", "in_progress", "completed", "blocked", "deleted"] },
                "owner": { "type": "string" }
            },
            "required": ["team", "task_id"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let task_id = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: task_id"))?;
        let status = args
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "in_progress" => crate::teams::tasks::TaskStatus::InProgress,
                "completed" => crate::teams::tasks::TaskStatus::Completed,
                "blocked" => crate::teams::tasks::TaskStatus::Blocked,
                "deleted" => crate::teams::tasks::TaskStatus::Deleted,
                _ => crate::teams::tasks::TaskStatus::Pending,
            });
        let owner = args.get("owner").and_then(|v| v.as_str()).map(String::from);

        let task = crate::teams::tasks::TeamTask::update(team, task_id, status, owner)?;
        Ok(ToolResult {
            output: format!(
                "Task #{} updated: status={}, owner={}.",
                task.id,
                task.status,
                task.owner.as_deref().unwrap_or("unassigned")
            ),
            title: format!("task_update({team}, #{task_id})"),
            metadata: serde_json::to_value(&task).unwrap_or_default(),
        })
    }
}

pub struct TaskListTool;

#[async_trait]
impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "task_list"
    }
    fn description(&self) -> &str {
        "List tasks on the team's shared board with optional status filter."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string" },
                "filter": { "type": "string", "enum": ["pending", "in_progress", "completed", "blocked"] }
            },
            "required": ["team"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let filter = args.get("filter").and_then(|v| v.as_str());

        let tasks = crate::teams::tasks::list_tasks(team, filter)?;
        if tasks.is_empty() {
            return Ok(ToolResult {
                output: format!(
                    "No tasks{} in team '{team}'.",
                    filter
                        .map(|f| format!(" with status '{f}'"))
                        .unwrap_or_default()
                ),
                title: format!("task_list({team})"),
                metadata: serde_json::json!({"count": 0}),
            });
        }

        let mut out = format!("Tasks in team '{team}':\n\n");
        for t in &tasks {
            let owner = t.owner.as_deref().unwrap_or("unassigned");
            let blocked = if t.blocked_by.is_empty() {
                String::new()
            } else {
                format!(" [blocked by: {}]", t.blocked_by.join(", "))
            };
            out.push_str(&format!(
                "- [{}] #{} ({}){} -- {}\n",
                t.status, t.id, owner, blocked, t.subject
            ));
        }

        Ok(ToolResult {
            output: out,
            title: format!("task_list({team})"),
            metadata: serde_json::json!({"count": tasks.len()}),
        })
    }
}

use crate::agent::AgentConfig;
use crate::agent_manager::AgentManager;
use std::sync::Arc;

pub struct SpawnTeammateTool {
    manager: Arc<AgentManager>,
}

impl SpawnTeammateTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for SpawnTeammateTool {
    fn name(&self) -> &str {
        "spawn_teammate"
    }
    fn description(&self) -> &str {
        "Spawn a new agent and register it as a team member. The agent starts \
         working immediately with the given message as its initial prompt. \
         Each teammate gets its own context window and can message other teammates."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string", "description": "Team name." },
                "name": { "type": "string", "description": "Agent name within the team." },
                "message": { "type": "string", "description": "Initial task/prompt for the agent." },
                "role": { "type": "string", "description": "Optional specialist role." },
                "plan_mode": { "type": "boolean", "description": "If true, teammate starts in read-only plan mode." }
            },
            "required": ["team", "name", "message"]
        })
    }
    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: name"))?;
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: message"))?;
        let role = args.get("role").and_then(|v| v.as_str()).map(String::from);

        let mut config = crate::teams::config::TeamConfig::load(team)?;
        let color = crate::teams::config::assign_color(config.members.len());

        let agent_config = AgentConfig {
            name: format!("teammate/{name}"),
            system_prompt: format!(
                "You are '{}', a teammate in team '{}'. You have your own context window. \
                 Use send_team_message to communicate with other teammates or the lead. \
                 Use task_list to see available work and task_update to claim/complete tasks. \
                 Focus on your assigned work and report findings via messages.",
                name, team
            ),
            max_steps: 100,
            team_name: Some(team.to_string()),
            agent_name: Some(name.to_string()),
            ..AgentConfig::default()
        };

        match self
            .manager
            .spawn_agent(
                message.to_string(),
                role.clone(),
                ctx.depth,
                ctx,
                agent_config,
                None,
            )
            .await
        {
            Ok((agent_id, nickname)) => {
                config.add_member(crate::teams::config::TeamMemberConfig {
                    name: name.to_string(),
                    agent_id: Some(agent_id.clone()),
                    agent_type: "general-purpose".to_string(),
                    color: color.clone(),
                    model: None,
                    role: role.clone(),
                    worktree_path: None,
                })?;

                Ok(ToolResult {
                    output: format!(
                        "Teammate '{name}' (nickname: {nickname}) spawned in team '{team}' \
                         and is now working on the assigned task.",
                    ),
                    title: format!("spawn_teammate({team}, {name})"),
                    metadata: serde_json::json!({
                        "team": team,
                        "name": name,
                        "agent_id": agent_id,
                        "nickname": nickname,
                        "color": color,
                        "role": role,
                    }),
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error spawning teammate '{name}': {e}"),
                title: format!("spawn_teammate({team}, {name}) error"),
                metadata: serde_json::json!({"error": e.to_string()}),
            }),
        }
    }
}

pub struct TeamListTool;

#[async_trait]
impl Tool for TeamListTool {
    fn name(&self) -> &str {
        "team_list"
    }
    fn description(&self) -> &str {
        "List all existing agent teams."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let teams = crate::teams::list_teams();
        if teams.is_empty() {
            return Ok(ToolResult {
                output: "No teams found.".to_string(),
                title: "team_list".to_string(),
                metadata: serde_json::json!({"count": 0}),
            });
        }

        let mut out = String::from("Active teams:\n\n");
        for name in &teams {
            if let Ok(config) = crate::teams::config::TeamConfig::load(name) {
                let member_names: Vec<&str> =
                    config.members.iter().map(|m| m.name.as_str()).collect();
                out.push_str(&format!(
                    "- {} ({} members: {})\n",
                    name,
                    config.members.len(),
                    member_names.join(", ")
                ));
            } else {
                out.push_str(&format!("- {} (config error)\n", name));
            }
        }

        Ok(ToolResult {
            output: out,
            title: "team_list".to_string(),
            metadata: serde_json::json!({"count": teams.len(), "teams": teams}),
        })
    }
}

pub struct ReadInboxTool;

#[async_trait]
impl Tool for ReadInboxTool {
    fn name(&self) -> &str {
        "read_inbox"
    }
    fn description(&self) -> &str {
        "Read unread messages from this agent's team inbox. Messages are \
         auto-injected at each turn, but this tool lets you check explicitly."
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "team": { "type": "string" }
            },
            "required": ["team"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let team = args
            .get("team")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: team"))?;
        let agent_name = ctx.agent_name.as_deref().unwrap_or("team-lead");

        let messages = crate::teams::mailbox::read_unread(team, agent_name)?;
        if messages.is_empty() {
            return Ok(ToolResult {
                output: "No unread messages.".to_string(),
                title: format!("read_inbox({team})"),
                metadata: serde_json::json!({"count": 0}),
            });
        }

        let formatted = crate::teams::mailbox::format_messages_for_injection(&messages);
        Ok(ToolResult {
            output: format!("{} unread message(s):\n\n{}", messages.len(), formatted),
            title: format!("read_inbox({team})"),
            metadata: serde_json::json!({"count": messages.len()}),
        })
    }
}
