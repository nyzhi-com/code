# Authentication

Nyzhi supports two authentication methods: **API keys** (environment variables or config) and **OAuth** (browser-based login). Multiple accounts per provider are supported with automatic rate-limit rotation.

---

## Credential Resolution Order

When Nyzhi needs credentials for a provider, it checks these sources in order:

1. **Config API key** -- `api_key` field in `[provider.<name>]` section of config.toml
2. **Environment variable** -- provider-specific env var (e.g., `OPENAI_API_KEY`)
3. **Token store** -- stored OAuth/API tokens in `auth.json`
   - If using async resolution: refresh expired tokens first
   - If using sync resolution: use non-expired bearer tokens only
4. **Error** -- if none found, return `AuthError::NoCredential` with a hint about the expected env var and whether OAuth is available

---

## API Keys

The simplest way to authenticate. Set the provider's environment variable:

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="AI..."
```

Or store it in config:

```toml
[provider.openai]
api_key = "sk-..."
```

Or store it in the token store:

```bash
nyz login openai
# Choose "API Key" when prompted, then paste your key
```

### Environment Variable Names

Each built-in provider has a default env var. Custom providers can specify their own via `env_var` in config:

| Provider | Env Var |
|----------|---------|
| openai | `OPENAI_API_KEY` |
| anthropic | `ANTHROPIC_API_KEY` |
| gemini | `GEMINI_API_KEY` |
| openrouter | `OPENROUTER_API_KEY` |
| deepseek | `DEEPSEEK_API_KEY` |
| groq | `GROQ_API_KEY` |
| kimi | `KIMI_API_KEY` |
| minimax | `MINIMAX_API_KEY` |
| glm | `GLM_API_KEY` |

---

## OAuth Login

For providers that support it, OAuth gives you a bearer token without needing a raw API key.

```bash
nyz login gemini      # Google PKCE flow
nyz login openai      # OpenAI device code flow
nyz login anthropic   # Anthropic PKCE flow
nyz login chatgpt     # ChatGPT device code flow (for Codex)
```

### Google / Gemini (PKCE)

1. Nyzhi starts a local HTTP server on a random port.
2. Opens your browser to Google's OAuth consent screen.
3. After you grant access, Google redirects to the local server with an authorization code.
4. Nyzhi exchanges the code for access and refresh tokens using PKCE.
5. Tokens are stored in `auth.json` with a label like `account-1`.

Scope: `https://www.googleapis.com/auth/generative-language.retriever`

### OpenAI (Device Code)

1. Nyzhi requests a device code from OpenAI's device authorization endpoint.
2. Prints a verification URL and user code for you to enter in your browser.
3. Polls until you complete authorization.
4. Exchanges the device code for access and refresh tokens.
5. Tokens are stored for provider `"openai"`.

### Anthropic (PKCE)

Same PKCE flow as Google, with Anthropic's OAuth endpoints.

Scope: `user:inference`

### ChatGPT (Device Code)

Same device code flow as OpenAI, stored under provider `"chatgpt"`. Used for ChatGPT Plus/Pro access (Codex-style).

---

## Token Storage

Tokens are stored in `~/.local/share/nyzhi/auth.json`. The format supports multiple accounts per provider:

```json
{
  "openai": [
    {
      "label": "account-1",
      "token": {
        "access_token": "...",
        "refresh_token": "...",
        "expires_at": "2025-03-01T00:00:00Z",
        "provider": "openai"
      },
      "active": true,
      "rate_limited_until": null
    }
  ]
}
```

### Legacy Migration

Nyzhi previously stored tokens in the OS keyring. On first run, `migrate_from_keyring()` moves any existing keyring tokens to `auth.json`.

---

## Multi-Account Support

You can store multiple accounts per provider:

```bash
nyz login openai    # stores as first account
nyz login openai    # stores as second account (with label)
```

### Rate-Limit Rotation

When a provider returns HTTP 429 (rate limited):

1. The current account is marked `rate_limited_until` for the specified wait period.
2. Nyzhi switches to the next non-rate-limited account.
3. If all accounts are rate-limited, it falls back to exponential backoff.

This happens automatically -- you only see a brief `Retrying...` message.

### Account Management

```bash
nyz whoami                    # show auth status for all providers
nyz logout <provider>         # remove all tokens for a provider
```

---

## Token Refresh

OAuth tokens expire. Nyzhi refreshes them automatically:

- **Google**: Uses the refresh token to get a new access token from Google's token endpoint.
- **OpenAI**: Uses the refresh token with OpenAI's token endpoint.
- **Anthropic**: Uses the refresh token with Anthropic's token endpoint.

A token is considered expired when `expires_at - 60 seconds <= now` (60-second buffer to avoid edge cases).

Refresh only happens in async code paths (the TUI and `nyz run`). The sync `resolve_credential()` path (used in some CLI commands) skips refresh and uses the stored token as-is.

---

## Auth Status

Check your authentication status:

```bash
nyz whoami
```

Output shows each provider's status:

- `env` -- using an API key from environment variable
- `connected` -- using a stored OAuth token
- `not connected` -- no credentials found

Inside the TUI, `/login` shows the same information.

---

## Plugin Auth (Experimental)

The `AuthPlugin` trait exists for custom auth providers, with a `AuthPluginRegistry` for registration. This is currently unused but designed for future extension:

```rust
trait AuthPlugin {
    fn provider_id(&self) -> &str;
    fn provider_name(&self) -> &str;
    fn auth_methods(&self) -> Vec<AuthMethod>;
    fn authorize(&self, method: &AuthMethod) -> Result<AuthorizationResult>;
    fn callback(&self, code: &str, state: &str) -> Result<StoredToken>;
    fn refresh(&self, token: &StoredToken) -> Result<StoredToken>;
}
```
