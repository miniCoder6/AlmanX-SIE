# Flux

Flux is a local-first Rust CLI and TUI for recording shell commands, managing aliases, searching command history, computing workflow stats, and querying the local command store.

## What Is In This Repo

This repository is a single Cargo package named `flux` with two binaries:

- `flux` - the main CLI and TUI entrypoint
- `flux-daemon` - an optional Unix socket daemon for ingesting shell events in the background

The code is organized around these modules:

- `src/main.rs` - command dispatch and history ingestion
- `src/cli.rs` - clap CLI definitions
- `src/config.rs` - local config and data paths
- `src/shell.rs` - shell init scripts for Bash, Zsh, Fish, and POSIX shells
- `src/store.rs` - command store, aliases, and WAL persistence
- `src/search.rs` - fuzzy search and alias suggestions
- `src/query.rs` - custom query language parser and executor
- `src/miner.rs` - stats and workflow analysis
- `src/tui/` - Ratatui-based interactive UI

## Runtime Flow

Shell integration calls `flux custom <command>` from a prompt hook or preexec handler. The main binary ingests that command into the local store and writes it to the WAL. If you run `flux-daemon`, it can receive shell events over the Unix socket and persist them in the background.

By default Flux stores data under `~/.flux`:

- `config.json` - optional local config
- `command_store.json` - materialized command store
- `events.wal` - append-only event log
- `aliases` - primary alias file
- `flux.sock` - daemon socket

## Installation

```bash
git clone https://github.com/ishreyanshkumar/flux.git
cd flux
cargo build --release
cp target/release/flux ~/.local/bin/
cp target/release/flux-daemon ~/.local/bin/  # optional
```

If you prefer a one-off local run, `cargo run` launches the TUI by default.

## Shell Integration

Add the relevant init command to your shell profile:

```bash
eval "$(flux init bash)"
eval "$(flux init zsh)"
flux init fish | source
eval "$(flux init posix)"
```

The `posix` shell option also accepts `ksh` as an alias.

## CLI

```bash
flux                         # Launch the TUI
flux tui                     # Launch the TUI explicitly
flux add gs -c "git status"  # Add an alias
flux remove gs               # Remove an alias
flux change gs gst "git status --short"
flux list                    # List aliases
flux suggest -n 10           # Show alias suggestions
flux search "git commit" -l 20
flux stats                   # Show workflow stats
flux query "SELECT * COMMANDS WHERE frequency > 5 LIMIT 10"
flux suppress "git status"   # Remove a command from suggestions
```

## Query Language

Flux includes a small query language for slicing the local command store:

```sql
SELECT * COMMANDS WHERE frequency > 10 ORDER BY frequency LIMIT 20
SELECT * COMMANDS WHERE length > 30 AND frequency > 3
SELECT * WORKFLOWS WHERE frequency > 5
```

## Development

```bash
cargo build
cargo run
cargo run --bin flux-daemon
```

## Notes

The TUI is built with Ratatui and Crossterm. The CLI currently focuses on alias management, fuzzy search, stats, and query execution rather than a multi-crate workspace layout.
