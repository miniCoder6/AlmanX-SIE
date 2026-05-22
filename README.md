# 🚀 AlmanX — Shell Intelligence Engine

AlmanX is a **local-first, zero-dependency shell intelligence platform** written entirely in Rust. It silently tracks your terminal activity, mines workflow patterns, provides sub-millisecond fuzzy search, and suggests personalized shell aliases.

---

## Features

| Feature | Status | Description |
|---|---|---|
| Shell hooks (bash/zsh/fish) | ✅ | Auto-records every command you run |
| Frecency-scored command DB | ✅ | Scores commands by frequency + recency |
| Alias suggestion engine | ✅ | Semantic, abbreviation, and vowel-strip suggestions |
| Fuzzy search (`almanx search`) | ✅ | Jaro-Winkler + substring + recency scoring |
| Workflow mining (`almanx workflows`) | ✅ | N-gram DAG reconstruction from session history |
| Productivity analytics (`almanx stats`) | ✅ | Keystroke savings, top commands, directories |
| Interactive TUI | ✅ | Full ratatui interface with all features |
| Event log (JSONL) | ✅ | Append-only log at `~/.almanx/events.jsonl` |
| Alias file management | ✅ | Add/remove/rename/list aliases |

---

## Quick Start

```bash
# 1. Build and install
bash install.sh

# 2. Wire up your shell (add to ~/.bashrc or ~/.zshrc)
eval "$(almanx init bash)"    # bash
eval "$(almanx init zsh)"     # zsh
almanx init fish | source     # fish

# 3. Reload your shell
source ~/.bashrc

# 4. Use your terminal normally for a while, then:
almanx                        # Open interactive TUI
almanx suggest                # Get alias suggestions
almanx stats                  # View productivity report
almanx search "git"           # Fuzzy search history
almanx workflows              # View workflow patterns
```

---

## CLI Reference

```
almanx                          Launch interactive TUI
almanx search <query>           Fuzzy search command history
almanx stats                    Show productivity analytics
almanx workflows [--min-freq N] Show mined workflow DAGs
almanx suggest [--num N]        Suggest shell aliases
almanx add <alias> <command>    Add an alias
almanx remove <alias>           Remove an alias
almanx rename <old> <new>       Rename an alias
almanx list                     List all tracked aliases
almanx dismiss <command>        Stop tracking a command
almanx init bash|zsh|fish       Print shell integration snippet
```

---

## TUI Keybindings

| Key | Action |
|---|---|
| `/` | Search/filter commands |
| `↑ ↓` or `j k` | Navigate |
| `a` or `Enter` | Add alias for selected command |
| `d` | Dismiss command |
| `l` | List all aliases |
| `w` | View workflow patterns |
| `q` | Quit |

---

## Architecture

```
~/.almanx/
├── database.json    # Frecency-scored command index
├── deleted.json     # Tombstoned (dismissed) commands
├── events.jsonl     # Append-only event log (full telemetry)
├── config.json      # User configuration
└── aliases          # Generated shell alias file
```

Data flow:
1. Shell hook calls `almanx record <cmd> --cwd <dir> --exit-code <n> --duration <ms>`
2. Command goes into `database.json` (frecency scoring) AND `events.jsonl` (raw log)
3. `almanx search` / `almanx workflows` / `almanx stats` query `events.jsonl`
4. TUI reads from `database.json` for fast ranked retrieval

---

## Running Tests

```bash
# Build first
cargo build --release

# Run integration tests
bash integration_tests/test_hooks.sh

# Run unit tests
cargo test
```
