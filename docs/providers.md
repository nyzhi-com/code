# Providers

`nyzhi-provider` exposes a common `Provider` trait and routes concrete providers by `api_style`.

## Provider interface

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn supported_models(&self) -> &[ModelInfo];
    fn model_for_tier(&self, tier: ModelTier) -> Option<&ModelInfo>;
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>>;
}
```

## API style mapping

Provider selection is resolved from config in this order:

1. `[provider.<id>].api_style` override
2. Built-in provider definition
3. Fallback to `openai`

Supported styles:

- `openai` -> `OpenAIProvider`
- `anthropic` -> `AnthropicProvider`
- `gemini` -> `GeminiProvider`
- `cursor` -> `CursorProvider`
- `claude-sdk` -> `ClaudeSDKProvider` (feature-gated stub)
- `codex` -> `CodexProvider` (stub; not fully integrated)

## OpenAI provider

Default model: `gpt-5.3-codex`

| id | name | context | max output | tier | thinking |
|---|---|---:|---:|---|---|
| `gpt-5.3-codex` | GPT-5.3 Codex | 400,000 | 128,000 | high | reasoning effort |
| `gpt-5.2-codex` | GPT-5.2 Codex | 272,000 | 100,000 | high | reasoning effort |
| `gpt-5.2` | GPT-5.2 | 272,000 | 100,000 | high | reasoning effort |

Notes:

- Uses OpenAI Chat Completions API by default.
- If the credential looks like a JWT (`starts_with("ey")`) and base URL is default, provider switches to Codex subscription endpoint (`https://chatgpt.com/backend-api/codex`) and uses the Responses API.
- When base URL contains `openrouter.ai`, it adds `HTTP-Referer` and `X-Title` headers.
- Kimi models sent through the OpenAI-compatible path get special `thinking` payload behavior.

## Anthropic provider

Default model: `claude-sonnet-4-6-20260217`

| id | name | context | max output | tier | thinking |
|---|---|---:|---:|---|---|
| `claude-opus-4-6-20260205` | Claude Opus 4.6 | 1,000,000 | 128,000 | high | adaptive effort |
| `claude-sonnet-4-6-20260217` | Claude Sonnet 4.6 | 1,000,000 | 16,384 | medium | adaptive effort |
| `claude-haiku-4-5-20251022` | Claude Haiku 4.5 | 200,000 | 8,192 | low | none |

Notes:

- Sends `x-api-key` and `anthropic-version: 2023-06-01`.
- System prompt is emitted via Anthropic `system` field with ephemeral cache control.
- Stream parser supports text deltas, thinking deltas, tool call start/delta, and usage events.

## Gemini provider

Default model: `gemini-3-flash`

| id | name | context | max output | tier | thinking |
|---|---|---:|---:|---|---|
| `gemini-3.1-pro-preview` | Gemini 3.1 Pro | 1,048,576 | 65,536 | high | thinking levels |
| `gemini-3-flash` | Gemini 3 Flash | 1,048,576 | 65,536 | low | thinking levels |
| `gemini-3-pro-preview` | Gemini 3 Pro | 1,048,576 | 65,536 | high | thinking levels |
| `gemini-2.5-flash` | Gemini 2.5 Flash | 1,048,576 | 65,536 | low | budget tokens |

Notes:

- Supports API key mode (`?key=` URL parameter) and bearer mode (`Authorization` header).
- Uses `generateContent` / `streamGenerateContent` with `alt=sse`.
- Thinking in streaming mode is sent through `generationConfig.thinkingConfig.thinkingBudget`.

## Cursor provider

Uses Cursor credentials (`access_token` + `machine_id`) and calls `https://api2.cursor.sh/v1/chat/completions`.

Bundled models include Claude, GPT, and Gemini aliases plus `auto`.

| id | context | max output | tier |
|---|---:|---:|---|
| `claude-4-sonnet` | 200,000 | 64,000 | high |
| `claude-4.5-sonnet-thinking` | 200,000 | 64,000 | high |
| `claude-4.5-opus-high` | 200,000 | 64,000 | high |
| `claude-4.5-opus-high-thinking` | 200,000 | 64,000 | high |
| `gpt-5.3-codex` | 400,000 | 128,000 | high |
| `gpt-5.2` | 272,000 | 100,000 | medium |
| `gpt-4o` | 128,000 | 16,384 | medium |
| `gemini-3-pro` | 1,048,576 | 65,536 | high |
| `gemini-3-flash` | 1,048,576 | 65,536 | low |
| `auto` | 200,000 | 64,000 | medium |

## ModelRegistry defaults

`ModelRegistry::new()` includes models for:

- `openai`
- `anthropic`
- `gemini`
- `deepseek`
- `groq`
- `kimi` and `kimi-coding`
- `minimax` and `minimax-coding`
- `glm` and `glm-coding`
- `cursor`
- `together`
- `ollama`
- `openrouter` (empty list by default)

## Thinking support types

`ModelInfo.thinking` may contain:

- `ReasoningEffort` (OpenAI-style effort levels)
- `AdaptiveEffort` (Anthropic adaptive thinking)
- `BudgetTokens` (token-budget style)
- `ThinkingLevel` (named levels such as minimal/low/medium/high)

The UI converts provider-specific knobs into a unified `/thinking` experience.

## Stubs / partial integrations

- `claude-sdk`: requires `claude-sdk` feature and external CLI; returns informative error when unavailable.
- `codex`: verifies `codex` binary exists, but runtime MCP integration is not complete; recommends OpenAI provider with Codex models.
