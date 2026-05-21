# AlmanX

**A local-first shell intelligence engine.**

AlmanX silently watches your shell commands, learns what you type most often,
and suggests smart, conflict-free aliases to save you keystrokes — no cloud,
no telemetry, no subscriptions.

---

## Quick Start

```bash
# 1. Build
cargo build --release

# 2. Wire up your shell (pick one)
eval "$(almanx init bash)"   # → add to ~/.bashrc
eval "$(almanx init zsh)"    # → add to ~/.zshrc
almanx init fish | source    # → add to config.fish

# 3. Use your shell as normal for a day or two, then:
almanx suggest

# 4. Add an alias from the suggestions
almanx add gp "git push"

# 5. Or just open the TUI
almanx
```

---

## How It Works

```
Your shell  ──preexec hook──►  almanx record "<cmd>"
                                      │
                               ~/.almanx/database.json
                                      │
                         frecency score = frequency × recency × length_weight
                                      │
                           almanx suggest  (top-N by score)
                                      │
                         Alias suggester checks PATH + existing aliases
                         for conflicts, then ranks candidates
                                      │
                           You: almanx add <alias> <command>
                                      │
                           ~/.almanx/aliases  ←  alias gp='git push'
```

### Frecency Scoring

| Recency band     | Multiplier |
|------------------|------------|
| Used < 1 hour ago    | ×4.0  |
| Used < 1 day ago     | ×2.0  |
| Used < 1 week ago    | ×0.5  |
| Older                | ×0.25 |

`score = multiplier × frequency × command_length^0.6`

Longer commands score higher because they save more typing.

---

## Commands

| Command | Description |
|---|---|
| `almanx` | Open TUI |
| `almanx suggest [-n N]` | Print top-N alias suggestions |
| `almanx add <alias> <cmd>` | Add an alias |
| `almanx remove <alias>` | Remove an alias |
| `almanx rename <old> <new>` | Rename an alias |
| `almanx list` | List all tracked aliases |
| `almanx dismiss <cmd>` | Never suggest this command again |
| `almanx init bash\|zsh\|fish` | Print shell integration snippet |

---

## TUI Keys

| Key | Action |
|---|---|
| `/` | Search commands |
| `↑ ↓` | Navigate list |
| `a` / `Enter` | Add alias for selected command |
| `d` | Dismiss selected command |
| `l` | List all aliases |
| `q` / `Esc` | Quit |

---

## File Layout

```
~/.almanx/
├── database.json     # command frequency/timing data
├── deleted.json      # dismissed commands
├── config.json       # tracked alias file paths
└── aliases           # default alias output file
```

---

## Architecture

```
src/
├── main.rs              # CLI dispatch
├── shell.rs             # shell init script generator
├── cli/
│   ├── mod.rs
│   └── args.rs          # clap argument definitions
├── database/
│   ├── mod.rs
│   ├── structs.rs       # Database, Command, DeletedCommands
│   ├── ops.rs           # record, tombstone, top-N
│   ├── scoring.rs       # frecency formula + decay
│   └── persistence.rs   # JSON load/save
├── ops/
│   ├── mod.rs
│   ├── alias_file.rs    # read/write shell alias files
│   └── suggest.rs       # alias name suggestion engine
└── tui/
    ├── mod.rs           # terminal setup + event loop
    ├── app.rs           # all TUI state
    ├── events.rs        # keyboard event dispatch
    └── render.rs        # ratatui drawing code
```
