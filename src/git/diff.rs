use crate::git::cli::{GitCommandLog, run_git};
use crate::git::status::{ChangeGroup, FileStatusKind};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_lineno: Option<usize>,
    pub new_lineno: Option<usize>,
    pub content: String,
    pub no_newline_after: bool,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub section_context: String,
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct SideBySideCell {
    pub line_idx: usize,
    pub lineno: usize,
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SideBySideRow {
    pub left: Option<SideBySideCell>,
    pub right: Option<SideBySideCell>,
}

impl DiffHunk {
    /// Costruisce le righe allineate Side-by-Side (stile VS Code Diff Editor) per questo hunk.
    pub fn build_side_by_side_rows(&self) -> Vec<SideBySideRow> {
        let mut rows = Vec::new();
        let mut i = 0;
        let n = self.lines.len();

        while i < n {
            match self.lines[i].kind {
                DiffLineKind::Context => {
                    let line = &self.lines[i];
                    rows.push(SideBySideRow {
                        left: Some(SideBySideCell {
                            line_idx: i,
                            lineno: line.old_lineno.unwrap_or(0),
                            kind: DiffLineKind::Context,
                            content: line.content.clone(),
                        }),
                        right: Some(SideBySideCell {
                            line_idx: i,
                            lineno: line.new_lineno.unwrap_or(0),
                            kind: DiffLineKind::Context,
                            content: line.content.clone(),
                        }),
                    });
                    i += 1;
                }
                DiffLineKind::Removed | DiffLineKind::Added => {
                    // Raccogli blocco contiguo di Removed seguito da Added
                    let mut removed_indices = Vec::new();
                    let mut added_indices = Vec::new();

                    while i < n && self.lines[i].kind == DiffLineKind::Removed {
                        removed_indices.push(i);
                        i += 1;
                    }
                    while i < n && self.lines[i].kind == DiffLineKind::Added {
                        added_indices.push(i);
                        i += 1;
                    }

                    let max_len = removed_indices.len().max(added_indices.len());
                    for k in 0..max_len {
                        let left = removed_indices.get(k).map(|&idx| {
                            let l = &self.lines[idx];
                            SideBySideCell {
                                line_idx: idx,
                                lineno: l.old_lineno.unwrap_or(0),
                                kind: DiffLineKind::Removed,
                                content: l.content.clone(),
                            }
                        });
                        let right = added_indices.get(k).map(|&idx| {
                            let l = &self.lines[idx];
                            SideBySideCell {
                                line_idx: idx,
                                lineno: l.new_lineno.unwrap_or(0),
                                kind: DiffLineKind::Added,
                                content: l.content.clone(),
                            }
                        });
                        rows.push(SideBySideRow { left, right });
                    }
                }
            }
        }

        rows
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffTarget {
    WorkingTree {
        path: String,
        kind: FileStatusKind,
    },
    Staged {
        path: String,
        kind: FileStatusKind,
    },
    Commit {
        commit_sha: String,
        path: String,
    },
}

impl DiffTarget {
    pub fn path(&self) -> &str {
        match self {
            Self::WorkingTree { path, .. }
            | Self::Staged { path, .. }
            | Self::Commit { path, .. } => path,
        }
    }

    pub fn tab_title(&self) -> String {
        let filename = Path::new(self.path())
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(self.path());
        match self {
            Self::WorkingTree { .. } => format!("{} (Working Tree)", filename),
            Self::Staged { .. } => format!("{} (Index)", filename),
            Self::Commit { commit_sha, .. } => {
                let short: String = commit_sha.chars().take(7).collect();
                format!("{} ({})", filename, short)
            }
        }
    }

    pub fn can_stage(&self) -> bool {
        matches!(self, Self::WorkingTree { .. })
    }

    pub fn can_unstage(&self) -> bool {
        matches!(self, Self::Staged { .. })
    }

    pub fn can_revert(&self) -> bool {
        matches!(self, Self::WorkingTree { .. })
    }
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub target: DiffTarget,
    pub old_path: String,
    pub new_path: String,
    pub header_lines: Vec<String>,
    pub hunks: Vec<DiffHunk>,
    pub is_binary: bool,
    pub is_untracked: bool,
    pub additions: usize,
    pub deletions: usize,
}

impl FileDiff {
    /// Genera una patch Unified Diff valida per un singolo hunk.
    pub fn build_hunk_patch(&self, hunk_idx: usize) -> Option<String> {
        let hunk = self.hunks.get(hunk_idx)?;
        let all_changed: BTreeSet<usize> = hunk
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.kind != DiffLineKind::Context)
            .map(|(i, _)| i)
            .collect();
        self.build_selected_lines_patch(hunk_idx, &all_changed, false)
    }

    /// Genera una patch Unified Diff per un sottoinsieme di righe selezionate in un hunk.
    /// - Se `reverse == false` (usato per Stage Selected Lines con `git apply --cached`):
    ///   - Le righe `Removed` NON selezionate diventano righe di contesto (`' '`).
    ///   - Le righe `Added` NON selezionate vengono omesse.
    /// - Se `reverse == true` (usato per Unstage o Revert Selected Lines con `git apply [--cached] --reverse`):
    ///   - Le righe `Added` NON selezionate diventano righe di contesto (`' '`).
    ///   - Le righe `Removed` NON selezionate vengono omesse.
    pub fn build_selected_lines_patch(
        &self,
        hunk_idx: usize,
        selected_line_indices: &BTreeSet<usize>,
        reverse: bool,
    ) -> Option<String> {
        let hunk = self.hunks.get(hunk_idx)?;

        let mut body_lines: Vec<String> = Vec::new();
        let mut old_count = 0usize;
        let mut new_count = 0usize;
        let mut has_any_change = false;

        for (idx, line) in hunk.lines.iter().enumerate() {
            let is_selected = selected_line_indices.contains(&idx);
            match line.kind {
                DiffLineKind::Context => {
                    body_lines.push(format!(" {}", line.content));
                    old_count += 1;
                    new_count += 1;
                }
                DiffLineKind::Removed => {
                    if is_selected {
                        body_lines.push(format!("-{}", line.content));
                        old_count += 1;
                        has_any_change = true;
                        if line.no_newline_after {
                            body_lines.push("\\ No newline at end of file".to_string());
                        }
                    } else if !reverse {
                        // In forward mode (staging), unselected removal stays in index -> context
                        body_lines.push(format!(" {}", line.content));
                        old_count += 1;
                        new_count += 1;
                        if line.no_newline_after {
                            body_lines.push("\\ No newline at end of file".to_string());
                        }
                    }
                }
                DiffLineKind::Added => {
                    if is_selected {
                        body_lines.push(format!("+{}", line.content));
                        new_count += 1;
                        has_any_change = true;
                        if line.no_newline_after {
                            body_lines.push("\\ No newline at end of file".to_string());
                        }
                    } else if reverse {
                        // In reverse mode (unstaging/reverting), unselected addition stays -> context
                        body_lines.push(format!(" {}", line.content));
                        old_count += 1;
                        new_count += 1;
                        if line.no_newline_after {
                            body_lines.push("\\ No newline at end of file".to_string());
                        }
                    }
                }
            }
        }

        if !has_any_change {
            return None;
        }

        let mut patch = String::new();
        if self.header_lines.is_empty() {
            patch.push_str(&format!("--- a/{}\n", self.old_path));
            patch.push_str(&format!("+++ b/{}\n", self.new_path));
        } else {
            for h in &self.header_lines {
                patch.push_str(h);
                patch.push('\n');
            }
        }

        patch.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.old_start, old_count, hunk.new_start, new_count
        ));
        for bl in body_lines {
            patch.push_str(&bl);
            patch.push('\n');
        }

        Some(patch)
    }
}

