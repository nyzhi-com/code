# Authentication

`nyzhi-auth` supports API keys, OAuth tokens, token refresh, and multi-account rotation.

## Credential resolution order

### Synchronous path (`resolve_credential`)

1. Config key (`[provider.<id>].api_key`)
2. Environment variable (derived from provider definition)
3. Token store (`auth.json`)
   - if token has refresh token and is **not expired**, use bearer token
   - if token has no refresh token, treat as API key
4. Error with provider/env hint

### Async path (`resolve_credential_async`)

1. Config key
2. Environment variable
3. `refresh_if_needed(provider)` (refreshes expired OAuth token when possible)
4. Token store fallback
5. Error with provider/env hint

## API key auth

```bash
export OPENAI_API_KEY="..."
export ANTHROPIC_API_KEY="..."
export GEMINI_API_KEY="..."
```

Or store inline:

```toml
[provider.openai]
api_key = "..."
```

## OAuth flows

Implemented OAuth entrypoints:

- `openai` (PKCE)
- `gemini` / `google` (PKCE)
- `anthropic` (PKCE)
- `chatgpt` (delegates to OpenAI login and relabels provider)
- `cursor` (local Cursor credential extraction)

### OpenAI PKCE

- authorize URL: `https://auth.openai.com/oauth/authorize`
- token URL: `https://auth.openai.com/oauth/token`
- fixed redirect: `http://localhost:1455/auth/callback`
- scope: `openid profile email offline_access`

### Google/Gemini PKCE

- authorize URL: `https://accounts.google.com/o/oauth2/v2/auth`
- token URL: `https://oauth2.googleapis.com/token`
- redirect: random local port (`127.0.0.1:<port>/oauth2callback`)
- scopes include:
  - `openid`
  - `email`
  - `cloud-platform`
  - `generative-language`
  - `cloudaicompanion`

### Anthropic PKCE

- authorize URL: `https://console.anthropic.com/oauth/authorize`
- token URL: `https://console.anthropic.com/oauth/token`
- redirect: random local port (`127.0.0.1:<port>/oauth2callback`)
- scope: `user:inference`

### ChatGPT

`chatgpt::login()` reuses OpenAI login and stores the token under provider id `chatgpt`.

### Cursor

Cursor auth is not browser OAuth in nyzhi:

- reads Cursor SQLite state DB (`state.vscdb`) from OS-specific Cursor global storage path
- extracts:
  - `cursorAuth/accessToken`
  - `cursorAuth/cachedSignUpType` (machine id)
- stores combined value as `access_token:::machine_id`

## CLI login behavior

`nyz login`:

- prompts for provider when omitted
- attempts OAuth if the provider definition says `supports_oauth = true`
- on OAuth failure (or non-OAuth provider), falls back to API key prompt

## Token storage

Auth data is persisted at:

- `<data_local_dir>/nyzhi/auth.json`

Schema supports multiple accounts per provider:

```json
{
  "openai": [
    {
      "label": "account-2",
      "token": {
        "access_token": "...",
        "refresh_token": "...",
        "expires_at": 1760000000,
        "provider": "openai"
      },
      "active": true,
      "rate_limited_until": null
    }
  ]
}
```

Legacy keyring migration still exists (`migrate_from_keyring()`).

## Refresh behavior

Expired check uses a 60-second buffer:

- expired when `now >= expires_at - 60`

Refresh providers:

- `gemini` / `google`
- `openai` / `chatgpt`
- `anthropic`

If refresh succeeds, token is overwritten in store.

## Multi-account rotation on 429

`handle_rate_limit(provider)` can rotate to next account:

- marks active account as rate-limited for `wait_seconds`
- activates next available non-rate-limited account
- if none available, restores first account active and returns `None`

## Auth status

`nyz whoami` status values:

- `env`
- `connected`
- `not connected`

For providers with multiple accounts, CLI prints account labels, active marker, and rate-limit marker.

## Provider env vars

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `CURSOR_API_KEY`
- `OPENROUTER_API_KEY`
- `GROQ_API_KEY`
- `TOGETHER_API_KEY`
- `DEEPSEEK_API_KEY`
- `OLLAMA_API_KEY`
- `MOONSHOT_API_KEY`
- `KIMI_CODING_API_KEY`
- `MINIMAX_API_KEY`
- `MINIMAX_CODING_API_KEY`
- `ZHIPU_API_KEY`
- `ZHIPU_CODING_API_KEY`
- `CODEX_API_KEY`
