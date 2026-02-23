use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::agent::AgentEvent;
use crate::tools::permission::ToolPermission;

pub struct AskUserTool;

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Present a multiple-choice question to the user and wait for their selection. \
         Use when you need a decision, preference, or clarification that cannot be \
         resolved by reading the codebase. The user can also type a custom response."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to present to the user."
                },
                "options": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "value": { "type": "string", "description": "Machine-readable value returned when selected." },
                            "label": { "type": "string", "description": "Human-readable label shown to the user." }
                        },
                        "required": ["value", "label"]
                    },
                    "minItems": 2,
                    "maxItems": 6,
                    "description": "2-6 options for the user to choose from."
                },
                "allow_custom": {
                    "type": "boolean",
                    "description": "If true, show a 'Custom...' option for free-form input. Default: true."
                }
            },
            "required": ["question", "options"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let question = args
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("Please choose an option:")
            .to_string();

        let options: Vec<(String, String)> = args
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let value = item.get("value")?.as_str()?.to_string();
                        let label = item.get("label")?.as_str()?.to_string();
                        Some((value, label))
                    })
                    .collect()
            })
            .unwrap_or_default();

        if options.len() < 2 {
            return Ok(ToolResult {
                output: "Error: at least 2 options are required.".to_string(),
                title: "ask_user".to_string(),
                metadata: json!({ "error": true }),
            });
        }

        let allow_custom = args
            .get("allow_custom")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let event_tx = ctx
            .event_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No event channel available"))?;

        let (tx, rx) = tokio::sync::oneshot::channel::<String>();
        let respond = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));

        let _ = event_tx.send(AgentEvent::UserQuestion {
            question: question.clone(),
            options: options.clone(),
            allow_custom,
            respond,
        });

        let selected = rx
            .await
            .unwrap_or_else(|_| "__cancelled__".to_string());

        if selected == "__cancelled__" {
            Ok(ToolResult {
                output: "User dismissed the question without selecting an option.".to_string(),
                title: "ask_user".to_string(),
                metadata: json!({ "cancelled": true }),
            })
        } else {
            let label = options
                .iter()
                .find(|(v, _)| v == &selected)
                .map(|(_, l)| l.as_str())
                .unwrap_or(&selected);

            Ok(ToolResult {
                output: format!("User selected: {}", selected),
                title: "ask_user".to_string(),
                metadata: json!({ "selected_value": selected, "selected_label": label }),
            })
        }
    }
}
