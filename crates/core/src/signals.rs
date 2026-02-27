use crate::conversation::Thread;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSignal {
    pub kind: SignalKind,
    pub turn: usize,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalKind {
    Friction,
    Delight,
    ToolFailure,
    Retry,
    LongPause,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionAnalytics {
    pub total_turns: usize,
    pub total_tool_calls: usize,
    pub tool_failures: usize,
    pub retries: usize,
    pub friction_moments: Vec<SessionSignal>,
    pub avg_turn_duration_ms: u64,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}

pub fn analyze_session(thread: &Thread) -> SessionAnalytics {
    let messages = thread.messages();
    let mut analytics = SessionAnalytics {
        total_turns: messages.len(),
        ..Default::default()
    };

    let mut consecutive_user_msgs = 0;
    let mut turn_idx = 0;

    for msg in messages {
        turn_idx += 1;
        match msg.role {
            nyzhi_provider::Role::User => {
                consecutive_user_msgs += 1;
                if consecutive_user_msgs >= 3 {
                    analytics.friction_moments.push(SessionSignal {
                        kind: SignalKind::Friction,
                        turn: turn_idx,
                        description: format!(
                            "{consecutive_user_msgs} consecutive user messages (possible rephrasing)"
                        ),
                    });
                }
            }
            nyzhi_provider::Role::Assistant => {
                consecutive_user_msgs = 0;
            }
            nyzhi_provider::Role::Tool => {
                analytics.total_tool_calls += 1;
                let content = match &msg.content {
                    nyzhi_provider::MessageContent::Text(t) => t.clone(),
                    nyzhi_provider::MessageContent::Parts(parts) => {
                        parts
                            .iter()
                            .filter_map(|p| match p {
                                nyzhi_provider::ContentPart::Text { text } => Some(text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    }
                };
                if content.contains("Error:") || content.contains("error:") || content.contains("failed") {
                    analytics.tool_failures += 1;
                    analytics.friction_moments.push(SessionSignal {
                        kind: SignalKind::ToolFailure,
                        turn: turn_idx,
                        description: "Tool returned an error".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    analytics
}

pub fn format_analytics(a: &SessionAnalytics) -> String {
    let mut out = String::from("Session Analytics:\n");
    out.push_str(&format!("  Turns: {}\n", a.total_turns));
    out.push_str(&format!("  Tool calls: {}\n", a.total_tool_calls));
    out.push_str(&format!("  Tool failures: {}\n", a.tool_failures));
    out.push_str(&format!("  Tokens: {}in / {}out\n", a.total_input_tokens, a.total_output_tokens));

    if a.friction_moments.is_empty() {
        out.push_str("  Friction: none detected\n");
    } else {
        out.push_str(&format!("  Friction moments: {}\n", a.friction_moments.len()));
        for s in &a.friction_moments {
            out.push_str(&format!("    [turn {}] {:?}: {}\n", s.turn, s.kind, s.description));
        }
    }

    out
}
