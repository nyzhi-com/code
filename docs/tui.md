# Terminal UI

The TUI (`nyzhi-tui`) is built with `ratatui` and `crossterm`, and drives both interactive chat and tool execution state.

## App modes

- `Input`: normal typing and command mode.
- `Streaming`: model output/tool execution in progress.
- `AwaitingApproval`: approval gate waiting for `y` / `n`.

## Slash commands (completion source)

Current built-in slash commands include:

- `/accent`
- `/agents`
- `/autopilot`
- `/background`, `/bg`
- `/bug`
- `/changes`
- `/clear`, `/clear queue`
- `/commands`
- `/compact`
- `/connect`
- `/context`
- `/doctor`
- `/editor`
- `/enable_exa`
- `/exit`, `/quit`
- `/export`
- `/handoff`
- `/help`
- `/hooks`
- `/image`
- `/init`
- `/init-deep`
- `/learn`
- `/login`
- `/mcp`
- `/model`
- `/notepad`
- `/notify`
- `/persist`
- `/plan`
- `/qa`
- `/refactor`
- `/resume`
- `/retry`
- `/search`
- `/session delete`
- `/session rename`
- `/sessions`
- `/status`
- `/stop`
- `/style`
- `/team`
- `/theme`
- `/think`
- `/thinking`
- `/todo`
- `/todo enforce on`
- `/todo enforce off`
- `/todo clear`
- `/trust`
- `/undo`
- `/undo all`
- `/verify`

`/help` also shows live command guidance and keyboard shortcuts.

## Keybindings

### Global and input editing

- `Ctrl+C`: quit
- `Ctrl+K`: open command palette
- `Ctrl+R`: history search mode
- `Ctrl+U`: clear input to start
- `Ctrl+W`: delete previous word
- `Ctrl+A` / `Ctrl+E`: cursor to start/end of line

### Completion and send behavior

- `Tab`:
  - if completion open -> next completion
  - if input empty -> cycle thinking level
  - otherwise -> open completion
- `Shift+Tab`:
  - if completion open -> previous completion
  - if input empty and no completion -> enter/open plan transition selector
- `Enter`: submit (or accept completion if menu open)
- `Alt+Enter` / `Shift+Enter`: insert newline

### Search and navigation

- `Ctrl+N` / `Ctrl+P`: next/prev match for active `/search`
- `Esc`: clear completion/search; during streaming, cancel foreground task
- `PageUp` / `PageDown`: scroll transcript

### Streaming/background task controls

- `Ctrl+B` (while streaming): move current task to background
- `Ctrl+F` (double-press in input mode): kill all background tasks
- `& <prompt>`: run prompt directly in background queue

## Theme system

Presets:

- `nyzhi-dark`
- `nyzhi-light`
- `tokyonight`
- `catppuccin-mocha`
- `dracula`
- `solarized-dark`
- `solarized-light`
- `gruvbox-dark`

Accent options:

- `copper` (default)
- `blue`
- `orange`
- `emerald`
- `violet`
- `rose`
- `amber`
- `cyan`
- `red`
- `pink`
- `teal`
- `indigo`
- `lime`
- `monochrome`

Color overrides are supported via `[tui.colors]` hex values.

## Completion behavior

Completion contexts:

- Slash commands (`/`)
- `@` mentions (`@path`)
- file path args for `/image ...`

Candidate limits:

- max candidates: 50
- max visible rows: 12

## Search, history, and session UX

- `/search <query>` highlights matches in transcript.
- `/sessions` and `/resume` open selector UIs.
- `/session rename` and `/session delete` operate on saved sessions.
- `/export` writes markdown exports.

## Notifications

Turn completion notifications come from `[tui.notify]`:

- bell (`true` by default)
- desktop (`false` by default)
- minimum duration threshold (`5000ms` default)

## Trust mode in TUI

`/trust` supports:

- `off`
- `limited`
- `autoedit`
- `full`

## Notes

- Shortcut docs in old guides that mapped `Ctrl+A` to accent cycling are outdated; `Ctrl+A` is cursor-home in current code.
- Theme/accent pickers are command-driven (`/theme`, `/accent`) and via selector shortcuts (`Ctrl+T` for theme picker).
