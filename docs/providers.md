# Providers

Source of truth:

- `crates/config/src/lib.rs` (`BUILT_IN_PROVIDERS`)
- `crates/provider/src/lib.rs`
- provider implementations under `crates/provider/src/*`

## Provider Model

The runtime uses a provider abstraction:

- trait: `Provider`
- creation: `create_provider` / `create_provider_async`
- capability metadata: `ModelInfo`, `ModelTier`, `ThinkingSupport`

Each provider has:

- provider id
- auth path (API key, OAuth, local token)
- API style (`openai`, `anthropic`, `gemini`, `cursor`, `copilot`, `claude-sdk`, `codex`)
- model inventory (hardcoded + optional remote refresh)

## Built-in Providers

| Provider id | Name | API style | Env var | OAuth support | Default base URL |
| --- | --- | --- | --- | --- | --- |
| `openai` | OpenAI | `openai` | `OPENAI_API_KEY` | yes | `https://api.openai.com/v1` |
| `anthropic` | Anthropic | `anthropic` | `ANTHROPIC_API_KEY` | yes | `https://api.anthropic.com/v1` |
| `gemini` | Google Gemini | `gemini` | `GEMINI_API_KEY` | yes | `https://generativelanguage.googleapis.com/v1beta` |
| `cursor` | Cursor | `cursor` | `CURSOR_API_KEY` | yes | `https://api2.cursor.sh` |
| `github-copilot` | GitHub Copilot | `copilot` | `GITHUB_COPILOT_TOKEN` | yes | `https://api.githubcopilot.com` |
| `openrouter` | OpenRouter | `openai` | `OPENROUTER_API_KEY` | no | `https://openrouter.ai/api/v1` |
| `claude-sdk` | Claude Agent SDK | `claude-sdk` | `ANTHROPIC_API_KEY` | no | empty (resolved at runtime) |
| `codex` | OpenAI Codex CLI | `codex` | `CODEX_API_KEY` | yes | empty (resolved at runtime) |
| `groq` | Groq | `openai` | `GROQ_API_KEY` | no | `https://api.groq.com/openai/v1` |
| `together` | Together AI | `openai` | `TOGETHER_API_KEY` | no | `https://api.together.xyz/v1` |
| `deepseek` | DeepSeek | `openai` | `DEEPSEEK_API_KEY` | no | `https://api.deepseek.com/v1` |
| `ollama` | Ollama (local) | `openai` | `OLLAMA_API_KEY` | no | `http://localhost:11434/v1` |
| `kimi` | Kimi (Moonshot) | `openai` | `MOONSHOT_API_KEY` | no | `https://api.moonshot.ai/v1` |
| `kimi-coding` | Kimi Coding Plan | `anthropic` | `KIMI_CODING_API_KEY` | no | `https://api.kimi.com/coding` |
| `minimax` | MiniMax | `openai` | `MINIMAX_API_KEY` | no | `https://api.minimax.io/v1` |
| `minimax-coding` | MiniMax Coding Plan | `anthropic` | `MINIMAX_CODING_API_KEY` | no | `https://api.minimax.io/anthropic` |
| `glm` | GLM (Z.ai) | `openai` | `ZHIPU_API_KEY` | no | `https://api.z.ai/api/paas/v4` |
| `glm-coding` | GLM Coding Plan | `openai` | `ZHIPU_CODING_API_KEY` | no | `https://api.z.ai/api/coding/paas/v4` |

## Runtime Provider Resolution

Provider selection path:

1. CLI `--provider` if set
2. `config.provider.default` otherwise

Model selection path:

1. CLI `--model` if set
2. provider entry `model` if set
3. provider first supported model fallback

## Special Cases

### `claude-sdk`

- resolves credential for `claude-sdk`
- if unavailable, falls back to `anthropic` credential
- may use anthropic base URL fallback

### `codex`

- resolves credential for `codex`
- if unavailable, falls back to `openai` credential
- may use openai base URL fallback

### `cursor`

- credential is parsed into token and machine id via cursor OAuth token parser

### `github-copilot`

- loads stored token from auth store under `github-copilot`
- requires refresh token path for runtime provider instantiation

## Model Registry and Tiering

`ModelRegistry` tracks hardcoded model lists by provider and supports:

- `models_for(provider)`
- `find(provider, model_id)`
- `find_any("provider/model")` or bare model id
- list providers and all models

Remote model refresh flow:

- `refresh_provider_models(provider_id, cache)`
- attempts provider model listing API
- merges fetched models with hardcoded models
- falls back to hardcoded list if fetch fails

## Routing Integration

When routing is enabled (`agent.routing.enabled=true`):

- prompt is classified into tier (`low|medium|high`)
- provider `model_for_tier` selects best matching model
- fallback is provider first model

See `docs/routing.md`.

## Practical Recommendations

- Use explicit `provider/model` notation in high-control workflows.
- Pin `model` under `[provider.<id>]` for reproducibility.
- Keep API keys out of repository files; prefer env vars or OAuth.
- For local/offline style workflows, configure `ollama` with local runtime.
