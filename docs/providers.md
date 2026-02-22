# Providers

Nyzhi abstracts LLM access behind the `Provider` trait. Each provider implements streaming chat completions with tool use, and optionally supports thinking/reasoning modes.

---

## Provider Trait

Every provider implements:

```rust
trait Provider {
    fn name(&self) -> &str;
    fn supported_models(&self) -> Vec<ModelInfo>;
    fn model_for_tier(&self, tier: ModelTier) -> Option<String>;
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<BoxStream<StreamEvent>>;
}
```

---

## OpenAI

**API style**: `openai` (Bearer token auth)

**Default models**:

| Model | Context Window | Tier | Thinking |
|-------|---------------|------|----------|
| GPT-5.3 Codex | 1,048,576 | High | ReasoningEffort |
| GPT-5.2 Codex | 1,048,576 | High | ReasoningEffort |
| GPT-5.2 | 1,048,576 | High | ReasoningEffort |
| o3 | 200,000 | High | ReasoningEffort |
| o4-mini | 200,000 | Medium | ReasoningEffort |

**Auth**: API key (`OPENAI_API_KEY`) or OAuth device code flow.

**Thinking support**: `reasoning_effort` parameter (low/medium/high). For `o3` and `o4-mini` models, thinking is enabled via OpenAI's reasoning effort API.

**Streaming**: SSE with text deltas, tool call deltas, and usage events.

**Prompt caching**: Automatic (OpenAI caches internally). Cache read/creation tokens are tracked in usage.

---

## Anthropic

**API style**: `anthropic` (`x-api-key` header auth)

**Default models**:

| Model | Context Window | Tier | Thinking |
|-------|---------------|------|----------|
| Claude Opus 4.6 | 200,000 | High | AdaptiveEffort |
| Claude Sonnet 4.6 | 200,000 | Medium | AdaptiveEffort |
| Claude Haiku 4.5 | 200,000 | Low | None |

**Auth**: API key (`ANTHROPIC_API_KEY`) or OAuth PKCE flow.

**Thinking support**: Adaptive effort via `thinking.budget_tokens`. The system prompt and the last tool result are marked with `cache_control: { type: "ephemeral" }` for prompt caching.

**Streaming**: SSE with content_block_start/delta/stop events. Handles thinking blocks (type: "thinking") separately from text blocks.

**Special handling**: System messages are extracted from the message array and sent as a separate `system` parameter (Anthropic API requirement).

---

## Gemini

**API style**: `gemini`

**Default models**:

| Model | Context Window | Tier | Thinking |
|-------|---------------|------|----------|
| Gemini 3.1 Pro | 1,048,576 | High | BudgetTokens |
| Gemini 3 Flash | 1,048,576 | Low | BudgetTokens |
| Gemini 3 Pro | 1,048,576 | High | BudgetTokens |
| Gemini 2.5 Flash | 1,048,576 | Medium | BudgetTokens |

**Auth**: API key (`GEMINI_API_KEY`) as a query parameter, or OAuth Bearer token (Google PKCE flow).

**Thinking support**: `thinkingConfig.thinkingBudget` parameter for models that support extended thinking.

**Streaming**: SSE with `generateContent` streaming response format. Content parts include both text and function calls.

**Dual auth mode**: `GeminiAuthMode::ApiKey(key)` appends `?key=...` to the URL. `GeminiAuthMode::Bearer(token)` uses the `Authorization` header.

---

## OpenRouter

**API style**: `openai`

Use OpenRouter to access any model available on their platform:

```toml
[provider]
default = "openrouter"

[provider.openrouter]
model = "anthropic/claude-sonnet-4-20250514"
# api_key via OPENROUTER_API_KEY env var
```

---

## DeepSeek

**API style**: `openai`

```toml
[provider.deepseek]
model = "deepseek-chat"
```

---

## Groq

**API style**: `openai`

```toml
[provider.groq]
model = "llama-3.3-70b-versatile"
```

---

## Kimi (Moonshot)

**API style**: `openai`

Kimi models support thinking via a special `thinking` field in the request (not `reasoning_effort`). Nyzhi detects Kimi models by name prefix and adjusts the thinking parameter format.

---

## MiniMax and GLM

**API style**: `openai`

Standard OpenAI-compatible endpoints.

---

## Custom Providers

Any OpenAI-compatible API can be added:

```toml
[provider.my-local-llm]
base_url = "http://localhost:8080/v1"
api_key = "not-needed"
api_style = "openai"
env_var = "MY_LLM_KEY"
```

The `api_style` determines which provider implementation is used:

| api_style | Implementation | Auth Header |
|-----------|---------------|-------------|
| `openai` | `OpenAIProvider` | `Authorization: Bearer <key>` |
| `anthropic` | `AnthropicProvider` | `x-api-key: <key>` |
| `gemini` | `GeminiProvider` | Query param or Bearer |

---

## Thinking and Reasoning

Different providers use different thinking mechanisms:

| Type | Provider | Parameter |
|------|----------|-----------|
| `ReasoningEffort` | OpenAI (o3, o4-mini, GPT-5.x) | `reasoning_effort: "low"/"medium"/"high"` |
| `AdaptiveEffort` | Anthropic (Claude Opus/Sonnet) | `thinking.budget_tokens: N` |
| `BudgetTokens` | Gemini | `thinkingConfig.thinkingBudget: N` |
| `ThinkingLevel` | Kimi | `thinking: true` |

Thinking can be configured in the agent:

```toml
[agent]
# thinking_enabled = true
# thinking_budget = 10000
# reasoning_effort = "medium"
```

---

## Model Registry

The `ModelRegistry` aggregates models from all configured providers:

- `models_for(provider)` -- list models for a specific provider
- `all_models()` -- list all available models
- `find(model_id)` -- find a model by exact ID
- `find_any(model_id)` -- find across all providers
- `providers()` -- list provider names with models

### Model Info

Each model carries metadata:

```rust
struct ModelInfo {
    id: String,              // e.g., "gpt-5.2-codex"
    name: String,            // e.g., "GPT-5.2 Codex"
    provider: String,        // e.g., "openai"
    context_window: usize,   // e.g., 1048576
    max_output_tokens: usize,
    input_price_per_m: f64,  // USD per million tokens
    output_price_per_m: f64,
    tier: ModelTier,         // Low, Medium, High
    thinking: Option<ThinkingSupport>,
}
```

---

## Stub Providers

Two providers exist as stubs for future integration:

- **Claude SDK** (`claude-sdk`): Placeholder for Claude Agent SDK integration. Returns an error instructing to install Claude Code CLI.
- **Codex** (`codex`): Placeholder for OpenAI Codex CLI integration. Checks for the `codex` binary on PATH.

Neither is usable in the current release.
