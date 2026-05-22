// ─── shell.rs ─────────────────────────────────────────────────────────────────
//
// Generates shell-specific init snippets.
// The snippet wires preexec/precmd hooks to call `almanx record` after each
// command, passing full telemetry (CWD, exit code, duration).

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
        Self { bin, alias_file: alias_file.to_owned() }
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
# Add to ~/.bashrc:   eval "$(almanx init bash)"

_almanx_preexec() {{
    _ALMANX_CMD="$BASH_COMMAND"
    _ALMANX_START=$(date +%s%3N 2>/dev/null || echo 0)
}}

_almanx_precmd() {{
    local _exit=$?
    if [ -n "$_ALMANX_CMD" ] && [ -n "$_ALMANX_START" ]; then
        local _end _dur
        _end=$(date +%s%3N 2>/dev/null || echo 0)
        _dur=$(( _end - _ALMANX_START ))
        "{bin}" record "$_ALMANX_CMD" \
            --cwd "$PWD" \
            --exit-code "$_exit" \
            --duration "$_dur" 2>/dev/null &
    fi
    _ALMANX_CMD=""
    _ALMANX_START=""
}}

[[ -n "$PS1" ]] && trap '_almanx_preexec' DEBUG
PROMPT_COMMAND="_almanx_precmd;${{PROMPT_COMMAND:-}}"

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
# Add to ~/.zshrc:   eval "$(almanx init zsh)"

autoload -U add-zsh-hook
zmodload zsh/datetime 2>/dev/null || true

_almanx_preexec() {{
    _ALMANX_CMD=$1
    _ALMANX_START=$EPOCHREALTIME
}}

_almanx_precmd() {{
    local _exit=$?
    if [[ -n $_ALMANX_CMD && -n $_ALMANX_START ]]; then
        local _dur=$(( int(($EPOCHREALTIME - $_ALMANX_START) * 1000) ))
        "{bin}" record "$_ALMANX_CMD" \
            --cwd "$PWD" \
            --exit-code "$_exit" \
            --duration "$_dur" 2>/dev/null &
    fi
    _ALMANX_CMD=""
    _ALMANX_START=""
}}

add-zsh-hook preexec _almanx_preexec
add-zsh-hook precmd _almanx_precmd

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
# Add to ~/.config/fish/config.fish:   almanx init fish | source

function _almanx_preexec --on-event fish_preexec
    set -g _almanx_cmd $argv[1]
    set -g _almanx_start (date +%s%3N 2>/dev/null; or echo 0)
end

function _almanx_postexec --on-event fish_postexec
    set -l _exit $status
    if test -n "$_almanx_cmd"
        set -l _end (date +%s%3N 2>/dev/null; or echo 0)
        set -l _dur (math "$_end - $_almanx_start" 2>/dev/null; or echo 0)
        "{bin}" record "$_almanx_cmd" \
            --cwd "$PWD" \
            --exit-code "$_exit" \
            --duration "$_dur" 2>/dev/null &
    end
    set -e _almanx_cmd
    set -e _almanx_start
end

test -f "{alias_file}"; and source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}