/// Parsa un header hunk del tipo `@@ -12,5 +12,7 @@ optional context`
fn parse_hunk_header(line: &str) -> Option<(usize, usize, usize, usize, String)> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, section) = rest.split_once(" @@")?;
    let (old_part, new_part) = ranges.split_once(" +")?;

    let parse_range = |s: &str| -> (usize, usize) {
        if let Some((start, count)) = s.split_once(',') {
            (start.parse().unwrap_or(1), count.parse().unwrap_or(1))
        } else {
            (s.parse().unwrap_or(1), 1)
        }
    };

    let (old_start, old_count) = parse_range(old_part);
    let (new_start, new_count) = parse_range(new_part);

    Some((
        old_start,
        old_count,
        new_start,
        new_count,
        section.trim().to_string(),
    ))
}

/// Parsa l'output di `git diff` in `FileDiff`.
pub fn parse_unified_diff(raw: &str, target: DiffTarget) -> FileDiff {
    let default_path = target.path().to_string();
    let mut diff = FileDiff {
        target,
        old_path: default_path.clone(),
        new_path: default_path,
        header_lines: Vec::new(),
        hunks: Vec::new(),
        is_binary: false,
        is_untracked: false,
        additions: 0,
        deletions: 0,
    };

    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_ln = 0usize;
    let mut new_ln = 0usize;

    for line in raw.lines() {
        if current_hunk.is_none() {
            if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
                diff.is_binary = true;
                continue;
            }
            if let Some(old_p) = line.strip_prefix("--- a/") {
                diff.old_path = old_p.to_string();
                diff.header_lines.push(line.to_string());
                continue;
            }
            if let Some(new_p) = line.strip_prefix("+++ b/") {
                diff.new_path = new_p.to_string();
                diff.header_lines.push(line.to_string());
                continue;
            }
            if line.starts_with("diff --git ")
                || line.starts_with("index ")
                || line.starts_with("new file mode")
                || line.starts_with("deleted file mode")
                || line.starts_with("similarity index")
                || line.starts_with("rename from")
                || line.starts_with("rename to")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
            {
                diff.header_lines.push(line.to_string());
                continue;
            }
        }

        if line.starts_with("@@ ") {
            if let Some(h) = current_hunk.take() {
                diff.hunks.push(h);
            }
            if let Some((old_start, old_count, new_start, new_count, section)) =
                parse_hunk_header(line)
            {
                old_ln = old_start;
                new_ln = new_start;
                current_hunk = Some(DiffHunk {
                    section_context: section,
                    old_start,
                    old_count,
                    new_start,
                    new_count,
                    lines: Vec::new(),
                });
            }
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            if let Some(content) = line.strip_prefix('+') {
                diff.additions += 1;
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Added,
                    old_lineno: None,
                    new_lineno: Some(new_ln),
                    content: content.to_string(),
                    no_newline_after: false,
                });
                new_ln += 1;
            } else if let Some(content) = line.strip_prefix('-') {
                diff.deletions += 1;
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Removed,
                    old_lineno: Some(old_ln),
                    new_lineno: None,
                    content: content.to_string(),
                    no_newline_after: false,
                });
                old_ln += 1;
            } else if let Some(content) = line.strip_prefix(' ') {
                hunk.lines.push(DiffLine {
                    kind: DiffLineKind::Context,
                    old_lineno: Some(old_ln),
                    new_lineno: Some(new_ln),
                    content: content.to_string(),
                    no_newline_after: false,
                });
                old_ln += 1;
                new_ln += 1;
            } else if line.starts_with("\\ No newline at end of file") {
                if let Some(last) = hunk.lines.last_mut() {
                    last.no_newline_after = true;
                }
            }
        }
    }

    if let Some(h) = current_hunk.take() {
        diff.hunks.push(h);
    }

    diff
}

