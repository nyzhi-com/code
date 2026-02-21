use anyhow::Result;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Response;

pub fn parse_sse_stream(
    response: Response,
) -> BoxStream<'static, Result<SseEvent>> {
    let byte_stream = response.bytes_stream();
    let buffer = String::new();

    let stream = futures::stream::unfold(
        (byte_stream, buffer),
        |(mut byte_stream, mut buffer)| async move {
            loop {
                if let Some(pos) = buffer.find("\n\n") {
                    let event_text = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    if let Some(event) = parse_event(&event_text) {
                        return Some((Ok(event), (byte_stream, buffer)));
                    }
                    continue;
                }

                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Some(Err(e)) => {
                        return Some((Err(e.into()), (byte_stream, buffer)));
                    }
                    None => return None,
                }
            }
        },
    );

    Box::pin(stream)
}

#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

fn parse_event(text: &str) -> Option<SseEvent> {
    let mut event_type = None;
    let mut data_lines = Vec::new();

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value);
        } else if line == "data:" {
            data_lines.push("");
        }
    }

    if data_lines.is_empty() {
        return None;
    }

    let data = data_lines.join("\n");
    if data == "[DONE]" {
        return None;
    }

    Some(SseEvent {
        event: event_type,
        data,
    })
}
