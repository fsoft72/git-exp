use chrono::Local;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct GitCommandLog {
    pub timestamp: String,
    pub command_line: String,
    pub exit_code: i32,
    pub duration_ms: u128,
    pub stdout_preview: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub log: GitCommandLog,
}

impl GitOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Esegue un comando `git` nella directory `repo_path`.
/// Imposta `-c core.quotepath=false` per preservare caratteri UTF-8 nei nomi dei file
/// e `LC_ALL=C.UTF-8` (o `LANG=C`) per messaggi coerenti.
pub fn run_git(
    repo_path: &Path,
    args: &[&str],
    stdin_data: Option<&str>,
    optional_locks: bool,
) -> Result<GitOutput, String> {
    let start = Instant::now();
    let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();

    let mut full_args = vec!["-c", "core.quotepath=false", "-c", "color.ui=false"];
    full_args.extend_from_slice(args);

    let cmd_str = format!("git {}", args.join(" "));

    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path)
        .args(&full_args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C.UTF-8");

    if !optional_locks {
        cmd.env("GIT_OPTIONAL_LOCKS", "0");
    }

    if stdin_data.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Impossibile avviare git ({}): {}", cmd_str, e))?;

    if let Some(input) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .map_err(|e| format!("Errore scrittura stdin per {}: {}", cmd_str, e))?;
        }
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Errore attesa git ({}): {}", cmd_str, e))?;

    let duration_ms = start.elapsed().as_millis();
    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let preview = if stdout.len() > 400 {
        format!("{}... ({} bytes)", &stdout[..400], stdout.len())
    } else {
        stdout.clone()
    };

    let log = GitCommandLog {
        timestamp,
        command_line: cmd_str,
        exit_code,
        duration_ms,
        stdout_preview: preview,
        stderr: stderr.clone(),
    };

    Ok(GitOutput {
        stdout,
        stderr,
        exit_code,
        log,
    })
}