/// Carica il `FileDiff` per un elemento selezionato (WorkingTree, Staged o Commit).
pub fn fetch_file_diff(
    repo_path: &Path,
    target: &DiffTarget,
) -> Result<(FileDiff, Option<GitCommandLog>), String> {
    match target {
        DiffTarget::WorkingTree {
            path,
            kind: FileStatusKind::Untracked,
        } => {
            // File untracked: leggi direttamente da disco e mostra tutte le righe come Added
            let full_path = repo_path.join(path);
            let bytes = fs::read(&full_path)
                .map_err(|e| format!("Impossibile leggere {}: {}", path, e))?;
            if bytes.iter().take(8000).any(|&b| b == 0) {
                return Ok((
                    FileDiff {
                        target: target.clone(),
                        old_path: path.clone(),
                        new_path: path.clone(),
                        header_lines: Vec::new(),
                        hunks: Vec::new(),
                        is_binary: true,
                        is_untracked: true,
                        additions: 0,
                        deletions: 0,
                    },
                    None,
                ));
            }
            let text = String::from_utf8_lossy(&bytes);
            let lines_vec: Vec<&str> = text.lines().collect();
            let count = lines_vec.len();
            let diff_lines: Vec<DiffLine> = lines_vec
                .into_iter()
                .enumerate()
                .map(|(i, l)| DiffLine {
                    kind: DiffLineKind::Added,
                    old_lineno: None,
                    new_lineno: Some(i + 1),
                    content: l.to_string(),
                    no_newline_after: false,
                })
                .collect();

            let hunk = DiffHunk {
                section_context: "Untracked File".to_string(),
                old_start: 0,
                old_count: 0,
                new_start: 1,
                new_count: count,
                lines: diff_lines,
            };

            Ok((
                FileDiff {
                    target: target.clone(),
                    old_path: "/dev/null".to_string(),
                    new_path: path.clone(),
                    header_lines: vec![
                        format!("diff --git a/{0} b/{0}", path),
                        "new file mode 100644".to_string(),
                        "--- /dev/null".to_string(),
                        format!("+++ b/{}", path),
                    ],
                    hunks: if count > 0 { vec![hunk] } else { Vec::new() },
                    is_binary: false,
                    is_untracked: true,
                    additions: count,
                    deletions: 0,
                },
                None,
            ))
        }
        DiffTarget::WorkingTree { path, .. } => {
            let out = run_git(repo_path, &["diff", "--", path], None, false)?;
            let parsed = parse_unified_diff(&out.stdout, target.clone());
            Ok((parsed, Some(out.log)))
        }
        DiffTarget::Staged { path, .. } => {
            let out = run_git(repo_path, &["diff", "--cached", "--", path], None, false)?;
            let parsed = parse_unified_diff(&out.stdout, target.clone());
            Ok((parsed, Some(out.log)))
        }
        DiffTarget::Commit { commit_sha, path } => {
            let out = run_git(
                repo_path,
                &["show", "--format=", commit_sha, "--", path],
                None,
                false,
            )?;
            let parsed = parse_unified_diff(&out.stdout, target.clone());
            Ok((parsed, Some(out.log)))
        }
    }
}

