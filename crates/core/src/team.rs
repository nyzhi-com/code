use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Executor,
    Reviewer,
    Tester,
    Coordinator,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Executor => write!(f, "executor"),
            AgentRole::Reviewer => write!(f, "reviewer"),
            AgentRole::Tester => write!(f, "tester"),
            AgentRole::Coordinator => write!(f, "coordinator"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: u32,
    pub role: AgentRole,
    pub task: String,
    pub status: TeamMemberStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamMemberStatus {
    Pending,
    Active,
    Done,
    Failed,
}

impl std::fmt::Display for TeamMemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamMemberStatus::Pending => write!(f, "pending"),
            TeamMemberStatus::Active => write!(f, "active"),
            TeamMemberStatus::Done => write!(f, "done"),
            TeamMemberStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TeamConfig {
    pub team_size: u32,
    pub task: String,
}

#[derive(Debug, Clone)]
pub struct TeamState {
    pub members: Vec<TeamMember>,
    pub phase: TeamPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamPhase {
    Planning,
    Assigning,
    Executing,
    Verifying,
    Fixing,
    Complete,
}

impl std::fmt::Display for TeamPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamPhase::Planning => write!(f, "planning"),
            TeamPhase::Assigning => write!(f, "assigning"),
            TeamPhase::Executing => write!(f, "executing"),
            TeamPhase::Verifying => write!(f, "verifying"),
            TeamPhase::Fixing => write!(f, "fixing"),
            TeamPhase::Complete => write!(f, "complete"),
        }
    }
}

impl TeamState {
    pub fn new(config: &TeamConfig) -> Self {
        let mut members = vec![];

        members.push(TeamMember {
            id: 0,
            role: AgentRole::Coordinator,
            task: format!("Coordinate: {}", config.task),
            status: TeamMemberStatus::Pending,
        });

        for i in 1..=config.team_size.saturating_sub(2) {
            members.push(TeamMember {
                id: i,
                role: AgentRole::Executor,
                task: String::new(),
                status: TeamMemberStatus::Pending,
            });
        }

        if config.team_size >= 2 {
            members.push(TeamMember {
                id: config.team_size - 1,
                role: AgentRole::Reviewer,
                task: "Review and verify implementation".to_string(),
                status: TeamMemberStatus::Pending,
            });
        }

        Self {
            members,
            phase: TeamPhase::Planning,
        }
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![format!("Team phase: {}", self.phase)];
        for m in &self.members {
            let task_preview = if m.task.len() > 60 {
                format!("{}...", &m.task[..60])
            } else {
                m.task.clone()
            };
            lines.push(format!(
                "  [{}] Agent #{}: {} - {}",
                m.status, m.id, m.role, task_preview,
            ));
        }
        lines.join("\n")
    }

    pub fn all_done(&self) -> bool {
        self.members.iter().all(|m| m.status == TeamMemberStatus::Done)
    }
}

pub fn build_executor_prompt(task: &str, role: &AgentRole) -> String {
    match role {
        AgentRole::Executor => format!(
            "You are an executor agent. Implement the following task thoroughly:\n\n{task}"
        ),
        AgentRole::Reviewer => format!(
            "You are a code reviewer. Review the changes for bugs, security issues, and improvements:\n\n{task}"
        ),
        AgentRole::Tester => format!(
            "You are a test engineer. Write comprehensive tests for:\n\n{task}"
        ),
        AgentRole::Coordinator => format!(
            "You are the coordinator. Plan, delegate, and verify the following:\n\n{task}"
        ),
    }
}
