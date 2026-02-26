use nyzhi_provider::StreamEvent;

#[derive(Debug, Clone, Default)]
pub struct StreamAccumulator {
    pub text: String,
    pub tool_calls: Vec<AccumulatedToolCall>,
    pub usage: Option<nyzhi_provider::Usage>,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct AccumulatedToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::TextDelta(text) => {
                self.text.push_str(text);
            }
            StreamEvent::ToolCallStart { id, name, .. } => {
                self.tool_calls.push(AccumulatedToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: String::new(),
                });
            }
            StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } => {
                let idx = *index as usize;
                if let Some(tc) = self.tool_calls.get_mut(idx) {
                    tc.arguments.push_str(arguments_delta);
                } else if let Some(tc) = self.tool_calls.last_mut() {
                    tc.arguments.push_str(arguments_delta);
                }
            }
            StreamEvent::Usage(usage) => {
                if let Some(existing) = &mut self.usage {
                    if usage.input_tokens > 0 {
                        existing.input_tokens = usage.input_tokens;
                    }
                    if usage.output_tokens > 0 {
                        existing.output_tokens = usage.output_tokens;
                    }
                } else {
                    self.usage = Some(usage.clone());
                }
            }
            StreamEvent::Done => {
                self.done = true;
            }
            _ => {}
        }
    }

    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}
