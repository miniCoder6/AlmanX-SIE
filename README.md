# 🚀 AlmanX — Shell Intelligence Engine

AlmanX is a **local-first, zero-dependency shell intelligence platform** written entirely in Rust. It silently tracks your terminal activity, mines workflow patterns, provides sub-millisecond fuzzy search, and suggests personalized shell aliases. AlmanX acts as a true companion for the command line, predicting what you need next based on where you are and what you've done before.

---

## 🌟 Features

| Feature | Status | Description |
|---|---|---|
| Shell hooks (bash/zsh/fish) | ✅ | Auto-records every command you run seamlessly in the background |
| Frecency-scored command DB | ✅ | Scores commands intelligently based on frequency and recency |
| Alias suggestion engine | ✅ | Provides semantic, abbreviation, and vowel-strip alias suggestions |
| Fuzzy search (`almanx search`) | ✅ | Jaro-Winkler + substring + recency + contextual scoring |
| Workflow mining (`almanx workflows`)| ✅ | N-gram DAG reconstruction from chronological session history |
| Context Suggestions (`almanx context`) | ✅ | Recommends relevant commands based on CWD and time of day |
| Command Prediction (`almanx predict`) | ✅ | Suggests your next command based on Markov chain/DAG transition probabilities |
| Productivity analytics (`almanx stats`) | ✅ | Keystroke savings, top commands, most active directories |
| Interactive TUI | ✅ | Full ratatui interface for browsing, managing, and alias mapping |
| Event log (JSONL) | ✅ | Append-only telemetry log at `~/.almanx/events.jsonl` with automatic compaction |
| Alias file management | ✅ | Safely add/remove/rename/list aliases and sync with your shell |

---

## 🧠 How AlmanX Analyzes Your Data (Analysis Steps)

AlmanX is more than just a history logger. It acts as an offline AI for your shell by following these rigorous analysis steps:

1. **Telemetry Collection (`crates/collector`)**: Every command is captured with detailed context:
   - Raw command text and arguments
   - Current Working Directory (CWD)
   - Exit code (0 for success, non-zero for failures)
   - Execution duration (ms)
   - Current Git Branch (if applicable)

2. **Fuzzy Indexing & Search (`crates/indexer`)**: 
   - Commands are deduplicated with priority given to the most recent execution.
   - Searching uses **Jaro-Winkler edit distance**, bolstered heavily by *substring* and *prefix* matching.
   - Results are contextually boosted based on command success (`exit_code == 0`) and execution recency.

3. **Workflow Mining / N-Gram Extraction (`crates/workflow-engine`)**:
   - **Sessionization**: Command events are grouped into "sessions" split by 30-minute idle gaps.
   - **N-Gram Counting**: The engine scans for repeated command sequences (length 2 to 4).
   - **Subsumption**: If `git add . -> git commit -> git push` is frequent, the engine will "subsume" (hide) the shorter `git add . -> git commit` to avoid noise.
   - **DAG Reconstruction**: Mined sequences are turned into a Directed Acyclic Graph (DAG) of your personal workflows.

4. **Context & Prediction Engine (`crates/query-engine`)**:
   - **Predict Next**: Given your last command, AlmanX uses transition probabilities from the mined workflow DAG to predict the command you are most likely to run next.
   - **Context Suggestions**: AlmanX looks at your current directory (`CWD`) and the time of day, boosting commands you historically run in that specific environment.

---

## 🛠️ Quick Start

```bash
# 1. Build and install
bash install.sh

# 2. Wire up your shell (add to ~/.bashrc or ~/.zshrc)
eval "$(almanx init bash)"    # bash
eval "$(almanx init zsh)"     # zsh
almanx init fish | source     # fish

# 3. Reload your shell
source ~/.bashrc

# 4. Use your terminal normally for a while, then try:
almanx                        # Open interactive TUI
almanx suggest                # Get alias suggestions
almanx stats                  # View productivity report
almanx search "git"           # Fuzzy search history
almanx workflows              # View workflow patterns
almanx context                # See commands relevant to your current folder
almanx predict "git status"   # Predict your next command
```

---

## 💻 CLI Reference

```
almanx                          Launch interactive TUI
almanx search <query>           Fuzzy search command history
almanx stats                    Show productivity analytics
almanx workflows [--min-freq N] Show mined workflow DAGs
almanx suggest [--num N]        Suggest shell aliases
almanx predict <last_cmd>       Predict the most likely next commands
almanx context [cwd]            Contextual command suggestions based on environment
almanx add <alias> <command>    Add an alias
almanx remove <alias>           Remove an alias
almanx rename <old> <new>       Rename an alias
almanx list                     List all tracked aliases
almanx dismiss <command>        Stop tracking a command
almanx init bash|zsh|fish       Print shell integration snippet
```

---

## 🏗️ Architecture Overview

```text
~/.almanx/
├── database.json    # Frecency-scored command index
├── deleted.json     # Tombstoned (dismissed) commands
├── events.jsonl     # Append-only event log (full telemetry)
├── config.json      # User configuration
└── aliases          # Generated shell alias file
```

**Crate Structure**:
- `crates/collector`: Shell integration and `CommandEvent` struct generation.
- `crates/storage`: Fast, append-only JSONL event logging with compaction.
- `crates/indexer`: High-speed string similarity and query ranking.
- `crates/workflow-engine`: Session analysis and n-gram mining.
- `crates/query-engine`: Context-aware inference and predictive suggestions.
- `src/tui/`: `ratatui`-based interactive terminal UI.
- `src/cli/`: `clap`-based command-line interface.

---

## 🧪 Testing Strategy

AlmanX requires robust testing across its intelligence components and CLI infrastructure.

### 1. Unit Tests (`cargo test`)
Test the core logic of individual crates:
- **Indexer**: Verify Jaro-Winkler scoring logic and prefix-boosting.
- **Workflow Engine**: Feed a mock slice of `CommandEvent`s and verify that strict sub-sequences are correctly removed (subsumed).
- **Query Engine**: Verify that context suggestions correctly rank commands executed in matching CWDs higher than global commands.
- **Storage**: Test read/append performance and log compaction boundaries.

```bash
cargo test --workspace
```

### 2. Integration Tests (`bash integration_tests/test_hooks.sh`)
Test the entire executable and environment integration:
- Simulates shell hooks recording commands.
- Verifies that `~/.almanx/database.json` and `~/.almanx/events.jsonl` are populated correctly.
- Verifies output format of `almanx stats` and `almanx search`.

```bash
bash integration_tests/test_hooks.sh
```

### 3. Manual UI/UX Verification
- Run `almanx` to launch the Ratatui interface. 
- Use `/` to test the real-time responsiveness of the fuzzy search.
- Press `a` to verify the alias addition flow successfully writes to the generated `~/.almanx/aliases` file.
