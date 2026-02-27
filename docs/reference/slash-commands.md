# Slash Command Reference

Source of truth: `crates/tui/src/completion.rs` (`SLASH_COMMANDS`) and `crates/tui/src/input.rs` (dispatch semantics).

## Full Command List

| Command | Description |
| --- | --- |
| `/accent` | change accent color |
| `/agents` | list available agent roles |
| `/analytics` | session analytics and friction detection |
| `/autopilot` | autonomous multi-step execution |
| `/background` | alias for `/bg` |
| `/bg` | manage background tasks |
| `/bug` | generate a bug report |
| `/checkpoint` | save/list/restore session checkpoints |
| `/checkpoint save` | save a named checkpoint |
| `/checkpoint list` | list all checkpoints |
| `/checkpoint restore` | restore a checkpoint by id or name |
| `/changes` | list file changes this session |
| `/clear` | clear the session |
| `/clear queue` | clear the message queue |
| `/commands` | list custom commands |
| `/compact` | compress conversation history |
| `/connect` | connect a provider |
| `/context` | show context window usage |
| `/deep` | deep mode: autonomous research then implement |
| `/diff` | show all file changes this session as diffs |
| `/docs` | view/manage cached documentation (librarian) |
| `/docs add` | cache docs by key from URL or text |
| `/docs get` | retrieve cached docs by key |
| `/docs clear` | clear cached docs |
| `/doctor` | run diagnostics |
| `/editor` | open `$EDITOR` for multi-line input |
| `/enable_exa` | set up Exa web search |
| `/exit` | exit nyzhi |
| `/export` | export conversation as markdown |
| `/handoff` | create session handoff for continuation |
| `/help` | show commands and shortcuts |
| `/hooks` | list configured hooks |
| `/image` | attach image to next prompt |
| `/index` | force re-index codebase |
| `/index off` | disable auto-context for session |
| `/index status` | show index stats |
| `/init` | initialize `.nyzhi/` project config |
| `/init-deep` | generate AGENTS.md files across project |
| `/learn` | create or list learned skills |
| `/login` | show OAuth login status |
| `/mcp` | list connected MCP servers |
| `/memory` | view auto-memory index and status |
| `/memory toggle` | toggle auto-memory |
| `/memory clear` | clear project memory |
| `/model` | choose model |
| `/notepad` | view saved notepads |
| `/notify` | configure notifications |
| `/persist` | enable verify-and-fix mode |
| `/plan` | view or create execution plans |
| `/qa` | run autonomous QA cycling |
| `/quit` | exit nyzhi |
| `/refactor` | structured refactoring workflow |
| `/resume` | restore a saved session |
| `/review` | code review mode |
| `/retry` | resend last prompt |
| `/search` | search session messages |
| `/share` | share session to share.nyzhi.com |
| `/voice` | toggle voice input |
| `/walkthrough` | generate codebase walkthrough diagram |
| `/session delete` | delete saved session |
| `/session rename` | rename current session |
| `/sessions` | list saved sessions |
| `/status` | show session status and usage |
| `/stop` | stop continuation mechanisms |
| `/style` | change output verbosity |
| `/thinking toggle` | toggle thinking display |
| `/settings` | open settings menu |
| `/subagent-config` | show or set model overrides per role |
| `/subagent-config set` | set model for role |
| `/subagent-config reset` | clear model overrides |
| `/team` | spawn coordinated sub-agents |
| `/teams-config` | list teams and config |
| `/teams-config show` | detailed team view |
| `/teams-config set` | set team defaults |
| `/teams-config member` | set member override |
| `/teams-config reset` | clear team overrides |
| `/theme` | choose theme |
| `/think` | toggle extended thinking |
| `/thinking` | set thinking effort level |
| `/todo` | view todo list and progress |
| `/todo enforce on` | enable todo enforcer |
| `/todo enforce off` | pause todo enforcer |
| `/todo clear` | clear todos |
| `/trust` | show or set trust mode |
| `/undo` | undo last file change |
| `/undo all` | undo all file changes in session |
| `/undo git` | restore all files from git HEAD |
| `/verify` | detect/list project checks |
| `/quick` | ad-hoc task with commit discipline |
| `/map` | map codebase stack/architecture/conventions |
| `/init-project` | structured project initialization with research |
| `/profile` | switch model profile |
| `/worktree` | manage git worktrees |
| `/worktree create` | create isolated worktree |
| `/worktree list` | list worktrees |
| `/worktree merge` | merge worktree back |
| `/worktree remove` | remove worktree |
| `/resume-work` | load latest handoff and resume |

## Notes

- custom slash commands from `.nyzhi/commands/*.md` are merged into completion candidates
- completion supports `Tab`/`Shift+Tab` cycling
- command kind (`Instant`, `StreamingSafe`, `Prompt`) determines execution path and streaming safety
