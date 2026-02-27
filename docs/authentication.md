# Authentication

Source of truth:

- `crates/auth/src/lib.rs`
- `crates/auth/src/token_store.rs`
- `crates/auth/src/oauth/*`
- `crates/cli/src/main.rs` (`login`, `logout`, `whoami`)
- `crates/tui/src/app.rs` + `crates/tui/src/input.rs` (`/connect`)

## Default Auth Flow

For interactive use, default to:

- `nyz` -> `/connect`

`/connect` opens provider selection and then uses:

- OAuth when available
- API key entry as fallback

CLI fallback remains:

- `nyz login [provider]`

## Credential Resolution Order

`resolve_credential(provider, config_key)` resolves in this order:

1. provider `api_key` from config (`[provider.<id>].api_key`)
2. provider env var (`OPENAI_API_KEY`, etc.)
3. stored token/account from local token store

If no credentials are found:

- error includes expected env var name
- OAuth-capable providers include hint to use `/connect` or run `nyz login <provider>`

## Async Resolution

`resolve_credential_async` follows similar precedence, but can refresh OAuth token via `oauth::refresh::refresh_if_needed` before fallback to stored token.

## CLI Commands

```bash
nyz login [provider]
nyz logout <provider>
nyz whoami
```

Behavior:

- `login` without provider shows built-in provider picker
- OAuth is attempted for OAuth-capable providers
- API key prompt is used when OAuth is unavailable or fails
- `logout` removes stored token entries for provider
- `whoami` prints status for built-ins plus custom configured providers

## Auth Status Values

`auth_status(provider)` returns:

- `env` when env var is available
- `connected` when token/account exists in store
- `not connected` otherwise

## Token Store and Multi-account Model

Token storage is JSON-based (`auth.json`) and supports multi-account per provider.

Important structures:

- `StoredToken`:
  - `access_token`
  - `refresh_token`
  - `expires_at`
  - `provider`
- `AccountEntry`:
  - `label`
  - `token`
  - `active`
  - `rate_limited_until`

Capabilities:

- active-account selection
- labeled account storage
- account listing/removal
- rate-limit rotation (`rotate_on_rate_limit`)

## Rate Limit Rotation

`handle_rate_limit(provider)` attempts rotation to another account for 60 seconds.

If another account is available:

- active account is marked rate-limited
- next eligible account becomes active
- credential is returned to runtime

If no eligible fallback account exists:

- returns `None`

## Storage Locations

- auth store: `<data_local_dir>/nyzhi/auth.json`
  - typically `~/.local/share/nyzhi/auth.json` (platform dependent)
- migration from legacy keyring entries exists for selected providers

## Provider Env Vars

Provider env vars are defined in `BUILT_IN_PROVIDERS` (`crates/config/src/lib.rs`), including:

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `CURSOR_API_KEY`
- `GITHUB_COPILOT_TOKEN`
- `OPENROUTER_API_KEY`
- `GROQ_API_KEY`
- `TOGETHER_API_KEY`
- `DEEPSEEK_API_KEY`
- `OLLAMA_API_KEY`
- `MOONSHOT_API_KEY`
- `MINIMAX_API_KEY`
- `ZHIPU_API_KEY`
- provider-specific coding-plan variants

See `docs/providers.md` for full provider metadata.

## Security Notes

- `nyz logout <provider>` clears local stored token entries for that provider.
- Do not commit config files containing `api_key`.
- Prefer environment variables or OAuth token storage for shared repositories.
