# Terminal UI

Nyzhi's TUI is built on [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm). It provides a full-featured chat interface with syntax highlighting, theming, completion, and session management.

---

## Modes

The TUI operates in three modes:

| Mode | Description |
|------|-------------|
| **Input** | Normal state. You type messages, use slash commands, and browse history. |
| **Streaming** | Active while the agent is generating a response. Shows thinking, text, and tool calls in real-time. |
| **AwaitingApproval** | A tool needs your approval. Press `y` to approve, `n` to deny. |

---

## Slash Commands

| Command | Description |
|---------|-------------|
| `/help` | Show all commands and shortcuts |
| `/model [name]` | List or switch models |
| `/image <path>` | Attach image to next prompt |
| `/login` | Show OAuth login status |
| `/init` | Initialize `.nyzhi/` project config |
| `/mcp` | List connected MCP servers and tools |
| `/commands` | List custom commands |
| `/hooks` | List configured hooks |
| `/clear` | Clear the current session |
| `/compact` | Compress conversation history to save context |
| `/sessions [query]` | List saved sessions (optionally filter by title) |
| `/resume <id>` | Restore a saved session |
| `/session delete <id>` | Delete a saved session |
| `/session rename <title>` | Rename the current session |
| `/theme` | Cycle through theme presets |
| `/accent` | Cycle through accent colors |
| `/trust [mode]` | Show or set trust mode (off/limited/full) |
| `/editor` | Open `$EDITOR` for multi-line input |
| `/retry` | Resend the last prompt |
| `/undo` | Undo the last file change |
| `/undo all` | Undo all file changes in this session |
| `/changes` | List all file changes made in this session |
| `/export [path]` | Export conversation as markdown |
| `/search <query>` | Search session messages |
| `/notify` | Toggle notification settings (bell, desktop, duration) |
| `/autopilot <idea>` | Start autonomous 5-phase execution |
| `/team N <task>` | Spawn N coordinated sub-agents |
| `/plan [name]` | List or view saved plans |
| `/persist` | Activate verify/fix loop mode |
| `/qa` | Activate autonomous QA cycling |
| `/verify` | Show detected verification checks |
| `/todo` | View the current todo list |
| `/learn [name]` | List learned skills or create a new one |
| `/notepad [topic]` | List or view notepad entries |
| `/quit` | Exit nyzhi |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Tab** | Autocomplete slash commands, `@`-mention file paths |
| **Alt+Enter** | Insert a newline (multi-line input) |
| **Ctrl+R** | Reverse history search |
| **Ctrl+N** | Next search match |
| **Ctrl+P** | Previous search match |
| **Ctrl+T** | Cycle to next theme preset |
| **Ctrl+A** | Cycle to next accent color |
| **Ctrl+L** | Clear the screen |
| **PageUp** | Scroll up through messages |
| **PageDown** | Scroll down through messages |
| **Ctrl+C** | Exit |

---

## Themes

Eight built-in theme presets, selectable via `/theme`, Ctrl+T, or config:

| Preset | Mode | Description |
|--------|------|-------------|
| **Nyzhi Dark** | Dark | Pure black background (`#000000`). The signature theme. |
| **Nyzhi Light** | Light | Warm cream background (`#F5F0E8`). Paper-like feel. |
| **Tokyo Night** | Dark | Deep navy with blue-purple accents. |
| **Catppuccin Mocha** | Dark | Warm dark with pastel highlights. |
| **Dracula** | Dark | Classic dark with vivid colors. |
| **Solarized Dark** | Dark | Ethan Schoonover's dark palette. |
| **Solarized Light** | Light | Ethan Schoonover's light palette. |
| **Gruvbox Dark** | Dark | Retro groove with warm earth tones. |

### Theme Structure

Each theme defines 16 color slots:

