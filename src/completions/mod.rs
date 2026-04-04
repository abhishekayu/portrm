use std::io::Write;

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::Cli;

/// Generate shell completion script with dynamic port completion injected.
pub fn generate_completions(shell: &str, out: &mut dyn Write) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    let mut cmd = Cli::command();

    match shell {
        "bash" => generate(clap_complete::shells::Bash, &mut cmd, "ptrm", &mut buf),
        "zsh" => generate(clap_complete::shells::Zsh, &mut cmd, "ptrm", &mut buf),
        "fish" => generate(clap_complete::shells::Fish, &mut cmd, "ptrm", &mut buf),
        "powershell" => generate(clap_complete::shells::PowerShell, &mut cmd, "ptrm", &mut buf),
        _ => anyhow::bail!("Unsupported shell: {shell}. Use bash, zsh, fish, or powershell."),
    };

    let script = String::from_utf8(buf)?;
    let patched = patch_script(shell, &script);
    out.write_all(patched.as_bytes())?;
    Ok(())
}

/// Patch generated completion scripts to inject dynamic port completion.
fn patch_script(shell: &str, script: &str) -> String {
    match shell {
        "bash" => patch_bash(script),
        "zsh" => patch_zsh(script),
        "fish" => patch_fish(script),
        _ => script.to_string(),
    }
}

fn patch_bash(script: &str) -> String {
    // Prepend a helper function, then append port-aware COMPREPLY logic.
    let helper = r#"
# Dynamic port completion for ptrm
_ptrm_complete_ports() {
    local ports
    ports=$(ptrm _complete-ports 2>/dev/null)
    COMPREPLY+=($(compgen -W "$ports" -- "${cur}"))
}
"#;

    // Insert port completion into the main completion function.
    // We find the closing `\n}` of the _ptrm() function (starts at column 0)
    // and inject our hook just before it.
    let hook = r#"
    # Inject dynamic port completion for PORT arguments
    case "${prev}" in
        scan|kill|fix|info|log|watch|preflight|ptrm)
            _ptrm_complete_ports
            ;;
    esac
"#;

    // Look for a `\n}\n` that marks the end of the _ptrm function body.
    if let Some(func_end) = script.rfind("\n}\n") {
        let insert_at = func_end; // before the newline+}
        let mut patched = String::with_capacity(script.len() + helper.len() + hook.len());
        patched.push_str(helper);
        patched.push_str(&script[..insert_at]);
        patched.push_str(hook);
        patched.push_str(&script[insert_at..]);
        return patched;
    }

    format!("{helper}\n{script}")
}

fn patch_zsh(script: &str) -> String {
    // Add a dynamic port completion function for zsh
    let helper = r#"
# Dynamic port completion for ptrm
_ptrm_complete_ports() {
    local -a ports
    ports=(${(f)"$(ptrm _complete-ports 2>/dev/null)"})
    _describe 'active ports' ports
}
"#;

    // Inject port completion for relevant subcommands by adding
    // compadd for PORT arguments. We add this before the final line.
    let hook = r#"
# Override PORT argument completion with active ports
_ptrm_port_args() {
    local -a active_ports
    active_ports=(${(f)"$(ptrm _complete-ports 2>/dev/null)"})
    compadd -a active_ports
}
"#;

    format!("{helper}\n{hook}\n{script}")
}

fn patch_fish(script: &str) -> String {
    // Fish completions are additive, so we just append dynamic port completions.
    let extra = r#"
# Dynamic port completion for ptrm
complete -c ptrm -n '__fish_seen_subcommand_from scan kill fix info log watch preflight' -xa '(ptrm _complete-ports 2>/dev/null)'
# Also complete ports for bare `ptrm <port>` shorthand
complete -c ptrm -n 'not __fish_seen_subcommand_from scan kill fix info log watch preflight completions interactive group doctor history project up down init registry ci use restart status' -xa '(ptrm _complete-ports 2>/dev/null)'
"#;

    format!("{script}\n{extra}")
}

/// Print active port numbers, one per line (for shell completion consumption).
pub fn list_active_ports() -> anyhow::Result<()> {
    let adapter = crate::platform::adapter();
    let bindings = adapter.list_bindings()?;

    let mut ports: Vec<u16> = bindings.iter().map(|b| b.port).collect();
    ports.sort_unstable();
    ports.dedup();

    for port in ports {
        println!("{port}");
    }
    Ok(())
}
