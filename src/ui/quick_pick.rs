use crate::git::ops::{self, CommitVariant};
use crate::state::{AppState, QuickPickKind, QuickPickState};
use crate::theme::VsCodeTheme;
use egui::{Color32, CornerRadius, Key, Order, Pos2, RichText, Sense, Stroke, Vec2};

struct QuickPickItem {
    label: String,
    detail: String,
    action: Box<dyn FnOnce(&mut AppState)>,
}

pub fn render_quick_pick_modal(ctx: &egui::Context, state: &mut AppState) {
    // Scorciatoia globale Ctrl+Shift+P o F1 per aprire la Command Palette
    if ctx.input(|i| {
        (i.key_pressed(Key::P) && i.modifiers.command && i.modifiers.shift)
            || i.key_pressed(Key::F1)
    }) {
        state.quick_pick = Some(QuickPickState::new(QuickPickKind::CommandPalette));
    }

    let Some(qp) = state.quick_pick.clone() else {
        return;
    };

    // Chiudi con Escape
    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.quick_pick = None;
        return;
    }

    let screen = ctx.screen_rect();
    let modal_w = 560.0f32.min(screen.width() - 32.0);
    let modal_x = screen.center().x - modal_w * 0.5;
    let modal_y = screen.top() + 36.0;

    egui::Area::new(egui::Id::new("vscode_quick_pick_area"))
        .order(Order::Foreground)
        .fixed_pos(Pos2::new(modal_x, modal_y))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(VsCodeTheme::QUICK_PICK_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::FOCUS_BORDER))
                .corner_radius(CornerRadius::same(5))
                .shadow(egui::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 2,
                    color: Color32::from_black_alpha(140),
                })
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(modal_w);
                    render_quick_pick_body(ui, state, &qp);
                });
        });
}

