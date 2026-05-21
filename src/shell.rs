// ─── shell.rs ─────────────────────────────────────────────────────────────────
//
// Generates shell-specific init snippets.
// The snippet wires up a preexec hook that calls `almanx record <cmd>`
// silently after every command, and sources the user's alias file on startup.

use crate::cli::Shell;
use std::path::PathBuf;

pub struct ShellContext {
    /// Absolute path to the `almanx` binary.
    pub bin: String,
    /// Primary alias file path.
    pub alias_file: String,
}

impl ShellContext {
    pub fn load(alias_file: &str) -> Self {
        let bin = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("almanx"))
            .to_string_lossy()
            .to_string();
        Self {
            bin,
            alias_file: alias_file.to_owned(),
        }
    }
}

pub fn init_script(shell: &Shell, ctx: &ShellContext) -> String {
    match shell {
        Shell::Bash => bash(ctx),
        Shell::Zsh  => zsh(ctx),
        Shell::Fish => fish(ctx),
    }
}

// ── Bash ──────────────────────────────────────────────────────────────────────

fn bash(ctx: &ShellContext) -> String {
    format!(
        r#"# AlmanX shell integration — bash
# Add to your ~/.bashrc:
#   eval "$(almanx init bash)"

_almanx_record() {{
    [ -n "$1" ] && "{bin}" record "$1" 2>/dev/null &
}}

# Wire up preexec via the DEBUG trap (interactive shells only).
[[ -n "$PS1" ]] && trap '_almanx_record "$BASH_COMMAND"' DEBUG

# Source alias file.
[ -f "{alias_file}" ] && source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}

// ── Zsh ───────────────────────────────────────────────────────────────────────

fn zsh(ctx: &ShellContext) -> String {
    format!(
        r#"# AlmanX shell integration — zsh
# Add to your ~/.zshrc:
#   eval "$(almanx init zsh)"

_almanx_record() {{
    [ -n "$1" ] && "{bin}" record "$1" 2>/dev/null &
}}

autoload -U add-zsh-hook
add-zsh-hook preexec _almanx_record

# Source alias file.
[ -f "{alias_file}" ] && source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}

// ── Fish ──────────────────────────────────────────────────────────────────────

fn fish(ctx: &ShellContext) -> String {
    format!(
        r#"# AlmanX shell integration — fish
# Add to your ~/.config/fish/config.fish:
#   almanx init fish | source

function _almanx_record --on-event fish_preexec
    test -n "$argv[1]"; and "{bin}" record "$argv[1]" 2>/dev/null &
end

# Source alias file.
test -f "{alias_file}"; and source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}
