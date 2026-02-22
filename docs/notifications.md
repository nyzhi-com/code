# Notifications

Nyzhi can notify you when an agent turn completes, both locally (terminal bell, desktop notification) and remotely (webhook, Telegram, Discord, Slack).

---

## Local Notifications

### Terminal Bell

Plays a terminal bell (`\a`) when a turn finishes. Enabled by default.

```toml
[tui.notify]
bell = true
```

### Desktop Notifications

Sends a desktop notification via `notify-rust`. Disabled by default.

```toml
[tui.notify]
desktop = true
```

On macOS, this uses the native notification center. On Linux, it uses `libnotify` or a compatible notification daemon.

### Duration Threshold

Notifications only fire if the turn took longer than the threshold. This avoids noisy alerts for quick responses.

```toml
[tui.notify]
min_duration_ms = 5000         # default: 5000 (5 seconds)
```

### TUI Toggle

Use `/notify` in the TUI to view and toggle notification settings without editing config.

---

## External Notifications

External notifications are sent via HTTP after agent turns complete. Configure them in the `[notify]` section of config.toml.

### Webhook

Send a POST request to any URL:

```toml
[notify]
webhook = { url = "https://hooks.example.com/nyzhi" }
```

The payload is a JSON object with the notification message.

### Telegram

Send messages to a Telegram chat via the Bot API:

```toml
[notify]
telegram = { bot_token = "123456:ABC-DEF", chat_id = "-1001234567890" }
```

To set up:

1. Create a bot via [@BotFather](https://t.me/BotFather) and get the bot token.
2. Add the bot to your group or channel.
3. Get the chat ID (use `getUpdates` API or a bot like @userinfobot).

### Discord

Send messages to a Discord channel via webhook:

```toml
[notify]
discord = { webhook_url = "https://discord.com/api/webhooks/123/abc" }
```

To set up:

1. Go to your Discord channel settings.
2. Under Integrations, create a webhook.
3. Copy the webhook URL.

### Slack

Send messages to a Slack channel via webhook:

```toml
[notify]
slack = { webhook_url = "https://hooks.slack.com/services/T00/B00/xxx" }
```

To set up:

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps).
2. Enable Incoming Webhooks.
3. Add a webhook to your workspace and select a channel.
4. Copy the webhook URL.

---

## Multiple Channels

You can configure multiple notification channels simultaneously. All configured channels receive the notification:

```toml
[notify]
webhook = { url = "https://hooks.example.com/nyzhi" }
telegram = { bot_token = "...", chat_id = "..." }
discord = { webhook_url = "https://..." }
slack = { webhook_url = "https://..." }
```

---

## Message Format

Notification messages include:

- A completion indicator
- The session context (what the agent was working on)

The exact format varies by channel (plain text for Telegram, JSON for webhooks, etc.).
