use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};

pub struct LoadSkillTool;

#[async_trait]
impl Tool for LoadSkillTool {
    fn name(&self) -> &str {
        "load_skill"
    }

    fn description(&self) -> &str {
        "Load the full content of a skill by name. Skills are domain-specific instructions \
         stored in .nyzhi/skills/ or .claude/skills/. Use this when you need detailed \
         guidance for a specific task."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name of the skill to load (e.g. 'coding-standards', 'testing-patterns')."
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;

        let skills = crate::skills::load_skills(&ctx.project_root).unwrap_or_default();
        if let Some(skill) = skills.iter().find(|s| s.name == name) {
            Ok(ToolResult {
                output: format!("# Skill: {}\n\n{}", skill.name, skill.content),
                title: format!("load_skill({name})"),
                metadata: serde_json::json!({
                    "skill_name": skill.name,
                    "path": skill.path.display().to_string(),
                }),
            })
        } else {
            let available: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
            Ok(ToolResult {
                output: format!(
                    "Skill '{name}' not found. Available skills: {}",
                    if available.is_empty() {
                        "(none)".to_string()
                    } else {
                        available.join(", ")
                    }
                ),
                title: format!("load_skill({name}) - not found"),
                metadata: serde_json::json!({"error": "not_found"}),
            })
        }
    }
}
