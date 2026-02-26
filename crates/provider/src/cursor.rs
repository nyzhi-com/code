use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use futures::stream::{BoxStream, StreamExt};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::types::*;
use crate::{Provider, ProviderError};

const BASE_URL: &str = "https://api2.cursor.sh";
const CLIENT_VERSION: &str = "cli-2025.11.25-d5b3271";
const DEFAULT_MODEL: &str = "claude-4.6-sonnet";

const AGENT_RUN_SSE: &str = "/agent.v1.AgentService/RunSSE";
const BIDI_APPEND: &str = "/aiserver.v1.BidiService/BidiAppend";

const AGENT_MODE_AGENT: u32 = 1;

// ── Protobuf encoding ────────────────────────────────────────────────

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10);
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

fn encode_string_field(field_number: u32, value: &str) -> Vec<u8> {
    if value.is_empty() {
        return Vec::new();
    }
    let tag = ((field_number << 3) | 2) as u8;
    let data = value.as_bytes();
    let length = encode_varint(data.len() as u64);
    let mut out = Vec::with_capacity(1 + length.len() + data.len());
    out.push(tag);
    out.extend_from_slice(&length);
    out.extend_from_slice(data);
    out
}

fn encode_uint32_field(field_number: u32, value: u32) -> Vec<u8> {
    if value == 0 {
        return Vec::new();
    }
    let tag = (field_number << 3) as u8; // wire type 0
    let encoded = encode_varint(value as u64);
    let mut out = Vec::with_capacity(1 + encoded.len());
    out.push(tag);
    out.extend_from_slice(&encoded);
    out
}

fn encode_int64_field(field_number: u32, value: u64) -> Vec<u8> {
    let tag = (field_number << 3) as u8; // wire type 0
    let encoded = encode_varint(value);
    let mut out = Vec::with_capacity(1 + encoded.len());
    out.push(tag);
    out.extend_from_slice(&encoded);
    out
}

fn encode_message_field(field_number: u32, data: &[u8]) -> Vec<u8> {
    let tag = ((field_number << 3) | 2) as u8;
    let length = encode_varint(data.len() as u64);
    let mut out = Vec::with_capacity(1 + length.len() + data.len());
    out.push(tag);
    out.extend_from_slice(&length);
    out.extend_from_slice(data);
    out
}

fn concat_fields(parts: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(total);
    for p in parts {
        out.extend_from_slice(p);
    }
    out
}

// ── Protobuf decoding ────────────────────────────────────────────────

struct ProtoField {
    field_number: u32,
    wire_type: u32,
    bytes_value: Option<Vec<u8>>,
    varint_value: Option<u64>,
}

fn decode_varint(data: &[u8], offset: usize) -> (u64, usize) {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut n = 0;
    while offset + n < data.len() {
        let byte = data[offset + n];
        value |= ((byte & 0x7f) as u64) << shift;
        n += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            break;
        }
    }
    (value, n)
}

fn parse_proto_fields(data: &[u8]) -> Vec<ProtoField> {
    let mut fields = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (tag, tag_len) = decode_varint(data, offset);
        if tag_len == 0 {
            break;
        }
        offset += tag_len;
        let field_number = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u32;
        match wire_type {
            0 => {
                let (val, vl) = decode_varint(data, offset);
                offset += vl;
                fields.push(ProtoField {
                    field_number,
                    wire_type,
                    bytes_value: None,
                    varint_value: Some(val),
                });
            }
            2 => {
                let (length, ll) = decode_varint(data, offset);
                offset += ll;
                let end = offset + length as usize;
                if end > data.len() {
                    break;
                }
                fields.push(ProtoField {
                    field_number,
                    wire_type,
                    bytes_value: Some(data[offset..end].to_vec()),
                    varint_value: None,
                });
                offset = end;
            }
            1 => {
                if offset + 8 > data.len() {
                    break;
                }
                fields.push(ProtoField {
                    field_number,
                    wire_type,
                    bytes_value: Some(data[offset..offset + 8].to_vec()),
                    varint_value: None,
                });
                offset += 8;
            }
            5 => {
                if offset + 4 > data.len() {
                    break;
                }
                fields.push(ProtoField {
                    field_number,
                    wire_type,
                    bytes_value: Some(data[offset..offset + 4].to_vec()),
                    varint_value: None,
                });
                offset += 4;
            }
            _ => break,
        }
    }
    fields
}

