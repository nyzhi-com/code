# Notifications

Source of truth:

- `crates/config/src/lib.rs` (`tui.notify`, `external_notify`)
- `crates/tui/src/input.rs` (`/notify` command handling)
- `crates/core/src/notify.rs`
- `crates/core/src/agent/mod.rs` (teammate idle mailbox notifications)

## Notification Surfaces

There are three distinct notification channels:

1. TUI local notifications (`[tui.notify]`)
2. External webhook-style notifications (`[external_notify]`)
3. Internal team mailbox idle notifications

## TUI Notification Config

`[tui.notify]`:

- `bell` (default `true`)
- `desktop` (default `false`)
- `min_duration_ms` (default `5000`)

TUI slash support:

- `/notify`
- `/notify bell on|off`
- `/notify desktop on|off`
- `/notify duration <ms>`

## External Notification Config

Config keys:

- `external_notify.webhook_url`
- `external_notify.telegram_bot_token`
- `external_notify.telegram_chat_id`
- `external_notify.discord_webhook_url`
- `external_notify.slack_webhook_url`

Core notify module (`crates/core/src/notify.rs`) includes sender implementations for:

- generic webhook JSON payload
- Telegram bot API
- Discord webhook
- Slack webhook

## Team Idle Notifications

When a non-lead teammate completes a turn, agent runtime can send an idle notification message to the team lead inbox (`MessageType::IdleNotification`).

This is team mailbox signaling, not desktop/webhook notification.

## Practical Guidance

- enable `desktop` notifications only for long-running tasks
- tune `min_duration_ms` to avoid noisy short-turn alerts
- store webhook credentials in private config or environment, not in repository files
- for team workflows, monitor inbox with `read_inbox` and `/teams-config`
