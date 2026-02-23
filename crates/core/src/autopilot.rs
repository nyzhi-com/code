use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutopilotPhase {
    Expansion,
    Planning,
    Execution,
    Qa,
    Validation,
    Complete,
    Cancelled,
}

impl std::fmt::Display for AutopilotPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutopilotPhase::Expansion => write!(f, "expansion"),
            AutopilotPhase::Planning => write!(f, "planning"),
            AutopilotPhase::Execution => write!(f, "execution"),
            AutopilotPhase::Qa => write!(f, "qa"),
            AutopilotPhase::Validation => write!(f, "validation"),
            AutopilotPhase::Complete => write!(f, "complete"),
            AutopilotPhase::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotState {
    pub idea: String,
    pub phase: AutopilotPhase,
    pub requirements: Option<String>,
    pub plan: Option<String>,
    pub execution_log: Vec<String>,
    pub qa_results: Vec<String>,
    pub validation_report: Option<String>,
}

impl AutopilotState {
    pub fn new(idea: &str) -> Self {
        Self {
            idea: idea.to_string(),
            phase: AutopilotPhase::Expansion,
            requirements: None,
            plan: None,
            execution_log: vec![],
            qa_results: vec![],
            validation_report: None,
        }
    }

    pub fn advance(&mut self) {
        self.phase = match self.phase {
            AutopilotPhase::Expansion => AutopilotPhase::Planning,
            AutopilotPhase::Planning => AutopilotPhase::Execution,
            AutopilotPhase::Execution => AutopilotPhase::Qa,
            AutopilotPhase::Qa => AutopilotPhase::Validation,
            AutopilotPhase::Validation => AutopilotPhase::Complete,
            AutopilotPhase::Complete | AutopilotPhase::Cancelled => AutopilotPhase::Complete,
        };
    }

    pub fn cancel(&mut self) {
        self.phase = AutopilotPhase::Cancelled;
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.phase,
            AutopilotPhase::Complete | AutopilotPhase::Cancelled
        )
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![];
        lines.push(format!("Autopilot: {}", self.idea));
        lines.push(format!("Phase: {}", self.phase));

        if let Some(reqs) = &self.requirements {
            let preview = if reqs.len() > 100 { &reqs[..100] } else { reqs };
            lines.push(format!("Requirements: {preview}..."));
        }

        if let Some(plan) = &self.plan {
            let preview = if plan.len() > 100 { &plan[..100] } else { plan };
            lines.push(format!("Plan: {preview}..."));
        }

        if !self.execution_log.is_empty() {
            lines.push(format!(
                "Execution log: {} entries",
                self.execution_log.len()
            ));
        }

        if !self.qa_results.is_empty() {
            lines.push(format!("QA results: {} cycles", self.qa_results.len()));
        }

        if let Some(report) = &self.validation_report {
            let preview = if report.len() > 100 {
                &report[..100]
            } else {
                report
            };
            lines.push(format!("Validation: {preview}..."));
        }

        lines.join("\n")
    }
}

fn state_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".nyzhi")
        .join("state")
        .join("autopilot.json")
}

pub fn save_state(project_root: &Path, state: &AutopilotState) -> Result<()> {
    let path = state_path(project_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn load_state(project_root: &Path) -> Result<Option<AutopilotState>> {
    let path = state_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let state: AutopilotState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

pub fn clear_state(project_root: &Path) -> Result<()> {
    let path = state_path(project_root);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn build_expansion_prompt(idea: &str) -> String {
    format!(
        "Analyze the following idea and expand it into detailed requirements and a technical spec:\n\n\
         {idea}\n\n\
         Output:\n\
         1. Functional requirements (user-facing behavior)\n\
         2. Technical requirements (architecture, data model, APIs)\n\
         3. Non-functional requirements (performance, security, UX)\n\
         4. Scope boundaries (what is NOT included)\n\
         5. Success criteria"
    )
}

pub fn build_planning_prompt(requirements: &str, idea: &str) -> String {
    format!(
        "Based on the following requirements, create a detailed implementation plan.\n\n\
         Original idea: {idea}\n\n\
         Requirements:\n{requirements}\n\n\
         Output a numbered list of implementation steps. Each step should:\n\
         - Be specific and actionable (file paths, function names, APIs)\n\
         - Have clear acceptance criteria\n\
         - List dependencies on other steps\n\
         - Estimate effort (small/medium/large)\n\n\
         Group steps into parallel execution waves where possible."
    )
}

pub fn build_execution_prompt(plan: &str, idea: &str) -> String {
    format!(
        "Execute the following implementation plan step by step. Do not re-plan or ask for confirmation.\n\n\
         Original idea: {idea}\n\n\
         Plan:\n{plan}\n\n\
         Rules:\n\
         - Work through each step in order.\n\
         - After completing each step, briefly report what you did.\n\
         - If a step fails, note the failure and continue with the next step.\n\
         - Run verification checks (tests, lint, type-check) after implementation.\n\
         - Do not stop until all steps are complete."
    )
}

pub fn build_qa_prompt(idea: &str) -> String {
    format!(
        "Run quality assurance checks on the implementation of: {idea}\n\n\
         Perform the following checks:\n\
         1. Run the project's test suite. Report failures.\n\
         2. Run the linter/formatter. Report violations.\n\
         3. Run type checks if applicable. Report errors.\n\
         4. Check for obvious regressions (broken imports, missing files, syntax errors).\n\
         5. Review edge cases and error handling.\n\n\
         Output a structured report:\n\
         - PASS: checks that passed\n\
         - FAIL: checks that failed with details\n\
         - FIX: immediate fixes you applied\n\
         - REMAINING: issues that need attention"
    )
}

pub fn build_validation_prompt(qa_results: &str, idea: &str) -> String {
    format!(
        "Validate the completed implementation of: {idea}\n\n\
         QA Results:\n{qa_results}\n\n\
         Perform final validation:\n\
         1. Verify all QA failures have been addressed.\n\
         2. Check that the original success criteria are met.\n\
         3. Verify no unintended side effects were introduced.\n\
         4. Confirm the implementation matches the original requirements.\n\n\
         Output a final verdict: PASS (ready to ship) or FAIL (with remaining issues)."
    )
}
