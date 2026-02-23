use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelTier::Low => write!(f, "low"),
            ModelTier::Medium => write!(f, "medium"),
            ModelTier::High => write!(f, "high"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThinkingSupport {
    /// OpenAI: reasoning.effort = "low" | "medium" | "high" | "xhigh"
    ReasoningEffort {
        levels: Vec<String>,
        default: String,
    },
    /// Anthropic Opus 4.6: adaptive thinking with effort levels
    AdaptiveEffort {
        levels: Vec<String>,
        default: String,
    },
    /// Anthropic (non-Opus 4.6): budget_tokens in thinking block
    BudgetTokens { max: u32, default: u32 },
    /// Gemini: thinking_level or thinking_budget
    ThinkingLevel {
        levels: Vec<String>,
        default: String,
    },
}

impl ThinkingSupport {
    pub fn openai_reasoning() -> Self {
        ThinkingSupport::ReasoningEffort {
            levels: vec!["low".into(), "medium".into(), "high".into(), "xhigh".into()],
            default: "medium".into(),
        }
    }

    pub fn anthropic_adaptive() -> Self {
        ThinkingSupport::AdaptiveEffort {
            levels: vec!["low".into(), "medium".into(), "high".into(), "max".into()],
            default: "high".into(),
        }
    }

    pub fn anthropic_budget(max: u32) -> Self {
        ThinkingSupport::BudgetTokens {
            max,
            default: max / 2,
        }
    }

    pub fn kimi_thinking() -> Self {
        ThinkingSupport::ReasoningEffort {
            levels: vec!["on".into()],
            default: "on".into(),
        }
    }

    pub fn gemini_levels(levels: &[&str]) -> Self {
        let lvls: Vec<String> = levels.iter().map(|s| s.to_string()).collect();
        let default = lvls
            .get(lvls.len() / 2)
            .cloned()
            .unwrap_or_else(|| "medium".into());
        ThinkingSupport::ThinkingLevel {
            levels: lvls,
            default,
        }
    }

    pub fn user_facing_levels(&self) -> Vec<(&str, &str)> {
        match self {
            ThinkingSupport::ReasoningEffort { levels, .. } => {
                if levels.len() == 1 && levels[0] == "on" {
                    return vec![("off", "Thinking off"), ("on", "Thinking on")];
                }
                let mut result = vec![("off", "No reasoning")];
                for l in levels {
                    let desc = match l.as_str() {
                        "low" => "Quick reasoning",
                        "medium" => "Balanced",
                        "high" => "Deep reasoning",
                        "xhigh" => "Maximum reasoning",
                        _ => l.as_str(),
                    };
                    result.push((l.as_str(), desc));
                }
                result
            }
            ThinkingSupport::AdaptiveEffort { .. } => vec![
                ("off", "No thinking"),
                ("low", "Light thinking"),
                ("medium", "Moderate thinking"),
                ("high", "Deep thinking (default)"),
                ("max", "Maximum thinking"),
            ],
            ThinkingSupport::BudgetTokens { max, .. } => {
                let mut levels = vec![("off", "No extended thinking")];
                if *max >= 4096 {
                    levels.push(("low", "4K token budget"));
                }
                if *max >= 8192 {
                    levels.push(("medium", "8K token budget"));
                }
                if *max >= 16384 {
                    levels.push(("high", "16K token budget"));
                }
                levels.push(("max", "Max token budget"));
                levels
            }
            ThinkingSupport::ThinkingLevel { levels, .. } => levels
                .iter()
                .map(|l| {
                    let desc = match l.as_str() {
                        "minimal" => "Minimal thinking",
                        "low" => "Low thinking",
                        "medium" => "Balanced",
                        "high" => "Deep thinking",
                        other => other,
                    };
                    (l.as_str(), desc)
                })
                .collect(),
        }
    }

    /// Returns the ordered list of all level names (including "off") for Tab cycling.
    pub fn cycle_levels(&self) -> Vec<&str> {
        self.user_facing_levels()
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub input_price_per_m: f64,
    #[serde(default)]
    pub output_price_per_m: f64,
    #[serde(default)]
    pub cache_read_price_per_m: f64,
    #[serde(default)]
    pub cache_write_price_per_m: f64,
    #[serde(default = "default_tier")]
    pub tier: ModelTier,
    #[serde(default)]
    pub thinking: Option<ThinkingSupport>,
}

fn default_tier() -> ModelTier {
    ModelTier::Medium
}

#[derive(Debug, Clone, Default)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
    pub thinking_level: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system: Option<String>,
    pub stream: bool,
    pub thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    pub fn as_text(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::Parts(parts) => parts
                .iter()
                .find_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .unwrap_or(""),
        }
    }

    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => {
                parts.iter().any(|p| matches!(p, ContentPart::Image { .. }))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { media_type: String, data: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: Message,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

impl ModelInfo {
    pub fn cost_usd(&self, usage: &Usage) -> f64 {
        let uncached = usage
            .input_tokens
            .saturating_sub(usage.cache_read_tokens)
            .saturating_sub(usage.cache_creation_tokens);
        let input_cost = uncached as f64 * self.input_price_per_m;
        let read_cost = usage.cache_read_tokens as f64 * self.cache_read_price_per_m;
        let write_cost = usage.cache_creation_tokens as f64 * self.cache_write_price_per_m;
        let output_cost = usage.output_tokens as f64 * self.output_price_per_m;
        (input_cost + read_cost + write_cost + output_cost) / 1_000_000.0
    }

    pub fn short_name(&self) -> &str {
        &self.name
    }

    pub fn context_display(&self) -> String {
        if self.context_window >= 1_000_000 {
            format!("{}M", self.context_window / 1_000_000)
        } else {
            format!("{}K", self.context_window / 1_000)
        }
    }

    pub fn has_thinking(&self) -> bool {
        self.thinking.is_some()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextDelta(String),
    ThinkingDelta(String),
    ReasoningSummary(String),
    ToolCallStart {
        index: u32,
        id: String,
        name: String,
    },
    ToolCallDelta {
        index: u32,
        arguments_delta: String,
    },
    ToolCallDone {
        index: u32,
    },
    Usage(Usage),
    Done,
    Error(String),
}
