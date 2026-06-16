//! `waeasictl completion <bash|zsh|fish|powershell>` — emit shell-completion script.
use crate::error::{CliError, CliResult};

const SUBCOMMANDS: &[&str] = &[
    "list", "ps", "run", "logs", "tail", "events",
    "metrics", "debug", "top", "inspect", "trace",
    "cap", "kill", "restart", "health", "version", "dmesg",
    "wasm", "manifest", "completion",
    "config", "doctor", "bench", "exec", "port-forward", "profile",
];

pub fn run(args: &[String]) -> CliResult {
    let sh = args.first().map(|s| s.as_str())
        .ok_or_else(|| CliError::Usage("completion <bash|zsh|fish|powershell>".into()))?;
    match sh {
        "bash"       => { print!("{}", bash());       Ok(()) }
        "zsh"        => { print!("{}", zsh());        Ok(()) }
        "fish"       => { print!("{}", fish());       Ok(()) }
        "powershell" => { print!("{}", powershell()); Ok(()) }
        u => Err(CliError::Usage(format!("unsupported shell: {}", u))),
    }
}

fn bash() -> String {
    let cmds = SUBCOMMANDS.join(" ");
    format!(r#"_waeasictl()
{{
    local cur cmds
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    cmds="{cmds}"
    COMPREPLY=( $(compgen -W "${{cmds}}" -- ${{cur}}) )
}}
complete -F _waeasictl waeasictl
"#, cmds = cmds)
}

fn zsh() -> String {
    let cmds = SUBCOMMANDS.join(" ");
    format!(r#"#compdef waeasictl
_arguments '*: :({cmds})'
"#, cmds = cmds)
}

fn fish() -> String {
    let mut out = String::new();
    for c in SUBCOMMANDS {
        out.push_str(&format!(
            "complete -c waeasictl -f -n '__fish_use_subcommand' -a '{}'\n", c));
    }
    out
}

fn powershell() -> String {
    let cmds = SUBCOMMANDS.iter()
        .map(|c| format!("'{}'", c)).collect::<Vec<_>>().join(", ");
    format!(r#"Register-ArgumentCompleter -CommandName waeasictl -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    @({cmds}) | Where-Object {{ $_ -like "$wordToComplete*" }}
}}
"#, cmds = cmds)
}
