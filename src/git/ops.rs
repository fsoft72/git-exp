use crate::git::cli::{GitCommandLog, GitOutput, run_git};
use crate::git::status::FileStatusKind;
use std::fs;
use std::path::Path;

fn ensure_ok(out: GitOutput) -> Result<GitCommandLog, (String, GitCommandLog)> {
    if out.is_success() {
        Ok(out.log)
    } else {
        let msg = if out.stderr.trim().is_empty() {
            out.stdout.trim().to_string()
        } else {
            out.stderr.trim().to_string()
        };
        Err((msg, out.log))
    }
}

pub fn stage_paths(repo: &Path, paths: &[String]) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["add", "-A", "--"];
    for p in paths {
        args.push(p.as_str());
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn stage_all(repo: &Path) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["add", "-A"], None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn unstage_paths(
    repo: &Path,
    paths: &[String],
    has_commits: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = if has_commits {
        vec!["restore", "--staged", "--"]
    } else {
        vec!["rm", "--cached", "-f", "--"]
    };
    for p in paths {
        args.push(p.as_str());
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn unstage_all(repo: &Path, has_commits: bool) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let args: &[&str] = if has_commits {
        &["reset", "-q", "HEAD"]
    } else {
        &["rm", "--cached", "-r", "-f", "."]
    };
    let out = run_git(repo, args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn discard_file(
    repo: &Path,
    path: &str,
    kind: FileStatusKind,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    if kind == FileStatusKind::Untracked {
        let full = repo.join(path);
        if full.is_dir() {
            fs::remove_dir_all(&full)
                .map_err(|e| (format!("Errore rimozione {}: {}", path, e), dummy_log(path)))?;
        } else {
            fs::remove_file(&full)
                .map_err(|e| (format!("Errore rimozione {}: {}", path, e), dummy_log(path)))?;
        }
        Ok(GitCommandLog {
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            command_line: format!("rm {}", path),
            exit_code: 0,
            duration_ms: 1,
            stdout_preview: format!("Eliminato file non tracciato: {}", path),
            stderr: String::new(),
        })
    } else {
        let out = run_git(repo, &["checkout", "--", path], None, true)
            .map_err(|e| (e.clone(), dummy_log(&e)))?;
        ensure_ok(out)
    }
}

pub fn discard_all(repo: &Path) -> Result<Vec<GitCommandLog>, (String, GitCommandLog)> {
    let mut logs = Vec::new();
    let out1 = run_git(repo, &["checkout", "--", "."], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    logs.push(ensure_ok(out1)?);

    let out2 = run_git(repo, &["clean", "-fd"], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    logs.push(ensure_ok(out2)?);
    Ok(logs)
}

/// Applica una patch parziale (Hunk o righe selezionate) tramite `git apply`.
/// - `cached == true && reverse == false`: Stage Hunk / Selected Lines
/// - `cached == true && reverse == true`: Unstage Hunk / Selected Lines
/// - `cached == false && reverse == true`: Revert Hunk / Selected Lines nel working tree
pub fn apply_patch(
    repo: &Path,
    patch: &str,
    cached: bool,
    reverse: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["apply", "--unidiff-zero", "--whitespace=nowarn", "-C0"];
    if cached {
        args.push("--cached");
    }
    if reverse {
        args.push("--reverse");
    }
    args.push("-");

    let out = run_git(repo, &args, Some(patch), true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitVariant {
    Normal,
    Amend,
    SignOff,
    CommitAndPush,
    CommitAndSync,
}

pub fn commit(
    repo: &Path,
    message: &str,
    amend: bool,
    signoff: bool,
    allow_empty: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["commit", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    if signoff {
        args.push("--signoff");
    }
    if allow_empty {
        args.push("--allow-empty");
    }

    let out =
        run_git(repo, &args, Some(message), true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn fetch_all(repo: &Path) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["fetch", "--all", "--prune"], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn pull(repo: &Path, rebase: bool) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let args: &[&str] = if rebase {
        &["pull", "--rebase"]
    } else {
        &["pull"]
    };
    let out = run_git(repo, args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn push(
    repo: &Path,
    set_upstream_branch: Option<&str>,
    force_with_lease: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["push"];
    if force_with_lease {
        args.push("--force-with-lease");
    }
    if let Some(branch) = set_upstream_branch {
        args.push("--set-upstream");
        args.push("origin");
        args.push(branch);
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn sync_repo(
    repo: &Path,
    has_upstream: bool,
    current_branch: &str,
) -> Result<Vec<GitCommandLog>, (String, GitCommandLog)> {
    let mut logs = Vec::new();
    if has_upstream {
        logs.push(pull(repo, false)?);
        logs.push(push(repo, None, false)?);
    } else {
        logs.push(push(repo, Some(current_branch), false)?);
    }
    Ok(logs)
}

pub fn checkout_branch(
    repo: &Path,
    target: &str,
    is_remote: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    if is_remote {
        // Se è `origin/feature`, usa `git checkout --track origin/feature`
        let out = run_git(repo, &["checkout", "--track", target], None, true)
            .map_err(|e| (e.clone(), dummy_log(&e)))?;
        ensure_ok(out)
    } else {
        let out = run_git(repo, &["checkout", target], None, true)
            .map_err(|e| (e.clone(), dummy_log(&e)))?;
        ensure_ok(out)
    }
}

pub fn create_branch(
    repo: &Path,
    name: &str,
    from_ref: Option<&str>,
    checkout: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = if checkout {
        vec!["checkout", "-b", name]
    } else {
        vec!["branch", name]
    };
    if let Some(base) = from_ref {
        args.push(base);
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn delete_branch(
    repo: &Path,
    name: &str,
    force: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let flag = if force { "-D" } else { "-d" };
    let out = run_git(repo, &["branch", flag, name], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn rename_branch(repo: &Path, new_name: &str) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["branch", "-m", new_name], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn merge_branch(repo: &Path, branch: &str) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["merge", branch], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn rebase_branch(repo: &Path, onto: &str) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["rebase", onto], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn stash_push(
    repo: &Path,
    message: Option<&str>,
    include_untracked: bool,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["stash", "push"];
    if include_untracked {
        args.push("-u");
    }
    if let Some(m) = message {
        if !m.trim().is_empty() {
            args.push("-m");
            args.push(m.trim());
        }
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn stash_action(
    repo: &Path,
    action: &str, // "pop", "apply", "drop"
    stash_ref: &str,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["stash", action, stash_ref], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn create_tag(
    repo: &Path,
    tag_name: &str,
    message: Option<&str>,
) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let mut args = vec!["tag"];
    if let Some(m) = message {
        if !m.trim().is_empty() {
            args.push("-a");
            args.push(tag_name);
            args.push("-m");
            args.push(m.trim());
        } else {
            args.push(tag_name);
        }
    } else {
        args.push(tag_name);
    }
    let out = run_git(repo, &args, None, true).map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn delete_tag(repo: &Path, tag_name: &str) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["tag", "-d", tag_name], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

pub fn undo_last_commit(repo: &Path) -> Result<GitCommandLog, (String, GitCommandLog)> {
    let out = run_git(repo, &["reset", "--soft", "HEAD~1"], None, true)
        .map_err(|e| (e.clone(), dummy_log(&e)))?;
    ensure_ok(out)
}

fn dummy_log(err: &str) -> GitCommandLog {
    GitCommandLog {
        timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
        command_line: "git".to_string(),
        exit_code: -1,
        duration_ms: 0,
        stdout_preview: String::new(),
        stderr: err.to_string(),
    }
}

