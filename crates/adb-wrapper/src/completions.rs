use anyhow::{Result, bail};
use std::io::{self, Write};

const SHELLS: &[&str] = &["bash", "zsh", "fish", "powershell"];

pub fn run_completions(args: &[String]) -> Result<u8> {
    let shell = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("usage: adb completions <bash|zsh|fish|powershell>"))?;

    if !SHELLS.contains(&shell) {
        bail!(
            "adb-wrapper: unknown shell '{shell}', expected one of: {}",
            SHELLS.join(", ")
        );
    }

    let script = match shell {
        "bash" => bash_script(),
        "zsh" => zsh_script(),
        "fish" => fish_script(),
        "powershell" => powershell_script(),
        _ => unreachable!(),
    };

    io::stdout().write_all(script.as_bytes())?;
    Ok(0)
}

fn common_commands() -> &'static str {
    "devices version help start-server kill-server reconnect shell push pull install uninstall logcat forward reverse bugreport jdwp get-state get-serialno wait-for-device start stop reboot emu"
}

fn bash_script() -> String {
    format!(
        r#"_adb_wrapper_list_devices() {{
    adb devices -l 2>/dev/null | awk 'NR>1 && $2=="device" {{print $1}}'
}}

_adb_wrapper_list_serials() {{
    compgen -W "$(_adb_wrapper_list_devices)" -- "$1"
}}

_adb() {{
    local cur prev
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    if [[ "$cur" == "-s" || "$prev" == "-s" ]]; then
        COMPREPLY=($(compgen -W "$(_adb_wrapper_list_devices)" -- "$cur"))
        return
    fi

    if [[ ${{COMP_CWORD}} -eq 1 ]]; then
        COMPREPLY=($(compgen -W "{cmds} completions --pick" -- "$cur"))
        return
    fi

    COMPREPLY=($(compgen -W "{cmds}" -- "$cur"))
}}

complete -F _adb adb
"#,
        cmds = common_commands()
    )
}

fn zsh_script() -> String {
    let cmds: Vec<&str> = common_commands().split_whitespace().collect();
    let cmd_list = cmds.join("\n      ");
    format!(
        r#"_adb_wrapper_devices() {{
  adb devices -l 2>/dev/null | awk 'NR>1 && $2=="device" {{print $1}}'
}}

_adb() {{
  local context state line
  _arguments -C \
    '--pick[Force device picker]' \
    '-s[Device serial]:serial:_adb_wrapper_devices' \
    '*:command:->cmds' && return 0

  case $state in
    serial)
      compadd $(_adb_wrapper_devices)
      ;;
    cmds)
      compadd {cmd_list} completions
      ;;
  esac
}}

compdef _adb adb
"#
    )
}

fn fish_script() -> String {
    let cmds = common_commands();
    format!(
        r#"function __adb_wrapper_devices
    adb devices -l 2>/dev/null | awk 'NR>1 && $2=="device" {{print $1}}'
end

complete -c adb -f -a "{cmds}"
complete -c adb -f -a "completions --pick"
complete -c adb -f -n "__fish_seen_subcommand_from -s" -a "(__adb_wrapper_devices)"
complete -c adb -f -s s -a "(__adb_wrapper_devices)"
"#,
        cmds = cmds
    )
}

fn powershell_script() -> String {
    format!(
        r#"function __adb_wrapper_devices {{
    adb devices -l 2>$null | ForEach-Object {{
        if ($_ -match '^\s*(\S+)\s+device\s') {{ $matches[1] }}
    }}
}}

Register-ArgumentCompleter -Native -CommandName adb -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $commands = @('{cmds}'.Split(' '))
    $commands + @('completions', '--pick') | Where-Object {{ $_ -like "$wordToComplete*" }} | ForEach-Object {{
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }}
    __adb_wrapper_devices | Where-Object {{ $_ -like "$wordToComplete*" }} | ForEach-Object {{
        [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
    }}
}}
"#,
        cmds = common_commands().replace(' ', "','")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_rejects_unknown_shell() {
        let err = run_completions(&["unknown".into()]).unwrap_err();
        assert!(err.to_string().contains("unknown shell"));
    }

    #[test]
    fn bash_completion_script_contains_adb() {
        assert!(bash_script().contains("complete -F _adb adb"));
    }
}