fn render_quick_pick_body(ui: &mut egui::Ui, state: &mut AppState, qp: &QuickPickState) {
    let title = match &qp.kind {
        QuickPickKind::CommandPalette => "Git Command Palette",
        QuickPickKind::Checkout => "Git: Checkout to... (Select a ref to checkout)",
        QuickPickKind::CreateBranch { from_ref: None } => {
            "Git: Create Branch (Enter new branch name and press Enter)"
        }
        QuickPickKind::CreateBranch {
            from_ref: Some(r),
        } => &format!("Git: Create Branch from '{}' (Enter branch name)", r),
        QuickPickKind::DeleteBranch { .. } => "Git: Delete Branch (Select a branch to delete)",
        QuickPickKind::RenameBranch => "Git: Rename Current Branch (Enter new name)",
        QuickPickKind::MergeBranch => "Git: Merge Branch into current branch",
        QuickPickKind::RebaseBranch => "Git: Rebase Current Branch onto...",
        QuickPickKind::StashPush {
            include_untracked: false,
        } => "Git: Stash Changes (Optional message, press Enter to stash)",
        QuickPickKind::StashPush {
            include_untracked: true,
        } => "Git: Stash Including Untracked (Optional message, press Enter)",
        QuickPickKind::StashPop => "Git: Pop Stash (Select a stash to pop)",
        QuickPickKind::StashApply => "Git: Apply Stash (Select a stash to apply)",
        QuickPickKind::StashDrop => "Git: Drop Stash (Select a stash to delete)",
        QuickPickKind::CreateTag => "Git: Create Tag at HEAD (Enter tag name and press Enter)",
        QuickPickKind::DeleteTag => "Git: Delete Tag (Select a tag to delete)",
        QuickPickKind::ConfirmDiscardAll => {
            "⚠ Discard ALL working tree & untracked changes? (Cannot be undone)"
        }
        QuickPickKind::ConfirmDiscardFile { path, .. } => {
            &format!("⚠ Discard changes in '{}'?", path)
        }
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(11.5)
                .strong()
                .color(VsCodeTheme::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button(RichText::new("Esc ×").size(11.0).color(VsCodeTheme::TEXT_MUTED))
                .clicked()
            {
                state.quick_pick = None;
            }
        });
    });
    ui.add_space(4.0);

    // Input di testo / filtro
    let mut query = qp.query.clone();
    let mut submitted_enter = false;

    egui::Frame::NONE
        .fill(VsCodeTheme::INPUT_BG)
        .stroke(Stroke::new(1.0_f32, VsCodeTheme::FOCUS_BORDER))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            let hint = match &qp.kind {
                QuickPickKind::CreateBranch { .. } => "New branch name (e.g. feat/new-ui)...",
                QuickPickKind::RenameBranch => "New branch name...",
                QuickPickKind::StashPush { .. } => "Optional stash message (press Enter)...",
                QuickPickKind::CreateTag => "Tag name (e.g. v0.2.0)...",
                _ => "Type to filter commands or branches...",
            };
            let te = egui::TextEdit::singleline(&mut query)
                .hint_text(RichText::new(hint).color(VsCodeTheme::TEXT_MUTED))
                .desired_width(f32::INFINITY)
                .frame(false);
            let resp = ui.add(te);
            if qp.just_opened {
                resp.request_focus();
            }
            if ui.input(|i| i.key_pressed(Key::Enter)) {
                submitted_enter = true;
            }
        });

    if let Some(ref mut current_qp) = state.quick_pick {
        if current_qp.query != query {
            current_qp.query = query.clone();
            current_qp.selected_index = 0;
        }
        current_qp.just_opened = false;
    }

    // Se è una modale di puro input (CreateBranch, RenameBranch, StashPush, CreateTag)
    match &qp.kind {
        QuickPickKind::CreateBranch { from_ref } => {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Press Enter to create and checkout the new branch.")
                    .size(11.5)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            if submitted_enter && !query.trim().is_empty() {
                let name = query.trim().to_string();
                let base = from_ref.clone();
                state.quick_pick = None;
                state.spawn_git_task("Creating branch...", move |repo| {
                    ops::create_branch(repo, &name, base.as_deref(), true)
                        .map(|l| (format!("Creato e attivato branch '{}'", name), vec![l]))
                        .map_err(|(e, l)| (e, vec![l]))
                });
            }
            return;
        }
        QuickPickKind::RenameBranch => {
            if submitted_enter && !query.trim().is_empty() {
                let name = query.trim().to_string();
                state.quick_pick = None;
                state.spawn_git_task("Renaming branch...", move |repo| {
                    ops::rename_branch(repo, &name)
                        .map(|l| (format!("Branch rinominato in '{}'", name), vec![l]))
                        .map_err(|(e, l)| (e, vec![l]))
                });
            }
            return;
        }
        QuickPickKind::StashPush { include_untracked } => {
            let inc = *include_untracked;
            ui.add_space(6.0);
            ui.label(
                RichText::new("Press Enter to save stash (leave empty for default message).")
                    .size(11.5)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            if submitted_enter {
                let msg = if query.trim().is_empty() {
                    None
                } else {
                    Some(query.trim().to_string())
                };
                state.quick_pick = None;
                state.spawn_git_task("Stashing changes...", move |repo| {
                    ops::stash_push(repo, msg.as_deref(), inc)
                        .map(|l| ("Modifiche salvate nello Stash".into(), vec![l]))
                        .map_err(|(e, l)| (e, vec![l]))
                });
            }
            return;
        }
        QuickPickKind::CreateTag => {
            if submitted_enter && !query.trim().is_empty() {
                let tag_name = query.trim().to_string();
                state.quick_pick = None;
                state.spawn_git_task("Creating tag...", move |repo| {
                    ops::create_tag(repo, &tag_name, None)
                        .map(|l| (format!("Tag '{}' creato", tag_name), vec![l]))
                        .map_err(|(e, l)| (e, vec![l]))
                });
            }
            return;
        }
        _ => {}
    }

    // Costruisci la lista di elementi selezionabili per i picker a lista
    let items = build_quick_pick_items(state, &qp.kind, &query);

    let item_count = items.len();
    let mut selected_idx = qp.selected_index.min(item_count.saturating_sub(1));

    if item_count > 0 {
        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
            selected_idx = (selected_idx + 1) % item_count;
            if let Some(ref mut current_qp) = state.quick_pick {
                current_qp.selected_index = selected_idx;
            }
        }
        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
            selected_idx = if selected_idx == 0 {
                item_count - 1
            } else {
                selected_idx - 1
            };
            if let Some(ref mut current_qp) = state.quick_pick {
                current_qp.selected_index = selected_idx;
            }
        }
    }

    ui.add_space(4.0);
    let mut triggered_index: Option<usize> = if submitted_enter && item_count > 0 {
        Some(selected_idx)
    } else {
        None
    };

    egui::ScrollArea::vertical()
        .id_salt("quick_pick_list_scroll")
        .max_height(320.0)
        .show(ui, |ui| {
            for (idx, item) in items.iter().enumerate() {
                let is_sel = idx == selected_idx;
                let row_h = 26.0;
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
                let hovered = resp.hovered() || ui.rect_contains_pointer(rect);

                if is_sel {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(3),
                        VsCodeTheme::LIST_ACTIVE_SELECTION_BG,
                    );
                } else if hovered {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::same(3),
                        VsCodeTheme::LIST_HOVER_BG,
                    );
                }

                let mut row_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                row_ui.add_space(8.0);
                row_ui.label(
                    RichText::new(&item.label).size(12.5).color(if is_sel {
                        VsCodeTheme::TEXT_BRIGHT
                    } else {
                        VsCodeTheme::TEXT_PRIMARY
                    }),
                );

                if !item.detail.is_empty() {
                    row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(&item.detail)
                                .size(11.0)
                                .color(VsCodeTheme::TEXT_MUTED),
                        );
                    });
                }

                if resp.clicked() {
                    triggered_index = Some(idx);
                }
            }
        });

    if let Some(chosen) = triggered_index {
        state.quick_pick = None;
        if let Some(item) = items.into_iter().nth(chosen) {
            (item.action)(state);
        }
    }
}