// ── gRPC-web framing ─────────────────────────────────────────────────

fn grpc_web_envelope(data: &[u8], flags: u8) -> Vec<u8> {
    let len = data.len() as u32;
    let mut out = Vec::with_capacity(5 + data.len());
    out.push(flags);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(data);
    out
}

struct GrpcFrame {
    flags: u8,
    data: Vec<u8>,
}

fn parse_grpc_frames(buffer: &[u8]) -> (Vec<GrpcFrame>, usize) {
    let mut frames = Vec::new();
    let mut offset = 0;
    while offset + 5 <= buffer.len() {
        let flags = buffer[offset];
        let length = u32::from_be_bytes([
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
            buffer[offset + 4],
        ]) as usize;
        let end = offset + 5 + length;
        if end > buffer.len() {
            break;
        }
        frames.push(GrpcFrame {
            flags,
            data: buffer[offset + 5..end].to_vec(),
        });
        offset = end;
    }
    (frames, offset)
}

// ── Checksum ─────────────────────────────────────────────────────────

pub fn generate_checksum_for_listing(token: &str) -> String {
    generate_checksum(token)
}

fn generate_checksum(token: &str) -> String {
    let parts: Vec<&str> = token.split('.').collect();

    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let rounded_ms = epoch_ms - (epoch_ms % 1_800_000);
    let timestamp = rounded_ms / 1_000_000;

    let mut bytes = [0u8; 6];
    let mut temp = timestamp;
    for b in bytes.iter_mut().rev() {
        *b = (temp & 0xff) as u8;
        temp >>= 8;
    }

    let mut key: u8 = 165;
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (*b ^ key).wrapping_add(i as u8);
        key = *b;
    }

    let hex1 = if parts.len() > 1 && !parts[1].is_empty() {
        let h = Sha256::digest(parts[1].as_bytes());
        format!("{:x}", h)[..8].to_string()
    } else {
        "00000000".to_string()
    };
    let hex2 = {
        let h = Sha256::digest(token.as_bytes());
        format!("{:x}", h)[..8].to_string()
    };

    format!("{}{hex1}/{hex2}", URL_SAFE_NO_PAD.encode(bytes))
}

// ── Hex helpers ──────────────────────────────────────────────────────

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ── Message building ─────────────────────────────────────────────────

fn encode_user_message(text: &str, message_id: &str, mode: u32) -> Vec<u8> {
    concat_fields(&[
        encode_string_field(1, text),
        encode_string_field(2, message_id),
        encode_uint32_field(4, mode),
    ])
}

fn encode_request_context_env(workspace_path: &str) -> Vec<u8> {
    let tz = std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string());
    let os_info = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    concat_fields(&[
        encode_string_field(1, &os_info),
        encode_string_field(2, workspace_path),
        encode_string_field(3, &shell),
        encode_string_field(10, &tz),
        encode_string_field(11, workspace_path),
    ])
}

fn encode_request_context(workspace_path: &str) -> Vec<u8> {
    let env = encode_request_context_env(workspace_path);
    encode_message_field(4, &env)
}

fn encode_user_message_action(user_message: &[u8], request_context: &[u8]) -> Vec<u8> {
    concat_fields(&[
        encode_message_field(1, user_message),
        encode_message_field(2, request_context),
    ])
}

fn encode_conversation_action(user_message_action: &[u8]) -> Vec<u8> {
    encode_message_field(1, user_message_action)
}

fn encode_model_details(model: &str) -> Vec<u8> {
    encode_string_field(1, model)
}

fn encode_agent_run_request(
    action: &[u8],
    model_details: &[u8],
    conversation_id: &str,
) -> Vec<u8> {
    concat_fields(&[
        encode_message_field(1, &[]), // empty ConversationState
        encode_message_field(2, action),
        encode_message_field(3, model_details),
        encode_string_field(5, conversation_id),
    ])
}

