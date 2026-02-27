# TUI

Source of truth:

- `crates/tui/src/app.rs`
- `crates/tui/src/input.rs`
- `crates/tui/src/completion.rs`

## App Modes

The TUI tracks explicit interaction modes:

- `Input`
- `Streaming`
- `AwaitingApproval`
- `AwaitingUserQuestion`

These modes drive keyboard routing, prompt dispatch, and UI panels.

## Input Model

- single-line input by default
- multiline insert with:
  - `Alt+Enter`
  - `Shift+Enter` (kitty protocol path)
- `!<shell command>` runs shell command and injects output
- `& <prompt>` dispatches background task
- `@path` injects file/directory context references
- `/editor` opens `$VISUAL` or `$EDITOR` (fallback `vi`)

## Completion Model

Completion contexts (`CompletionContext`):

- `SlashCommand`
- `AtMention`
- `FilePath`

Behavior:

- `Tab` opens/cycles completion
- `Shift+Tab` cycles backward
- `Esc` closes completion
- `@` uses fuzzy file search
- `/image <path>` uses file path completion

## Keybindings

Core keys:

- `Enter`: submit or accept completion
- `Tab` / `Shift+Tab`: completion forward/backward
- `Esc`: dismiss completion / clear search / clear input
- `Up` / `Down`: input history or selector navigation
- `Ctrl+R`: reverse history search
- `Ctrl+K`: command palette
- `Ctrl+T`: theme selector
- `Ctrl+,`: settings panel
- `Ctrl+L`: clear session
- `Ctrl+U`: clear to line start
- `Ctrl+W`: delete previous word
- `Ctrl+A` / `Ctrl+E`: line start/end
- `Ctrl+B`: move current streaming task to background
- `Ctrl+F` (double press): kill all background tasks
- `Ctrl+N` / `Ctrl+P`: search next/previous match when session search is active
- `Ctrl+C`: exit

## Panels and Selectors

Built-in panels include:

- plan panel
- todo panel
- settings panel
- model/provider/theme/accent selectors
- session picker
- command palette

## Slash Commands

Slash commands are declared in `SLASH_COMMANDS` and dispatched in `input.rs`.

### General utility

- `/help`
- `/clear`
- `/clear queue`
- `/quit`
- `/exit`
- `/status`
- `/commands`
- `/settings`

### Provider, model, and auth

- `/model`
- `/connect`
- `/login`
- `/trust`
- `/thinking`
- `/thinking toggle`
- `/think`
- `/voice`

### Session and context

- `/sessions`
- `/resume`
- `/session delete`
- `/session rename`
- `/context`
- `/compact`
- `/retry`
- `/search`
- `/export`
- `/handoff`

### Project tooling and runtime control

- `/index`
- `/index status`
- `/index off`
- `/mcp`
- `/hooks`
- `/memory`
- `/memory toggle`
- `/memory clear`
- `/docs`
- `/docs add`
- `/docs get`
- `/docs clear`
- `/verify`
- `/style`
- `/notify`

### Theme and UX

- `/theme`
- `/accent`
- `/bg`
- `/background`

### Planning and execution helpers

- `/autopilot`
- `/deep`
- `/qa`
- `/review`
- `/refactor`
- `/walkthrough`
- `/quick`
- `/map`
- `/profile`
- `/init-project`

### Teams and subagents

- `/team`
- `/teams-config`
- `/teams-config show`
- `/teams-config set`
- `/teams-config member`
- `/teams-config reset`
- `/subagent-config`
- `/subagent-config set`
- `/subagent-config reset`

### Worktree and undo

- `/worktree`
- `/worktree create`
- `/worktree list`
- `/worktree merge`
- `/worktree remove`
- `/undo`
- `/undo all`
- `/undo git`

### Misc

- `/agents`
- `/analytics`
- `/bug`
- `/changes`
- `/diff`
- `/doctor`
- `/editor`
- `/enable_exa`
- `/image`
- `/init`
- `/init-deep`
- `/learn`
- `/notepad`
- `/persist`
- `/plan`
- `/stop`
- `/todo`
- `/todo enforce on`
- `/todo enforce off`
- `/todo clear`
- `/resume-work`

Full list with one-line descriptions: `docs/reference/slash-commands.md`.

## Command Kinds

Commands are classified into kinds:

- `Instant`: local handling without model turn
- `StreamingSafe`: allowed while streaming
- `Prompt`: turns into agent prompt dispatch

## Background Task Model

- foreground turns can be moved to background (`Ctrl+B`)
- background queue is tracked by task id/label/start time
- message queue supports prompt buffering while runtime is busy

## Team and Subagent UX

- `/team <N> <task>` asks the model to fan out into multiple subagents
- `/subagent-config` controls session-scoped role->model overrides
- `/teams-config` inspects/updates team defaults and member overrides

## Notes and Caveats

- `--teammate-mode` is parsed by CLI, but currently TUI/runtime behavior is effectively in-process.
- Some slash commands are aliases (`/background` -> `/bg`, `/quit` -> exit path).