/// Costruisce un `DiffTarget` a partire da una `FileChange`.
pub fn target_from_change(change: &crate::git::status::FileChange) -> DiffTarget {
    match change.group {
        ChangeGroup::Staged => DiffTarget::Staged {
            path: change.path.clone(),
            kind: change.kind,
        },
        ChangeGroup::WorkingTree | ChangeGroup::Merge => DiffTarget::WorkingTree {
            path: change.path.clone(),
            kind: change.kind,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,4 +1,5 @@
 fn hello() {
-    println!(\"old 1\");
-    println!(\"old 2\");
+    println!(\"new 1\");
+    println!(\"new 2\");
+    println!(\"new 3\");
 }
";

    #[test]
    fn test_parse_and_partial_stage_patch() {
        let target = DiffTarget::WorkingTree {
            path: "src/lib.rs".to_string(),
            kind: FileStatusKind::Modified,
        };
        let diff = parse_unified_diff(SAMPLE_DIFF, target);
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.additions, 3);
        assert_eq!(diff.deletions, 2);

        let rows = diff.hunks[0].build_side_by_side_rows();
        // 1 context + 3 paired/unpaired change rows + 1 context = 5 rows
        assert_eq!(rows.len(), 5);

        // Select only line index 1 ("-    println!(\"old 1\");") and line index 3 ("+    println!(\"new 1\");")
        let mut selected = BTreeSet::new();
        selected.insert(1);
        selected.insert(3);

        // Forward patch (Stage Selected Lines):
        // Line 0: Context (old 1, new 1)
        // Line 1: Removed selected -> '-' (old 1)
        // Line 2: Removed unselected -> ' ' context (old 1, new 1)
        // Line 3: Added selected -> '+' (new 1)
        // Line 4,5: Added unselected -> omitted
        // Line 6: Context (old 1, new 1)
        // Total old = 4, new = 4
        let patch = diff
            .build_selected_lines_patch(0, &selected, false)
            .expect("patch generated");
        assert!(patch.contains("@@ -1,4 +1,4 @@"));
        assert!(patch.contains("-    println!(\"old 1\");\n"));
        assert!(patch.contains("     println!(\"old 2\");\n"));
        assert!(patch.contains("+    println!(\"new 1\");\n"));
        assert!(!patch.contains("new 2"));
    }

    #[test]
    fn test_partial_reverse_patch() {
        let target = DiffTarget::Staged {
            path: "src/lib.rs".to_string(),
            kind: FileStatusKind::Modified,
        };
        let diff = parse_unified_diff(SAMPLE_DIFF, target);

        // Select only line index 4 ("+    println!(\"new 2\");") to unstage/revert
        let mut selected = BTreeSet::new();
        selected.insert(4);

        // Reverse patch:
        // Unselected Removed (1, 2) are omitted
        // Unselected Added (3, 5) become context ' '
        // Selected Added (4) remains '+'
        // Context (0, 6) remain ' '
        // old_count = 2 context + 2 unselected added = 4
        // new_count = 4 + 1 selected added = 5
        let patch = diff
            .build_selected_lines_patch(0, &selected, true)
            .expect("reverse patch generated");
        assert!(patch.contains("@@ -1,4 +1,5 @@"));
        assert!(patch.contains("     println!(\"new 1\");\n"));
        assert!(patch.contains("+    println!(\"new 2\");\n"));
        assert!(patch.contains("     println!(\"new 3\");\n"));
        assert!(!patch.contains("old 1"));
    }

    #[test]
    fn test_real_git_apply_selected_lines_e2e() {
        let tmp = std::env::temp_dir().join(format!("vscode_git_egui_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        run_git(&tmp, &["init"], None, true).unwrap();
        run_git(&tmp, &["config", "user.email", "test@example.com"], None, true).unwrap();
        run_git(&tmp, &["config", "user.name", "Test User"], None, true).unwrap();

        let file_path = tmp.join("demo.rs");
        fs::write(&file_path, "fn main() {\n    let a = 1;\n    let b = 2;\n}\n").unwrap();
        crate::git::ops::stage_all(&tmp).unwrap();
        crate::git::ops::commit(&tmp, "initial commit", false, false, false).unwrap();

        // Modifica due righe e aggiungine una terza
        fs::write(
            &file_path,
            "fn main() {\n    let a = 10;\n    let b = 20;\n    let c = 30;\n}\n",
        )
        .unwrap();

        let target = DiffTarget::WorkingTree {
            path: "demo.rs".to_string(),
            kind: FileStatusKind::Modified,
        };
        let (diff, _) = fetch_file_diff(&tmp, &target).unwrap();
        assert_eq!(diff.hunks.len(), 1);

        // Seleziona solo la prima rimozione (`-    let a = 1;`, idx 1) e la prima aggiunta (`+    let a = 10;`, idx 3)
        let mut selected = BTreeSet::new();
        selected.insert(1);
        selected.insert(3);

        let patch = diff
            .build_selected_lines_patch(0, &selected, false)
            .expect("patch created");
        crate::git::ops::apply_patch(&tmp, &patch, true, false)
            .expect("git apply --cached succeeded");

        // Verifica che lo Staged Diff contenga solo `let a = 10;`
        let staged_target = DiffTarget::Staged {
            path: "demo.rs".to_string(),
            kind: FileStatusKind::Modified,
        };
        let (staged_diff, _) = fetch_file_diff(&tmp, &staged_target).unwrap();
        assert_eq!(staged_diff.additions, 1);
        assert_eq!(staged_diff.deletions, 1);

        // Ora fai Unstage di quell'hunk tramite apply_patch reverse
        let unstage_patch = staged_diff.build_hunk_patch(0).unwrap();
        crate::git::ops::apply_patch(&tmp, &unstage_patch, true, true)
            .expect("git apply --cached --reverse succeeded");

        let (staged_after, _) = fetch_file_diff(&tmp, &staged_target).unwrap();
        assert!(staged_after.hunks.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }
}