fn build_quick_pick_items(
    state: &AppState,
    kind: &QuickPickKind,
    query: &str,
) -> Vec<QuickPickItem> {
    let q = query.trim().to_ascii_lowercase();
    let mut raw: Vec<QuickPickItem> = Vec::new();

    match kind {
        QuickPickKind::Checkout => {
            raw.push(QuickPickItem {
                label: "+ Create new branch...".to_string(),
                detail: "git checkout -b".to_string(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                        from_ref: None,
                    }));
                }),
            });

            for br in &state.status.branches {
                let name = br.name.clone();
                let is_rem = br.is_remote;
                let prefix = if br.is_current {
                    "✓ "
                } else if br.is_remote {
                    "☁ "
                } else {
                    "⎇ "
                };
                raw.push(QuickPickItem {
                    label: format!("{}{}", prefix, name),
                    detail: format!("{} · {}", br.commit_short, br.subject),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Checking out branch...", move |repo| {
                            ops::checkout_branch(repo, &name, is_rem)
                                .map(|l| (format!("Switched to '{}'", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }

            for tag in &state.status.tags {
                let tname = tag.name.clone();
                raw.push(QuickPickItem {
                    label: format!("🏷 {}", tname),
                    detail: format!("tag · {}", tag.commit_short),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Checking out tag...", move |repo| {
                            ops::checkout_branch(repo, &tname, false)
                                .map(|l| (format!("Checked out tag '{}'", tname), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::DeleteBranch { force } => {
            let f = *force;
            for br in state
                .status
                .branches
                .iter()
                .filter(|b| !b.is_remote && !b.is_current)
            {
                let name = br.name.clone();
                raw.push(QuickPickItem {
                    label: format!("🗑 {}", name),
                    detail: br.commit_short.clone(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Deleting branch...", move |repo| {
                            ops::delete_branch(repo, &name, f)
                                .map(|l| (format!("Branch '{}' eliminato", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::MergeBranch => {
            for br in state.status.branches.iter().filter(|b| !b.is_current) {
                let name = br.name.clone();
                raw.push(QuickPickItem {
                    label: format!("⎇ Merge '{}' into '{}'", name, state.status.head_name),
                    detail: br.commit_short.clone(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Merging branch...", move |repo| {
                            ops::merge_branch(repo, &name)
                                .map(|l| (format!("Merge di '{}' completato", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::RebaseBranch => {
            for br in state.status.branches.iter().filter(|b| !b.is_current) {
                let name = br.name.clone();
                raw.push(QuickPickItem {
                    label: format!("⎇ Rebase '{}' onto '{}'", state.status.head_name, name),
                    detail: br.commit_short.clone(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Rebasing branch...", move |repo| {
                            ops::rebase_branch(repo, &name)
                                .map(|l| (format!("Rebase su '{}' completato", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::StashPop | QuickPickKind::StashApply | QuickPickKind::StashDrop => {
            let op_verb = match kind {
                QuickPickKind::StashPop => "pop",
                QuickPickKind::StashApply => "apply",
                _ => "drop",
            };
            for st_entry in &state.status.stashes {
                let r = st_entry.reference.clone();
                let verb = op_verb.to_string();
                raw.push(QuickPickItem {
                    label: format!("{}: {}", st_entry.reference, st_entry.message),
                    detail: st_entry.relative_time.clone(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Running stash command...", move |repo| {
                            ops::stash_action(repo, &verb, &r)
                                .map(|l| (format!("Stash {} ({}) completato", verb, r), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::DeleteTag => {
            for tag in &state.status.tags {
                let tname = tag.name.clone();
                raw.push(QuickPickItem {
                    label: format!("🗑 Delete tag '{}'", tname),
                    detail: tag.commit_short.clone(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Deleting tag...", move |repo| {
                            ops::delete_tag(repo, &tname)
                                .map(|l| (format!("Tag '{}' eliminato", tname), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::ConfirmDiscardAll => {
            raw.push(QuickPickItem {
                label: "↶ Yes, Discard All Changes (git checkout -- . && git clean -fd)".into(),
                detail: "Destructive".into(),
                action: Box::new(|st| st.discard_all_confirmed()),
            });
            raw.push(QuickPickItem {
                label: "Cancel".into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::ConfirmDiscardFile { path, kind } => {
            let p = path.clone();
            let k = *kind;
            raw.push(QuickPickItem {
                label: format!("↶ Yes, Discard Changes in '{}'", p),
                detail: "Destructive".into(),
                action: Box::new(move |st| st.discard_file_confirmed(p, k)),
            });
            raw.push(QuickPickItem {
                label: "Cancel".into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::CommandPalette => {
            raw.push(QuickPickItem {
                label: "Git: Checkout to...".into(),
                detail: "Switch branch or tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Create Branch...".into(),
                detail: "git checkout -b".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                        from_ref: None,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Commit".into(),
                detail: "Ctrl+Enter".into(),
                action: Box::new(|st| st.execute_commit(CommitVariant::Normal)),
            });
            raw.push(QuickPickItem {
                label: "Git: Commit (Amend)".into(),
                detail: "git commit --amend".into(),
                action: Box::new(|st| {
                    st.commit_amend = true;
                    st.execute_commit(CommitVariant::Amend);
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Sync".into(),
                detail: "Pull & Push".into(),
                action: Box::new(|st| {
                    let has_up = st.status.upstream.is_some();
                    let br = st.status.head_name.clone();
                    st.spawn_git_task("Syncing...", move |repo| {
                        ops::sync_repo(repo, has_up, &br)
                            .map(|logs| ("Sync completato".into(), logs))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Pull".into(),
                detail: "git pull".into(),
                action: Box::new(|st| {
                    st.spawn_git_task("Pulling...", move |repo| {
                        ops::pull(repo, false)
                            .map(|l| ("Pull completato".into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Push".into(),
                detail: "git push".into(),
                action: Box::new(|st| {
                    let has_up = st.status.upstream.is_some();
                    let br = st.status.head_name.clone();
                    st.spawn_git_task("Pushing...", move |repo| {
                        let up = if has_up { None } else { Some(br.as_str()) };
                        ops::push(repo, up, false)
                            .map(|l| ("Push completato".into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Fetch All (Prune)".into(),
                detail: "git fetch --all --prune".into(),
                action: Box::new(|st| {
                    st.spawn_git_task("Fetching...", move |repo| {
                        ops::fetch_all(repo)
                            .map(|l| ("Fetch completato".into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Stage All Changes".into(),
                detail: "git add -A".into(),
                action: Box::new(|st| st.stage_all()),
            });
            raw.push(QuickPickItem {
                label: "Git: Unstage All Changes".into(),
                detail: "git reset HEAD".into(),
                action: Box::new(|st| st.unstage_all()),
            });
            raw.push(QuickPickItem {
                label: "Git: Stash Changes...".into(),
                detail: "git stash push".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPush {
                        include_untracked: false,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Pop Stash...".into(),
                detail: "git stash pop".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPop));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Merge Branch...".into(),
                detail: "git merge".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::MergeBranch));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Rebase Branch...".into(),
                detail: "git rebase".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::RebaseBranch));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Delete Branch...".into(),
                detail: "git branch -d".into(),
                action: Box::new(|st| {
                    st.quick_pick =
                        Some(QuickPickState::new(QuickPickKind::DeleteBranch { force: false }));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Create Tag...".into(),
                detail: "git tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateTag));
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Undo Last Commit".into(),
                detail: "git reset --soft HEAD~1".into(),
                action: Box::new(|st| {
                    st.spawn_git_task("Undoing last commit...", move |repo| {
                        ops::undo_last_commit(repo)
                            .map(|l| ("Ultimo commit annullato".into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                label: "Git: Show Git Output".into(),
                detail: "Toggle Output Panel".into(),
                action: Box::new(|st| {
                    st.settings.show_output_panel = !st.settings.show_output_panel;
                    st.settings.save();
                }),
            });
        }
        _ => {}
    }

    if q.is_empty() {
        raw
    } else {
        raw.into_iter()
            .filter(|it| {
                it.label.to_ascii_lowercase().contains(&q)
                    || it.detail.to_ascii_lowercase().contains(&q)
            })
            .collect()
    }
}