fn encode_agent_client_message(run_request: &[u8]) -> Vec<u8> {
    encode_message_field(1, run_request)
}

fn encode_bidi_request_id(request_id: &str) -> Vec<u8> {
    encode_string_field(1, request_id)
}

fn encode_bidi_append_request(hex_data: &str, request_id: &str, append_seqno: u64) -> Vec<u8> {
    let request_id_msg = encode_bidi_request_id(request_id);
    concat_fields(&[
        encode_string_field(1, hex_data),
        encode_message_field(2, &request_id_msg),
        encode_int64_field(3, append_seqno),
    ])
}

fn format_message_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(s) => s.clone(),
        MessageContent::Parts(parts) => {
            let mut buf = String::new();
            for part in parts {
                match part {
                    ContentPart::Text { text } => buf.push_str(text),
                    ContentPart::ToolUse { name, input, .. } => {
                        buf.push_str(&format!("[Tool call: {name}({input})]"));
                    }
                    ContentPart::ToolResult {
                        content,
                        tool_name,
                        tool_use_id,
                    } => {
                        let label = tool_name.as_deref().unwrap_or(tool_use_id);
                        buf.push_str(&format!("[Tool result ({label}): {content}]"));
                    }
                    ContentPart::Image { .. } => buf.push_str("[Image]"),
                }
            }
            buf
        }
    }
}

fn build_prompt_text(request: &ChatRequest) -> String {
    let mut parts = Vec::new();
    if let Some(sys) = &request.system {
        if !sys.is_empty() {
            parts.push(sys.clone());
        }
    }
    for msg in &request.messages {
        let prefix = match msg.role {
            Role::System => {
                parts.push(format_message_content(&msg.content));
                continue;
            }
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
        };
        let text = format_message_content(&msg.content);
        if !text.is_empty() {
            parts.push(format!("{prefix}: {text}"));
        }
    }
    parts.join("\n\n")
}

fn build_chat_message(request: &ChatRequest, model: &str) -> Vec<u8> {
    let message_id = Uuid::new_v4().to_string();
    let conversation_id = Uuid::new_v4().to_string();
    let workspace_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/project".to_string());

    let prompt = build_prompt_text(request);
    let request_context = encode_request_context(&workspace_path);
    let user_message = encode_user_message(&prompt, &message_id, AGENT_MODE_AGENT);
    let user_message_action = encode_user_message_action(&user_message, &request_context);
    let conversation_action = encode_conversation_action(&user_message_action);
    let model_details = encode_model_details(model);
    let run_request =
        encode_agent_run_request(&conversation_action, &model_details, &conversation_id);
    encode_agent_client_message(&run_request)
}

// ── Response parsing ─────────────────────────────────────────────────

struct InteractionUpdate {
    text: Option<String>,
    thinking: Option<String>,
    is_complete: bool,
    #[allow(dead_code)]
    is_heartbeat: bool,
}

fn parse_interaction_update(data: &[u8]) -> InteractionUpdate {
    let fields = parse_proto_fields(data);
    let mut text = None;
    let mut thinking = None;
    let mut is_complete = false;
    let mut is_heartbeat = false;

    for field in &fields {
        match (field.field_number, field.wire_type) {
            (1, 2) => {
                if let Some(bytes) = &field.bytes_value {
                    for inner in parse_proto_fields(bytes) {
                        if inner.field_number == 1 && inner.wire_type == 2 {
                            if let Some(b) = &inner.bytes_value {
                                text = String::from_utf8(b.clone()).ok();
                            }
                        }
                    }
                }
            }
            (4, 2) => {
                if let Some(bytes) = &field.bytes_value {
                    for inner in parse_proto_fields(bytes) {
                        if inner.field_number == 1 && inner.wire_type == 2 {
                            if let Some(b) = &inner.bytes_value {
                                thinking = String::from_utf8(b.clone()).ok();
                            }
                        }
                    }
                }
            }
            (8, 2) => {
                if text.is_none() {
                    if let Some(bytes) = &field.bytes_value {
                        for inner in parse_proto_fields(bytes) {
                            if inner.field_number == 1 && inner.wire_type == 2 {
                                if let Some(b) = &inner.bytes_value {
                                    text = String::from_utf8(b.clone()).ok();
                                }
                            }
                        }
                    }
                }
            }
            (13, _) => is_heartbeat = true,
            (14, _) => is_complete = true,
            _ => {}
        }
    }

    InteractionUpdate {
        text,
        thinking,
        is_complete,
        is_heartbeat,
    }
}

