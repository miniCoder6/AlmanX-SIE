# ⚡ Flux — Shell Intelligence Platform

> A local-first, high-performance developer workflow intelligence platform built entirely in **Rust**. Flux acts as a programmable intelligence layer for your terminal — capturing shell activity in real-time, reconstructing workflow DAGs, compressing repetitive actions, and providing sub-millisecond context-aware command suggestions.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Installation](#installation)
- [Shell Integration Flow](#shell-integration-flow)
- [Data Pipeline](#data-pipeline)
- [Storage Layer](#storage-layer)
- [Search Engine](#search-engine)
- [Query Language](#query-language)
- [Workflow Mining](#workflow-mining)
- [TUI State Machine](#tui-state-machine)
- [Daemon Architecture](#daemon-architecture)
- [Alias Suggestion Engine](#alias-suggestion-engine)
- [Scoring Algorithm](#scoring-algorithm)
- [CLI Reference](#cli-reference)
- [TUI Keybindings](#tui-keybindings)
- [Configuration](#configuration)
- [Performance Targets](#performance-targets)
- [Development](#development)

---

## Overview

Most shell history tools wrap SQLite and call it done. Flux implements primitives:

| Primitive      | Implementation                                  |
| -------------- | ----------------------------------------------- |
| Storage        | Custom append-only WAL with periodic compaction |
| Indexing       | Radix Trie for O(k) prefix search               |
| Relevance      | BM25 scorer with configurable k1/b parameters   |
| Fuzzy match    | Levenshtein with early termination              |
| Workflow model | Session-clustered command transition DAG        |
| Query          | Hand-written recursive descent parser           |

---

## Architecture

```mermaid
graph TD
    SHELL["🐚 Shell Integration Layer<br/>(Bash · Zsh · Fish · POSIX)"]
    CORE["flux-core<br/>ShellEvent · FluxConfig · FluxError"]
    STORAGE["flux-storage<br/>WAL · CommandStore · AliasStore"]
    DAEMON["flux-daemon<br/>Tokio UDS server · async ingestor"]
    INDEXER["flux-indexer<br/>Radix Trie · BM25 · Fuzzy · AliasSuggester"]
    MINER["flux-miner<br/>Session clustering · Workflow DAG · Stats"]
    QUERY["flux-query<br/>Lexer → AST → Parser → Executor"]
    TUI["flux-tui<br/>Ratatui TUI · Clap CLI · Shell init scripts"]

    SHELL -->|"fire-and-forget (non-blocking)"| CORE
    CORE --> STORAGE
    CORE --> DAEMON
    STORAGE --> INDEXER
    STORAGE --> MINER
    INDEXER --> QUERY
    INDEXER --> TUI
    QUERY --> TUI
    MINER --> TUI
```

---

## Installation

```bash
git clone https://github.com/you/flux
cd flux
chmod +x install.sh && ./install.sh
```

### Shell Integration

**Bash** — add to `~/.bashrc`:

```bash
eval "$(flux init bash)"
```

**Zsh** — add to `~/.zshrc`:

```bash
eval "$(flux init zsh)"
```

**Fish** — add to `~/.config/fish/config.fish`:

```fish
flux init fish | source
```

---

## Shell Integration Flow

How Flux hooks into your shell without slowing it down:

```mermaid
sequenceDiagram
    participant U as User
    participant SH as Shell (Bash/Zsh/Fish)
    participant HOOK as preexec Hook
    participant FLUX as flux binary
    participant WAL as WAL (events.wal)
    participant STORE as CommandStore

    U->>SH: types and runs a command
    SH->>HOOK: fires preexec / PROMPT_COMMAND
    HOOK->>FLUX: flux custom "<command>" &
    Note over HOOK,FLUX: Non-blocking background process
    FLUX->>FLUX: build ShellEvent (cmd, ts, cwd, branch)
    FLUX->>WAL: append JSON line
    FLUX->>STORE: ingest → update frecency score
    FLUX->>STORE: save snapshot if dirty
    Note over SH: Shell prompt returns immediately
```

---

## Data Pipeline

End-to-end flow from keypress to ranked suggestion:

```mermaid
flowchart LR
    A([Raw command string]) --> B[Trim & validate]
    B --> C{Skip?\nlen≤5 AND single token}
    C -->|yes| Z([Drop])
    C -->|no| D[Build ShellEvent\nts · cwd · git_branch · session_id]
    D --> E[WAL append]
    D --> F{Command\nalready in index?}
    F -->|yes| G[touch: freq++ · update ts/cwd/branch]
    F -->|no| H[Record::new: char_len · token_count]
    G --> I[Recalculate frecency score]
    H --> I
    I --> J{total_score\n> 50,000?}
    J -->|yes| K[Decay: freq × 0.5\nevict freq < 1]
    J -->|no| L[Insert into BTreeSet + HashMap]
    K --> L
    L --> M([Sorted record corpus])
```

---

## Storage Layer

### WAL Lifecycle

```mermaid
flowchart TD
    A([Shell hook fires]) --> B[WAL::open\ncount existing lines]
    B --> C[Wal::append\nJSON line + flush]
    C --> D{count > max_wal_events?}
    D -->|no| E([Continue])
    D -->|yes| F[Wal::compact\ndrain oldest lines to max]
    F --> G[Write to .tmp file]
    G --> H[Atomic rename → events.wal]
    H --> I[Re-open writer in append mode]
    I --> E

    style F fill:#f59e0b,color:#000
    style H fill:#10b981,color:#fff
```

### Store Snapshot & Recovery

```mermaid
flowchart LR
    START([Process starts]) --> A[Store::load\nread command_store.json]
    A --> B{snapshot\nexists?}
    B -->|yes| C[Deserialize Snapshot\nrecords + deleted set]
    B -->|no| D[Empty Store]
    C --> E{index\nempty?}
    D --> E
    E -->|yes| F[WAL::replay\nreplay all events in order]
    F --> G[store.ingest each event]
    E -->|no| H([Store ready])
    G --> H

    H --> I[Normal operation…]
    I --> J{every N=500\nevents}
    J --> K[Store::save\nserde_json snapshot]
    K --> L{WAL needs\ncompaction?}
    L -->|yes| M[Wal::compact]
    L -->|no| I
    M --> I
```

---

## Search Engine

### Multi-Strategy Search Pipeline

```mermaid
flowchart TD
    Q([User query]) --> EMPTY{query\nempty?}
    EMPTY -->|yes| SORTED[Return all records\nsorted by frecency]
    EMPTY -->|no| TRIE[Radix Trie prefix scan\nO k complexity]

    TRIE --> TRIE_IDS[(prefix_ids: HashSet)]
    Q --> BM25[BM25 scorer\nIDF × normalized TF]
    BM25 --> BM25_MAP[(bm25_map: HashMap score)]

    TRIE_IDS --> MERGE[Merge candidates]
    BM25_MAP --> MERGE

    MERGE --> FUZZY{not in prefix\nAND bm25=0?}
    FUZZY -->|yes| LEV[Levenshtein\nedit-distance ≤ len/4]
    FUZZY -->|no| SCORE

    LEV --> SCORE[Composite score\nfrecency + bm25×2 + prefix_bonus+5]
    SCORE --> SORT[Sort descending]
    SORT --> LIMIT[Truncate to limit]
    LIMIT --> OUT([SearchResult vec])
```

### Radix Trie Node Split

```mermaid
flowchart TD
    INSERT([insert key, id]) --> FIRST{"child for key[0] exists?"}
    FIRST -->|no| LEAF[Create leaf node\nattach to parent]
    FIRST -->|yes| CP[Compute common prefix length c]
    CP --> FULL{"c == edge label length?"}
    FULL -->|yes| RECURSE["Recurse into child\nwith key[c..]"]
    FULL -->|no| SPLIT["Split edge at c\ncreate intermediate node"]
    SPLIT --> OLD["Re-attach old child\nwith old_label[c..]"]
    SPLIT --> NEW["Attach new leaf\nwith key[c..]"]
```

---

## Query Language

### Token → AST → Execution Pipeline

```mermaid
flowchart LR
    RAW([Raw SQL string]) --> LEX[Lexer\nbyte-by-byte scan]
    LEX --> TOKENS[(Token stream\nSELECT · WHERE · AND · OR · Ident · Num…)]
    TOKENS --> PARSE[Recursive descent parser\nP::query]
    PARSE --> SOURCE{COMMANDS\nor WORKFLOWS?}
    SOURCE --> FILTER[Parse WHERE clause\ninto Filter AST]
    FILTER --> COND[Cond: field op value]
    FILTER --> AND_NODE[And: Box Filter × 2]
    FILTER --> OR_NODE[Or: Box Filter × 2]
    FILTER --> NOT_NODE[Not: Box Filter]
    COND --> QUERY_AST[(Query AST\nsource · filter · order_by · limit)]
    AND_NODE --> QUERY_AST
    OR_NODE --> QUERY_AST
    NOT_NODE --> QUERY_AST
    QUERY_AST --> EXEC[Executor\neval each Record]
    EXEC --> EVCOND{eval_cond\nfield match}
    EVCOND -->|frequency/score/length| CMP[Numeric compare]
    EVCOND -->|command/cmd| STR[String contains/eq/neq]
    CMP --> ROWS[(Filtered rows)]
    STR --> ROWS
    ROWS --> ORDER[ORDER BY\nfrequency · length · score]
    ORDER --> LIM[LIMIT n]
    LIM --> OUT([Vec of Row])
```

### Supported Query Syntax

```sql
-- Most frequent commands
SELECT * COMMANDS WHERE frequency > 10 ORDER BY frequency LIMIT 20

-- Long commands worth aliasing
SELECT * COMMANDS WHERE length > 30 AND frequency > 3

-- Workflows run many times
SELECT * WORKFLOWS WHERE frequency > 5

-- Commands containing a substring
SELECT * COMMANDS WHERE command = "git" LIMIT 10
```

---

## Workflow Mining

### Session Clustering & N-Gram Subsumption

```mermaid
flowchart TD
    EVENTS([Raw ShellEvent stream]) --> SESS[sessionize\ngap_secs threshold]
    SESS --> S1[Session 1\nevent list]
    SESS --> S2[Session 2\nevent list]
    SESS --> SN[Session N…]

    S1 --> WIN[N-Gram Extraction\nLengths 2 to 4]
    S2 --> WIN
    SN --> WIN

    WIN --> COUNT[Frequency count]
    COUNT --> SUBSUME[Subsumption Engine\nPrune shorter sub-sequences]
    SUBSUME --> OUT([Filtered Workflows])

    style SUBSUME fill:#6366f1,color:#fff
```

### Markov Chain Transition Model

```mermaid
stateDiagram-v2
    [*] --> A: run "git status"
    A --> B: run "git add ."
    B --> C: run "git commit -m"
    C --> D: run "git push"
    D --> [*]

    A: git status
    B: git add .
    C: git commit -m "…"
    D: git push

    note right of A: P(add | status) = 0.72
    note right of B: P(commit | add) = 0.89
    note right of C: P(push | commit) = 0.91
```

---

## TUI State Machine

```mermaid
stateDiagram-v2
    [*] --> Main: flux (no args)

    Main --> Search: s or /
    Search --> Main: Esc

    Main --> AddAlias: a
    AddAlias --> PickSuggestion: Tab (load suggestions)
    PickSuggestion --> AddAlias: Enter (fill alias field)
    AddAlias --> Confirm_Add: Enter (confirm)
    Confirm_Add --> Main: y / Enter
    Confirm_Add --> Main: n / Esc

    Main --> RemoveAlias: r
    RemoveAlias --> Confirm_Remove: Enter
    Confirm_Remove --> Main: y / Enter
    Confirm_Remove --> Main: n / Esc

    Main --> ChangeAlias: c
    ChangeAlias --> Main: Enter / Esc

    Main --> ListAliases: l
    ListAliases --> Main: Esc / q

    Main --> Stats: t
    Stats --> Main: Esc / q

    Main --> Query: open query mode
    Query --> Main: Esc

    Main --> [*]: q
```

---

## Daemon Architecture

```mermaid
flowchart TD
    START([flux-daemon starts]) --> BIND[Bind UnixListener\nto cfg.socket_path]
    BIND --> CHAN[crossbeam bounded channel\ncap = 1024]
    CHAN --> WORKER[Spawn worker thread\nStore + WAL owner]
    CHAN --> ACCEPT[Tokio accept loop]

    ACCEPT --> CONN[New UDS connection]
    CONN --> SPAWN[tokio::spawn handle]
    SPAWN --> READ[AsyncBufRead lines]
    READ --> PARSE{serde_json\nShellEvent?}
    PARSE -->|ok| SEND[tx.try_send]
    PARSE -->|err| WARN[warn! parse error]
    SEND -->|full| DROP[warn! drop event]
    SEND -->|ok| CHAN2[(channel queue)]

    CHAN2 --> WORKER
    WORKER --> WAL_WRITE[wal.append]
    WAL_WRITE --> INGEST[store.ingest]
    INGEST --> COUNT{n % 500 == 0?}
    COUNT -->|yes| SNAP[store.save snapshot]
    SNAP --> COMPACT{WAL needs\ncompaction?}
    COMPACT -->|yes| WAL_COMPACT[wal.compact]
    COUNT -->|no| ACCEPT

    style WORKER fill:#0f172a,color:#fff
    style CHAN2 fill:#1e40af,color:#fff
```

---

## Alias Suggestion Engine

### Suggestion Strategy Pipeline

```mermaid
flowchart TD
    CMD([Input command]) --> S1[Semantic match\ngit/docker/cargo/npm/kubectl]
    CMD --> S2[Abbreviation\nfirst char of each token]
    CMD --> S3[Vowel removal\nstrip aeiou from tokens]
    CMD --> S4[Combined\nfirst_char + second_token]
    CMD --> S5[Truncated\ntool name 2..5 chars]

    S1 --> MERGE[Merge all suggestions]
    S2 --> MERGE
    S3 --> MERGE
    S4 --> MERGE
    S5 --> MERGE

    MERGE --> FILTER{ok?\nlen 2–8\nnot existing alias\nnot system command}
    FILTER -->|pass| PRIORITY[Priority sort\nSemantic=100 · Abbrev=90\nVowel=80 · Combined=70]
    FILTER -->|fail| DROP([Drop])
    PRIORITY --> DEDUP[Deduplicate by alias name]
    DEDUP --> OUT([Ranked Suggestion vec])
```

### Semantic Alias Map (Built-in)

```mermaid
mindmap
  root((flux suggest))
    git
      status → gs
      push → gp
      pull → gl
      log → glg
      branch → gb
      add → ga
      commit → gc
      checkout → gco
      diff → gd
      stash → gst
    docker
      ps → dps
      run → dr
      build → db
      exec → de
      compose → dc
    cargo
      build → cb
      test → ct
      run → cr
      check → cc
      clippy → cl
    npm/yarn/pnpm
      install → ni
      run → nr
      test → nt
    kubectl
      get → kg
      apply → ka
```

---

## Scoring Algorithm

### Frecency Score Calculation

```mermaid
flowchart TD
    INPUTS([frequency · last_access · char_len]) --> AGE[Age delta = now - last_access]
    AGE --> MULT{Time bucket}
    MULT -->|≤ 1 hour| M4["multiplier = 4.0 🔥"]
    MULT -->|≤ 1 day| M2["multiplier = 2.0"]
    MULT -->|≤ 1 week| M05["multiplier = 0.5"]
    MULT -->|older| M025["multiplier = 0.25"]

    M4 --> FORMULA["score = mult × char_len^0.6 × frequency"]
    M2 --> FORMULA
    M05 --> FORMULA
    M025 --> FORMULA

    FORMULA --> BOOST{Context\nboost?}
    BOOST -->|cwd matches| CWD["+30 points"]
    BOOST -->|git branch matches| BRANCH["+20 points"]
    BOOST -->|time of day match| TIME["Up to +25 points"]
    BOOST -->|neither| PLAIN["+0"]

    CWD --> FINAL([Final score])
    BRANCH --> FINAL
    TIME --> FINAL
    PLAIN --> FINAL
```

---

## CLI Reference

```
flux                                   Launch interactive TUI
flux suggest -n 10                     Top 10 alias suggestions
flux search "git commit"               Fuzzy search command history
flux search "docker" -l 5              Search with result limit
flux predict "git add ."               Predict the next command sequence
flux context                           Get context-aware commands for your current directory
flux stats                             Workflow analytics & keystroke savings
flux query "SELECT * COMMANDS WHERE frequency > 5 LIMIT 10"
flux add gs -c "git status"            Add an alias
flux remove gs                         Remove an alias
flux change gs gst "git status --short"
flux list                              List all aliases
flux suppress "some long command"      Remove from suggestions
flux init bash | zsh | fish            Print shell init script
```

---

## TUI Keybindings

| Key       | Mode      | Action                         |
| --------- | --------- | ------------------------------ |
| `s` / `/` | Main      | Open fuzzy search              |
| `a`       | Main      | Add alias for selected command |
| `r`       | Main      | Remove alias                   |
| `c`       | Main      | Change alias                   |
| `l`       | Main      | List all aliases               |
| `t`       | Main      | Workflow stats                 |
| `↑` / `k` | Main      | Navigate up                    |
| `↓` / `j` | Main      | Navigate down                  |
| `p`       | Main      | Show workflow predictions      |
| `x`       | Main      | Filter by local context        |
| `Tab`     | Add Alias | Pick from alias suggestions    |
| `F5`      | Main      | Refresh command list           |
| `Esc`     | Any       | Back / cancel                  |
| `q`       | Main      | Quit                           |

---

## Configuration

Config lives at `~/.flux/config.json`. All fields are optional — Flux uses sensible defaults.

```json
{
  "data_dir": "~/.flux",
  "socket_path": "~/.flux/flux.sock",
  "alias_file_paths": ["~/.flux/aliases"],
  "max_wal_events": 50000,
  "bm25_k1": 1.5,
  "bm25_b": 0.75
}
```

| Field              | Default               | Description                                       |
| ------------------ | --------------------- | ------------------------------------------------- |
| `data_dir`         | `~/.flux`             | Where all flux data lives                         |
| `socket_path`      | `~/.flux/flux.sock`   | Unix domain socket for the daemon                 |
| `alias_file_paths` | `["~/.flux/aliases"]` | Alias files to read/write (sourced by shell hook) |
| `max_wal_events`   | `50,000`              | WAL line limit before compaction triggers         |
| `bm25_k1`          | `1.5`                 | BM25 term saturation parameter                    |
| `bm25_b`           | `0.75`                | BM25 length normalization parameter               |

---

## Performance Targets

| Metric                         | Target                        | Implementation                              |
| ------------------------------ | ----------------------------- | ------------------------------------------- |
| Shell hook latency             | **< 2ms**                     | Fire-and-forget background process (`&`)    |
| Search latency (100k commands) | **< 5ms**                     | Radix Trie O(k) prefix + BM25               |
| Daemon idle RAM                | **< 15MB**                    | Bounded crossbeam channel, no heap bloat    |
| WAL compaction                 | triggered at `max_wal_events` | Atomic rename, no data loss                 |
| Score decay                    | automatic                     | Halve frequencies when total_score > 50,000 |

---

## Development

```bash
# Build all crates
cargo build

# Release binary
cargo build --release

# Run the TUI
cargo run

# Run the daemon (optional — CLI works standalone)
cargo run --bin flux-daemon

# Run tests
cargo test
```

### Crate Dependency Graph

```mermaid
graph BT
    TUI["flux-tui\n(binary: flux)"]
    DAEMON["flux-daemon\n(binary: flux-daemon)"]
    QUERY["query.rs"]
    SEARCH["search.rs"]
    MINER["miner.rs"]
    STORE["store.rs"]
    CONFIG["config.rs"]
    SHELL["shell.rs"]
    CLI["cli.rs"]

    TUI --> CLI
    TUI --> SHELL
    TUI --> SEARCH
    TUI --> QUERY
    TUI --> MINER
    TUI --> STORE
    TUI --> CONFIG

    DAEMON --> STORE
    DAEMON --> CONFIG

    SEARCH --> STORE
    SEARCH --> CONFIG
    MINER --> STORE
    QUERY --> STORE
```

---

## File Layout

```
flux/
├── src/
│   ├── main.rs          # CLI entry point + subcommand dispatch
│   ├── cli.rs           # Clap CLI definitions
│   ├── config.rs        # Config load/save (~/.flux/config.json)
│   ├── store.rs         # CommandStore · WAL · AliasStore · frecency
│   ├── search.rs        # Radix Trie · BM25 · Levenshtein · AliasSuggester
│   ├── miner.rs         # Session clustering · WorkflowDag · MarkovChain · Stats
│   ├── query.rs         # Lexer · AST · recursive descent parser · executor
│   ├── shell.rs         # Shell init script generator (bash/zsh/fish/posix)
│   ├── daemon.rs        # Tokio UDS server (binary: flux-daemon)
│   └── tui/
│       ├── mod.rs       # TUI entry point
│       ├── app.rs       # App state machine
│       ├── events.rs    # Crossterm event handling
│       └── ui.rs        # Rendering logic for all views
├── vendor/
│   └── unicode-segmentation/   # vendored dependency
├── Cargo.toml
├── Cargo.lock
└── install.sh
```

---

<div align="center">

Built with ⚡ in Rust · Local-first · Zero telemetry · Sub-millisecond

</div>
