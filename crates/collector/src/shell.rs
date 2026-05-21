use std::path::PathBuf;

pub struct ShellContext {
    pub bin: String,
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

pub fn bash(ctx: &ShellContext) -> String {
    format!(
        r#"# AlmanX telemetry — bash
_almanx_preexec() {{
    export _ALMANX_START=$(date +%s%3N)
    export _ALMANX_CMD="$BASH_COMMAND"
}}

_almanx_precmd() {{
    local exit_code=$?
    if [ -n "$_ALMANX_START" ] && [ -n "$_ALMANX_CMD" ]; then
        local end=$(date +%s%3N)
        local duration=$((end - _ALMANX_START))
        "{bin}" record \
            --cmd "$_ALMANX_CMD" \
            --cwd "$PWD" \
            --exit-code "$exit_code" \
            --duration "$duration" 2>/dev/null &
    fi
    _ALMANX_START=""
    _ALMANX_CMD=""
}}

trap '_almanx_preexec' DEBUG
PROMPT_COMMAND="_almanx_precmd;$PROMPT_COMMAND"

[ -f "{alias_file}" ] && source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}

pub fn zsh(ctx: &ShellContext) -> String {
    format!(
        r#"# AlmanX telemetry — zsh
zmodload zsh/datetime

_almanx_preexec() {{
    _ALMANX_START=$EPOCHREALTIME
    _ALMANX_CMD=$1
}}

_almanx_precmd() {{
    local exit_code=$?
    if [[ -n $_ALMANX_START && -n $_ALMANX_CMD ]]; then
        local duration=$(( (EPOCHREALTIME - _ALMANX_START) * 1000 ))
        "{bin}" record \
            --cmd "$_ALMANX_CMD" \
            --cwd "$PWD" \
            --exit-code "$exit_code" \
            --duration "${{duration%.*}}" 2>/dev/null &
    fi
    _ALMANX_START=""
    _ALMANX_CMD=""
}}

autoload -U add-zsh-hook
add-zsh-hook preexec _almanx_preexec
add-zsh-hook precmd _almanx_precmd

[ -f "{alias_file}" ] && source "{alias_file}"
"#,
        bin = ctx.bin,
        alias_file = ctx.alias_file,
    )
}