- **Surfaces**: `bg_page`, `bg_surface`, `bg_elevated`, `bg_sunken`
- **Text**: `text_primary`, `text_secondary`, `text_tertiary`, `text_disabled`
- **Borders**: `border_default`, `border_strong`
- **Accent**: `accent`, `accent_muted`
- **Semantic**: `success`, `danger`, `warning`, `info`

Any slot can be overridden with a hex color in `[tui.colors]`. See [configuration.md](configuration.md).

---

## Accents

Fourteen accent colors, selectable via `/accent`, Ctrl+A, or config:

| Accent | Color | Hex |
|--------|-------|-----|
| **copper** | Warm bronze | `#C49A6C` |
| **blue** | | `#3B82F6` |
| **orange** | | `#FF6600` |
| **emerald** | | `#10B981` |
| **violet** | | `#8B5CF6` |
| **rose** | | `#F43F5E` |
| **amber** | | `#F59E0B` |
| **cyan** | | `#06B6D4` |
| **red** | | `#EF4444` |
| **pink** | | `#EC4899` |
| **teal** | | `#14B8A6` |
| **indigo** | | `#6366F1` |
| **lime** | | `#84CC16` |
| **monochrome** | Mode-dependent | Light: `#825B32`, Dark: `#C49A6C` |

Copper is the default accent, reflecting Nyzhi's warm metallic design language. The monochrome accent is unique -- it shifts from warm brown in light mode to copper/bronze in dark mode.

Each accent produces a base color and a muted variant (blended with the theme background at ~15% opacity) used for highlights and selections.

---

## Tab Completion

Press **Tab** to autocomplete:

- **Slash commands**: Type `/` then Tab to cycle through commands.
- **`@`-mention files**: Type `@` then a partial path. Tab completes from the project file tree.
- **File path arguments**: After commands like `/image`, Tab completes file paths.

The completion system detects context (command vs. file path vs. `@`-mention) and generates candidates accordingly.

---

## Multi-Line Input

Three ways to enter multi-line input:

1. **Alt+Enter** -- insert a newline in the input box.
2. **`/editor`** -- opens your `$EDITOR` (or `$VISUAL`) with the current input. Save and quit to submit.
3. **Bracketed paste** -- paste multi-line text from your clipboard. The terminal's bracketed paste mode preserves newlines.

---

## Input History

Input history persists across sessions. Navigate with:

- **Up/Down arrows** -- cycle through previous inputs
- **Ctrl+R** -- reverse search through history (type to filter, Enter to select, Esc to cancel)

---

## In-Session Search

`/search <query>` highlights matching messages in the chat view. Navigate matches with:

- **Ctrl+N** -- jump to next match
- **Ctrl+P** -- jump to previous match
- **Esc** -- clear search highlights

---

## Syntax Highlighting

Code blocks in agent responses are highlighted using [syntect](https://github.com/trishume/syntect). Language detection is automatic based on the code fence language tag. Inline markdown formatting (bold, italic, code) is also rendered.

---

## Notifications

When a turn completes:

- **Terminal bell** -- enabled by default. Audible alert via `\a`.
- **Desktop notification** -- via `notify-rust`. Disabled by default.
- **Duration threshold** -- notifications only fire if the turn took longer than `min_duration_ms` (default: 5000ms).

Configure via `/notify` in the TUI or `[tui.notify]` in config.

---

## Conversation Export

`/export [path]` saves the current session as a markdown file. If no path is given, it uses a default path based on the session title.

The export includes:

- Session metadata (title, date, provider, model)
- All user messages and agent responses
- Tool call summaries with output
- Formatted code blocks

---

## Welcome Screen

On first launch (or with no active session), the TUI shows a welcome screen with:

- The triskelion logo (ASCII art)
- Provider and model information
- Quick-start hints

---

## Update Banner

When an update is available, a banner appears at the top of the screen:

- **`[u]`** -- Apply the update now
- **`[s]`** -- Skip for this session
- **`[i]`** -- Ignore this version permanently
