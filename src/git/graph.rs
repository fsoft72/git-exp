use crate::git::cli::{GitCommandLog, run_git};
use crate::git::status::FileStatusKind;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefBadgeKind {
    HeadBranch,
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone)]
pub struct RefBadge {
    pub name: String,
    pub kind: RefBadgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSyncRole {
    Outgoing, // Ahead of upstream
    Incoming, // Behind upstream
    History,  // Normal merged/shared history
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from_lane: usize,
    pub to_lane: usize,
    pub color_idx: usize,
}

#[derive(Debug, Clone)]
pub struct CommitNode {
    pub sha: String,
    pub short_sha: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_email: String,
    pub relative_time: String,
    pub iso_date: String,
    pub subject: String,
    pub refs: Vec<RefBadge>,
    pub is_head: bool,
    pub role: CommitSyncRole,
    // Layout corsie calcolato per il grafo visuale
    pub node_lane: usize,
    pub node_color_idx: usize,
    pub pass_through_lanes: Vec<(usize, usize)>, // (lane, color_idx)
    pub edges_to_next_row: Vec<GraphEdge>,
}

#[derive(Debug, Clone)]
pub struct CommitFileEntry {
    pub path: String,
    pub kind: FileStatusKind,
}

#[derive(Debug, Clone)]
pub struct CommitDetails {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub message: String,
    pub parents: Vec<String>,
    pub files: Vec<CommitFileEntry>,
}

/// Parsa i decoratori `%D` di `git log` (es. `HEAD -> main, tag: v0.1.0, origin/main`).
fn parse_ref_badges(decorations: &str) -> (bool, Vec<RefBadge>) {
    let mut is_head = false;
    let mut badges = Vec::new();
    if decorations.trim().is_empty() {
        return (false, badges);
    }

    for part in decorations.split(", ") {
        let part = part.trim();
        if part.is_empty() || part.ends_with("/HEAD") {
            continue;
        }
        if let Some(branch) = part.strip_prefix("HEAD -> ") {
            is_head = true;
            badges.push(RefBadge {
                name: branch.to_string(),
                kind: RefBadgeKind::HeadBranch,
            });
        } else if part == "HEAD" {
            is_head = true;
        } else if let Some(tag) = part.strip_prefix("tag: ") {
            badges.push(RefBadge {
                name: tag.to_string(),
                kind: RefBadgeKind::Tag,
            });
        } else if part.contains('/') {
            badges.push(RefBadge {
                name: part.to_string(),
                kind: RefBadgeKind::RemoteBranch,
            });
        } else {
            badges.push(RefBadge {
                name: part.to_string(),
                kind: RefBadgeKind::LocalBranch,
            });
        }
    }

    (is_head, badges)
}

/// Assegna a ogni commit una corsia (`node_lane`), i segmenti passanti e gli archi verso la riga successiva.
pub fn compute_graph_lanes(commits: &mut [CommitNode]) {
    // Ogni slot attivo contiene (target_sha, color_idx)
    let mut active_lanes: Vec<Option<(String, usize)>> = Vec::new();
    let mut next_color: usize = 0;

    for commit in commits.iter_mut() {
        // 1. Trova se questo commit era già atteso in una o più corsie
        let mut matching_lanes: Vec<usize> = active_lanes
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                if slot.as_ref().map(|(sha, _)| sha == &commit.sha).unwrap_or(false) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        let (node_lane, node_color) = if let Some(&first_lane) = matching_lanes.first() {
            let color = active_lanes[first_lane].as_ref().unwrap().1;
            (first_lane, color)
        } else {
            // Nuova corsia (es. branch head)
            let color = next_color;
            next_color = (next_color + 1) % 6;
            if let Some(empty_idx) = active_lanes.iter().position(|s| s.is_none()) {
                active_lanes[empty_idx] = Some((commit.sha.clone(), color));
                matching_lanes.push(empty_idx);
                (empty_idx, color)
            } else {
                let idx = active_lanes.len();
                active_lanes.push(Some((commit.sha.clone(), color)));
                matching_lanes.push(idx);
                (idx, color)
            }
        };

        commit.node_lane = node_lane;
        commit.node_color_idx = node_color;

        // Corsie che passano dritte senza toccare questo commit
        let mut pass_through = Vec::new();
        for (lane_idx, slot) in active_lanes.iter().enumerate() {
            if let Some((sha, col)) = slot {
                if sha != &commit.sha {
                    pass_through.push((lane_idx, *col));
                }
            }
        }
        commit.pass_through_lanes = pass_through.clone();

        // Libera tutte le corsie che convergevano su questo commit
        for &ml in &matching_lanes {
            active_lanes[ml] = None;
        }

        let mut edges = Vec::new();
        // Aggiungi archi verticali per tutte le corsie passanti
        for &(lane_idx, col) in &pass_through {
            edges.push(GraphEdge {
                from_lane: lane_idx,
                to_lane: lane_idx,
                color_idx: col,
            });
        }

        // Ora posiziona i genitori di questo commit
        for (p_idx, parent_sha) in commit.parents.iter().enumerate() {
            // Se un'altra corsia sta già puntando a `parent_sha`, colleghiamoci a quella corsia!
            if let Some(existing_lane) = active_lanes.iter().position(|s| {
                s.as_ref()
                    .map(|(sha, _)| sha == parent_sha)
                    .unwrap_or(false)
            }) {
                let edge_color = if p_idx == 0 {
                    node_color
                } else {
                    active_lanes[existing_lane].as_ref().unwrap().1
                };
                edges.push(GraphEdge {
                    from_lane: node_lane,
                    to_lane: existing_lane,
                    color_idx: edge_color,
                });
            } else if p_idx == 0 && active_lanes[node_lane].is_none() {
                // Il primo genitore eredita la corsia del commit stesso
                active_lanes[node_lane] = Some((parent_sha.clone(), node_color));
                edges.push(GraphEdge {
                    from_lane: node_lane,
                    to_lane: node_lane,
                    color_idx: node_color,
                });
            } else {
                // Genitore secondario (merge) -> alloca uno slot libero o nuovo
                let p_color = next_color;
                next_color = (next_color + 1) % 6;
                let target_lane = if let Some(empty_idx) = active_lanes.iter().position(|s| s.is_none()) {
                    active_lanes[empty_idx] = Some((parent_sha.clone(), p_color));
                    empty_idx
                } else {
                    let idx = active_lanes.len();
                    active_lanes.push(Some((parent_sha.clone(), p_color)));
                    idx
                };
                edges.push(GraphEdge {
                    from_lane: node_lane,
                    to_lane: target_lane,
                    color_idx: p_color,
                });
            }
        }

        // Se una corsia secondaria convergeva su questo commit (oltre a `node_lane`),
        // in realtà quella corsia proveniva dalla riga precedente; i suoi figli erano già stati collegati.
        // Rimuovi gli slot None in coda per mantenere compatto il grafo.
        while active_lanes.last().map(|s| s.is_none()).unwrap_or(false) {
            active_lanes.pop();
        }

        commit.edges_to_next_row = edges;
    }
}

/// Carica il grafo della cronologia commit (inclusi Incoming/Outgoing rispetto all'upstream).
pub fn fetch_commit_graph(
    repo_path: &Path,
    upstream: Option<&str>,
    max_count: usize,
) -> Result<(Vec<CommitNode>, Vec<GitCommandLog>), String> {
    let mut logs = Vec::new();
    let mut outgoing_shas = HashSet::new();
    let mut incoming_shas = HashSet::new();

    if let Some(up) = upstream {
        // Outgoing: ahead of upstream (up..HEAD)
        if let Ok(out) = run_git(
            repo_path,
            &["rev-list", &format!("{}..HEAD", up)],
            None,
            false,
        ) {
            if out.is_success() {
                for l in out.stdout.lines() {
                    outgoing_shas.insert(l.trim().to_string());
                }
            }
        }
        // Incoming: behind upstream (HEAD..up)
        if let Ok(inc) = run_git(
            repo_path,
            &["rev-list", &format!("HEAD..{}", up)],
            None,
            false,
        ) {
            if inc.is_success() {
                for l in inc.stdout.lines() {
                    incoming_shas.insert(l.trim().to_string());
                }
            }
        }
    }

    let limit_arg = format!("-n{}", max_count);
    let format_arg = "--format=%H%x1f%h%x1f%P%x1f%an%x1f%ae%x1f%cr%x1f%ci%x1f%D%x1f%s";
    let log_out = run_git(
        repo_path,
        &["log", "--all", "--topo-order", &limit_arg, format_arg],
        None,
        false,
    )?;
    logs.push(log_out.log.clone());

    if !log_out.is_success() {
        // Repo vuoto senza commit
        return Ok((Vec::new(), logs));
    }

    let mut nodes = Vec::new();
    for line in log_out.stdout.lines() {
        let fields: Vec<&str> = line.split('\x1f').collect();
        if fields.len() < 9 {
            continue;
        }
        let sha = fields[0].trim().to_string();
        let short_sha = fields[1].trim().to_string();
        let parents: Vec<String> = fields[2]
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let author = fields[3].trim().to_string();
        let author_email = fields[4].trim().to_string();
        let relative_time = fields[5].trim().to_string();
        let iso_date = fields[6].trim().to_string();
        let (is_head, refs) = parse_ref_badges(fields[7]);
        let subject = fields[8].to_string();

        let role = if outgoing_shas.contains(&sha) {
            CommitSyncRole::Outgoing
        } else if incoming_shas.contains(&sha) {
            CommitSyncRole::Incoming
        } else {
            CommitSyncRole::History
        };

        nodes.push(CommitNode {
            sha,
            short_sha,
            parents,
            author,
            author_email,
            relative_time,
            iso_date,
            subject,
            refs,
            is_head,
            role,
            node_lane: 0,
            node_color_idx: 0,
            pass_through_lanes: Vec::new(),
            edges_to_next_row: Vec::new(),
        });
    }

    compute_graph_lanes(&mut nodes);

    Ok((nodes, logs))
}

/// Carica i dettagli completi e la lista dei file modificati in uno specifico commit.
pub fn fetch_commit_details(
    repo_path: &Path,
    commit_sha: &str,
) -> Result<(CommitDetails, GitCommandLog), String> {
    let out = run_git(
        repo_path,
        &[
            "show",
            "--name-status",
            "--format=%H%n%h%n%an%n%ae%n%ci%n%P%n%B%x00",
            commit_sha,
        ],
        None,
        false,
    )?;

    if !out.is_success() {
        return Err(out.stderr);
    }

    let (header_part, files_part) = out
        .stdout
        .split_once('\0')
        .unwrap_or((&out.stdout, ""));

    let mut lines = header_part.lines();
    let sha = lines.next().unwrap_or(commit_sha).to_string();
    let short_sha = lines.next().unwrap_or("").to_string();
    let author = lines.next().unwrap_or("").to_string();
    let email = lines.next().unwrap_or("").to_string();
    let date = lines.next().unwrap_or("").to_string();
    let parents: Vec<String> = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let message: String = lines.collect::<Vec<&str>>().join("\n").trim().to_string();

    let mut files = Vec::new();
    for fline in files_part.lines() {
        let fline = fline.trim();
        if fline.is_empty() {
            continue;
        }
        let parts: Vec<&str> = fline.split('\t').collect();
        if parts.len() >= 2 {
            let status_char = parts[0].chars().next().unwrap_or('M');
            let kind = FileStatusKind::from_porcelain_char(status_char)
                .unwrap_or(FileStatusKind::Modified);
            let path = parts.last().unwrap_or(&parts[1]).to_string();
            files.push(CommitFileEntry { path, kind });
        }
    }

    Ok((
        CommitDetails {
            sha,
            short_sha,
            author,
            email,
            date,
            message,
            parents,
            files,
        },
        out.log,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_graph_lanes_branch_and_merge() {
        let mut commits = vec![
            CommitNode {
                sha: "c3".into(),
                short_sha: "c3".into(),
                parents: vec!["c1".into(), "c2".into()],
                author: "A".into(),
                author_email: "a@b".into(),
                relative_time: "now".into(),
                iso_date: "".into(),
                subject: "Merge branch".into(),
                refs: vec![],
                is_head: true,
                role: CommitSyncRole::History,
                node_lane: 0,
                node_color_idx: 0,
                pass_through_lanes: vec![],
                edges_to_next_row: vec![],
            },
            CommitNode {
                sha: "c2".into(),
                short_sha: "c2".into(),
                parents: vec!["c1".into()],
                author: "A".into(),
                author_email: "a@b".into(),
                relative_time: "1m".into(),
                iso_date: "".into(),
                subject: "Feature".into(),
                refs: vec![],
                is_head: false,
                role: CommitSyncRole::History,
                node_lane: 0,
                node_color_idx: 0,
                pass_through_lanes: vec![],
                edges_to_next_row: vec![],
            },
            CommitNode {
                sha: "c1".into(),
                short_sha: "c1".into(),
                parents: vec![],
                author: "A".into(),
                author_email: "a@b".into(),
                relative_time: "2m".into(),
                iso_date: "".into(),
                subject: "Init".into(),
                refs: vec![],
                is_head: false,
                role: CommitSyncRole::History,
                node_lane: 0,
                node_color_idx: 0,
                pass_through_lanes: vec![],
                edges_to_next_row: vec![],
            },
        ];

        compute_graph_lanes(&mut commits);
        assert_eq!(commits[0].node_lane, 0);
        assert_eq!(commits[0].edges_to_next_row.len(), 2); // branches to lane 0 and lane 1
        assert_eq!(commits[1].node_lane, 1); // c2 is on lane 1
        assert_eq!(commits[2].node_lane, 0); // c1 is on lane 0
    }
}