// ── KV blob handling ─────────────────────────────────────────────────

enum KvMsgType {
    GetBlob,
    SetBlob,
    Unknown,
}

struct KvServerMsg {
    id: u32,
    msg_type: KvMsgType,
    blob_id: Option<Vec<u8>>,
    blob_data: Option<Vec<u8>>,
}

fn parse_kv_server_message(data: &[u8]) -> KvServerMsg {
    let fields = parse_proto_fields(data);
    let mut msg = KvServerMsg {
        id: 0,
        msg_type: KvMsgType::Unknown,
        blob_id: None,
        blob_data: None,
    };
    for field in &fields {
        match (field.field_number, field.wire_type) {
            (1, 0) => msg.id = field.varint_value.unwrap_or(0) as u32,
            (2, 2) => {
                msg.msg_type = KvMsgType::GetBlob;
                if let Some(bytes) = &field.bytes_value {
                    for inner in parse_proto_fields(bytes) {
                        if inner.field_number == 1 && inner.wire_type == 2 {
                            msg.blob_id = inner.bytes_value.clone();
                        }
                    }
                }
            }
            (3, 2) => {
                msg.msg_type = KvMsgType::SetBlob;
                if let Some(bytes) = &field.bytes_value {
                    for inner in parse_proto_fields(bytes) {
                        if inner.field_number == 1 && inner.wire_type == 2 {
                            msg.blob_id = inner.bytes_value.clone();
                        } else if inner.field_number == 2 && inner.wire_type == 2 {
                            msg.blob_data = inner.bytes_value.clone();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    msg
}

fn build_kv_client_message(id: u32, is_get: bool, result: &[u8]) -> Vec<u8> {
    let field_num = if is_get { 2 } else { 3 };
    concat_fields(&[
        encode_uint32_field(1, id),
        encode_message_field(field_num, result),
    ])
}

fn build_agent_kv_message(kv_client: &[u8]) -> Vec<u8> {
    encode_message_field(3, kv_client)
}

fn extract_assistant_content(blob_data: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(blob_data).ok()?;
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    if json.get("role").and_then(|v| v.as_str()) != Some("assistant") {
        return None;
    }
    match json.get("content") {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(serde_json::Value::Array(arr)) => {
            let mut buf = String::new();
            for item in arr {
                if let Some(s) = item.as_str() {
                    buf.push_str(s);
                } else if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                        buf.push_str(t);
                    }
                }
            }
            if buf.is_empty() {
                None
            } else {
                Some(buf)
            }
        }
        _ => None,
    }
}

// ── gRPC trailer parsing ─────────────────────────────────────────────

fn parse_grpc_status(trailer: &str) -> Option<u32> {
    for line in trailer.split('\n') {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("grpc-status:") {
            return val.trim().parse().ok();
        }
    }
    None
}

fn parse_grpc_message(trailer: &str) -> Option<String> {
    for line in trailer.split('\n') {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("grpc-message:") {
            return Some(val.trim().to_string());
        }
    }
    None
}

// ── Static model definitions ─────────────────────────────────────────

pub fn cursor_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-4.6-sonnet".into(),
            name: "Claude 4.6 Sonnet".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "claude-4.6-opus".into(),
            name: "Claude 4.6 Opus".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            provider: "cursor".into(),
            context_window: 272_000,
            max_output_tokens: 128_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            provider: "cursor".into(),
            context_window: 272_000,
            max_output_tokens: 100_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gemini-3.1-pro".into(),
            name: "Gemini 3.1 Pro".into(),
            provider: "cursor".into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "gemini-3-flash".into(),
            name: "Gemini 3 Flash".into(),
            provider: "cursor".into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Low,
            thinking: None,
        },
        ModelInfo {
            id: "auto".into(),
            name: "Auto (server picks)".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: None,
        },
    ]
}

// ── Provider ─────────────────────────────────────────────────────────

pub struct CursorProvider {
    client: reqwest::Client,
    access_token: String,
    #[allow(dead_code)]
    machine_id: String,
    default_model: String,
    models: Vec<ModelInfo>,
}

impl CursorProvider {
    pub fn new(access_token: String, machine_id: String, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            access_token,
            machine_id,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models: cursor_models(),
        }
    }

    fn get_headers(&self, request_id: Option<&str>) -> reqwest::header::HeaderMap {
        let checksum = generate_checksum(&self.access_token);
        let tz = std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string());

        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            "authorization",
            format!("Bearer {}", self.access_token).parse().unwrap(),
        );
        h.insert("content-type", "application/grpc-web+proto".parse().unwrap());
        h.insert("user-agent", "connect-es/1.4.0".parse().unwrap());
        h.insert("x-cursor-checksum", checksum.parse().unwrap());
        h.insert(
            "x-cursor-client-version",
            CLIENT_VERSION.parse().unwrap(),
        );
        h.insert("x-cursor-client-type", "cli".parse().unwrap());
        h.insert("x-cursor-timezone", tz.parse().unwrap());
        h.insert("x-ghost-mode", "true".parse().unwrap());
        h.insert("x-cursor-streaming", "true".parse().unwrap());
        if let Some(rid) = request_id {
            h.insert("x-request-id", rid.parse().unwrap());
        }
        h
    }

    async fn bidi_append_raw(
        client: &reqwest::Client,
        headers: &reqwest::header::HeaderMap,
        request_id: &str,
        seqno: u64,
        data: &[u8],
    ) -> Result<()> {
        let hex_data = hex_encode(data);
        let append_request = encode_bidi_append_request(&hex_data, request_id, seqno);
        let envelope = grpc_web_envelope(&append_request, 0);

        let url = format!("{BASE_URL}{BIDI_APPEND}");
        let resp = client
            .post(&url)
            .headers(headers.clone())
            .body(envelope)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("BidiAppend failed: {text}");
        }
        Ok(())
    }
}

