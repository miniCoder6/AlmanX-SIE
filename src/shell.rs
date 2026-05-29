use crate::cli::Shell;
use crate::config::Config;

pub fn init_script(shell: &Shell, cfg: &Config) -> String {
    let bin = std::env::current_exe().unwrap_or_else(|_| "flux".into()).to_string_lossy().to_string();
    let aliases = cfg.primary_alias_file();
    match shell {
        Shell::Bash => format!(r#"# Flux — bash. Add to ~/.bashrc: eval "$({bin} init bash)"
_flux_record() {{ local c; c="$(history 1 | sed 's/^[[:space:]]*[0-9]*[[:space:]]*//')"; [ -n "$c" ] && "{bin}" custom "$c" 2>/dev/null &; }}
PROMPT_COMMAND="_flux_record;${{PROMPT_COMMAND}}"
[ -f "{aliases}" ] && source "{aliases}"
"#),
        Shell::Zsh => format!(r#"# Flux — zsh. Add to ~/.zshrc: eval "$({bin} init zsh)"
_flux_preexec() {{ [ -n "$1" ] && "{bin}" custom "$1" 2>/dev/null &; }}
autoload -U add-zsh-hook && add-zsh-hook preexec _flux_preexec
[ -f "{aliases}" ] && source "{aliases}"
"#),
        Shell::Fish => format!(r#"# Flux — fish. Add to config.fish: {bin} init fish | source
function _flux_preexec --on-event fish_preexec
    test -n "$argv[1]"; and "{bin}" custom "$argv[1]" 2>/dev/null &
end
test -f "{aliases}"; and source "{aliases}"
"#),
        Shell::Posix => format!(r#"# Flux — POSIX. Add to ~/.profile.
[ -f "{aliases}" ] && . "{aliases}"
"#),
    }
}