#[async_trait]
impl Provider for CursorProvider {
    fn name(&self) -> &str {
        "cursor"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        &self.models
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let mut full_text = String::new();
        let mut stream = self.chat_stream(request).await?;
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::TextDelta(t) => full_text.push_str(&t),
                StreamEvent::Done => break,
                _ => {}
            }
        }
        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(full_text),
            },
            usage: None,
            finish_reason: Some("stop".to_string()),
        })
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        let message_body = build_chat_message(request, model);
        let request_id = Uuid::new_v4().to_string();
        let headers = self.get_headers(Some(&request_id));

        let bidi_request_id = encode_bidi_request_id(&request_id);
        let sse_envelope = grpc_web_envelope(&bidi_request_id, 0);

        let sse_url = format!("{BASE_URL}{AGENT_RUN_SSE}");

        let client = self.client.clone();
        let headers_clone = headers.clone();

        // Start SSE connection and send initial message concurrently
        let sse_future = client
            .post(&sse_url)
            .headers(headers.clone())
            .body(sse_envelope)
            .send();

        let append_future = Self::bidi_append_raw(&client, &headers_clone, &request_id, 0, &message_body);

        let (sse_result, append_result) = tokio::join!(sse_future, append_future);
        append_result?;
        let sse_resp = sse_result?;

        if !sse_resp.status().is_success() {
            let status = sse_resp.status().as_u16();
            let text = sse_resp.text().await.unwrap_or_default();
            return Err(ProviderError::from_http(status, text, None).into());
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Result<StreamEvent>>();

        tokio::spawn(async move {
            let mut append_seqno: u64 = 1;
            let mut buffer: Vec<u8> = Vec::new();
            let mut blob_store: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
            let mut has_streamed_text = false;
            let mut pending_content: Vec<String> = Vec::new();
            let mut byte_stream = sse_resp.bytes_stream();

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(Err(e.into()));
                        return;
                    }
                };

                buffer.extend_from_slice(&chunk);
                let (frames, consumed) = parse_grpc_frames(&buffer);
                buffer = buffer.split_off(consumed);

                for frame in frames {
                    // Trailer frame
                    if frame.flags & 0x80 != 0 {
                        let trailer = String::from_utf8_lossy(&frame.data);
                        if let Some(status) = parse_grpc_status(&trailer) {
                            if status != 0 {
                                let msg = parse_grpc_message(&trailer)
                                    .unwrap_or_else(|| format!("gRPC error (status {status})"));
                                let _ = tx.send(Err(anyhow::anyhow!(msg)));
                            }
                        }
                        continue;
                    }

                    let server_fields = parse_proto_fields(&frame.data);
                    for field in &server_fields {
                        match (field.field_number, field.wire_type) {
                            // interaction_update
                            (1, 2) => {
                                if let Some(bytes) = &field.bytes_value {
                                    let update = parse_interaction_update(bytes);
                                    if let Some(text) = update.text {
                                        if !text.is_empty() {
                                            has_streamed_text = true;
                                            if tx.send(Ok(StreamEvent::TextDelta(text))).is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    if let Some(think) = update.thinking {
                                        if !think.is_empty()
                                            && tx
                                                .send(Ok(StreamEvent::ThinkingDelta(think)))
                                                .is_err()
                                        {
                                            return;
                                        }
                                    }
                                    if update.is_complete {
                                        if !has_streamed_text {
                                            for c in &pending_content {
                                                if tx
                                                    .send(Ok(StreamEvent::TextDelta(c.clone())))
                                                    .is_err()
                                                {
                                                    return;
                                                }
                                            }
                                        }
                                        let _ = tx.send(Ok(StreamEvent::Done));
                                        return;
                                    }
                                }
                            }
                            // kv_server_message
                            (4, 2) => {
                                if let Some(bytes) = &field.bytes_value {
                                    let kv = parse_kv_server_message(bytes);
                                    match kv.msg_type {
                                        KvMsgType::GetBlob => {
                                            let result = kv
                                                .blob_id
                                                .as_ref()
                                                .and_then(|id| blob_store.get(id))
                                                .map(|d| encode_message_field(1, d))
                                                .unwrap_or_default();
                                            let kv_client =
                                                build_kv_client_message(kv.id, true, &result);
                                            let response = build_agent_kv_message(&kv_client);
                                            let _ = Self::bidi_append_raw(
                                                &client,
                                                &headers_clone,
                                                &request_id,
                                                append_seqno,
                                                &response,
                                            )
                                            .await;
                                            append_seqno += 1;
                                        }
                                        KvMsgType::SetBlob => {
                                            if let (Some(blob_id), Some(blob_data)) =
                                                (&kv.blob_id, &kv.blob_data)
                                            {
                                                if let Some(content) =
                                                    extract_assistant_content(blob_data)
                                                {
                                                    pending_content.push(content);
                                                }
                                                blob_store
                                                    .insert(blob_id.clone(), blob_data.clone());
                                            }
                                            let kv_client =
                                                build_kv_client_message(kv.id, false, &[]);
                                            let response = build_agent_kv_message(&kv_client);
                                            let _ = Self::bidi_append_raw(
                                                &client,
                                                &headers_clone,
                                                &request_id,
                                                append_seqno,
                                                &response,
                                            )
                                            .await;
                                            append_seqno += 1;
                                        }
                                        KvMsgType::Unknown => {}
                                    }
                                }
                            }
                            // checkpoint, exec, control — acknowledged but not acted on yet
                            _ => {}
                        }
                    }
                }
            }

            // Stream ended without explicit turn_ended
            if !has_streamed_text {
                for c in &pending_content {
                    let _ = tx.send(Ok(StreamEvent::TextDelta(c.clone())));
                }
            }
            let _ = tx.send(Ok(StreamEvent::Done));
        });

        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(Box::pin(stream))
    }
}
