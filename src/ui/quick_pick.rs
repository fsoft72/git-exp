use crate::git::ops::{self, CommitVariant};
use crate::state::{AppState, QuickPickKind, QuickPickState};
use crate::theme::{Codicon, VsCodeTheme, codicon_button, draw_inline_codicon};
use egui::{Color32, CornerRadius, Key, Order, Pos2, RichText, Sense, Stroke, Vec2};

struct QuickPickItem {
    icon: Option<Codicon>,
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
        QuickPickKind::CommandPalette => "Git Command Palette".to_string(),
        QuickPickKind::Checkout => "Git: Checkout to... (Select a ref or type a new branch name)".to_string(),
        QuickPickKind::CreateBranch { from_ref: None } => format!(
            "Git: Create Branch from '{}' (Enter new branch name and press Enter)",
            state.status.head_name
        ),
        QuickPickKind::CreateBranch {
            from_ref: Some(r),
        } => format!("Git: Create Branch from '{}' (Enter branch name and press Enter)", r),
        QuickPickKind::CreateBranchFrom { preset_name: None } => {
            "Git: Create Branch From... (Select base branch or tag)".to_string()
        }
        QuickPickKind::CreateBranchFrom {
            preset_name: Some(name),
        } => format!("Git: Create Branch '{}' From... (Select base branch or tag)", name),
        QuickPickKind::DeleteBranch { .. } => {
            "Git: Delete Branch (Select a branch to delete)".to_string()
        }
        QuickPickKind::RenameBranch => {
            "Git: Rename Current Branch (Enter new name)".to_string()
        }
        QuickPickKind::MergeBranch => "Git: Merge Branch into current branch".to_string(),
        QuickPickKind::RebaseBranch => "Git: Rebase Current Branch onto...".to_string(),
        QuickPickKind::StashPush {
            include_untracked: false,
        } => "Git: Stash Changes (Optional message, press Enter to stash)".to_string(),
        QuickPickKind::StashPush {
            include_untracked: true,
        } => "Git: Stash Including Untracked (Optional message, press Enter)".to_string(),
        QuickPickKind::StashPop => "Git: Pop Stash (Select a stash to pop)".to_string(),
        QuickPickKind::StashApply => "Git: Apply Stash (Select a stash to apply)".to_string(),
        QuickPickKind::StashDrop => "Git: Drop Stash (Select a stash to delete)".to_string(),
        QuickPickKind::CreateTag => {
            "Git: Create Tag at HEAD (Enter tag name and press Enter)".to_string()
        }
        QuickPickKind::DeleteTag => "Git: Delete Tag (Select a tag to delete)".to_string(),
        QuickPickKind::ConfirmDiscardAll => {
            "Discard ALL working tree & untracked changes? (Cannot be undone)".to_string()
        }
        QuickPickKind::ConfirmDiscardFile { path, .. } => {
            format!("Discard changes in '{}'?", path)
        }
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&title)
                .size(11.5)
                .strong()
                .color(VsCodeTheme::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if codicon_button(ui, Codicon::Close, "Close (Esc)") {
                state.quick_pick = None;
            }
        });
    });
    ui.add_space(4.0);

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
                QuickPickKind::CreateBranchFrom { .. } => "Select base branch or tag...",
                QuickPickKind::RenameBranch => "New branch name...",
                QuickPickKind::StashPush { .. } => "Optional stash message (press Enter)...",
                QuickPickKind::CreateTag => "Tag name (e.g. v0.2.0)...",
                _ => "Type to filter commands or branches...",
            };
            let input_id = ui.make_persistent_id((
                "vscode_quick_pick_input",
                std::mem::discriminant(&qp.kind),
            ));
            if qp.just_opened {
                ui.memory_mut(|m| m.request_focus(input_id));
                ui.ctx().request_repaint();
            }
            let te = egui::TextEdit::singleline(&mut query)
                .id(input_id)
                .hint_text(RichText::new(hint).color(VsCodeTheme::TEXT_MUTED))
                .desired_width(f32::INFINITY)
                .frame(false);
            let resp = ui.add(te);
            if qp.just_opened {
                resp.request_focus();
            }
            let enter_pressed = ui.input(|i| {
                i.key_pressed(Key::Enter)
                    || i.events.iter().any(|ev| {
                        matches!(
                            ev,
                            egui::Event::Key {
                                key: Key::Enter,
                                pressed: true,
                                ..
                            }
                        )
                    })
            });
            if !qp.just_opened && (enter_pressed || (resp.lost_focus() && enter_pressed)) {
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

    match &qp.kind {
        QuickPickKind::CreateBranch { .. }
        | QuickPickKind::RenameBranch
        | QuickPickKind::CreateTag
            if query.trim().is_empty() =>
        {
            ui.add_space(6.0);
            let help_msg = match &qp.kind {
                QuickPickKind::CreateBranch { .. } => {
                    "Type the new branch name above and press Enter to create and checkout."
                }
                QuickPickKind::RenameBranch => {
                    "Type the new branch name above and press Enter to rename."
                }
                QuickPickKind::CreateTag => {
                    "Type the tag name above and press Enter to create the tag."
                }
                _ => "",
            };
            ui.label(
                RichText::new(help_msg)
                    .size(11.5)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            return;
        }
        _ => {}
    }

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
                if let Some(ic) = item.icon {
                    draw_inline_codicon(
                        &mut row_ui,
                        ic,
                        15.0,
                        if is_sel {
                            VsCodeTheme::TEXT_BRIGHT
                        } else {
                            VsCodeTheme::TEXT_PRIMARY
                        },
                    );
                }
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
        ui.ctx().request_repaint();
    }
}

fn build_quick_pick_items(
    state: &AppState,
    kind: &QuickPickKind,
    query: &str,
) -> Vec<QuickPickItem> {
    let trimmed = query.trim();
    let q = trimmed.to_ascii_lowercase();
    let mut raw: Vec<QuickPickItem> = Vec::new();

    match kind {
        QuickPickKind::CreateBranch { from_ref } => {
            if !trimmed.is_empty() {
                let name = trimmed.to_string();
                let base = from_ref.clone();
                let base_label = base
                    .as_deref()
                    .unwrap_or(&state.status.head_name)
                    .to_string();
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: format!("Create and checkout branch '{}'", name),
                    detail: format!("from '{}'", base_label),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Creating branch...", move |repo| {
                            ops::create_branch(repo, &name, base.as_deref(), true)
                                .map(|l| (format!("Creato e attivato branch '{}'", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
            return raw;
        }
        QuickPickKind::RenameBranch => {
            if !trimmed.is_empty() {
                let name = trimmed.to_string();
                raw.push(QuickPickItem {
                    icon: Some(Codicon::GitBranch),
                    label: format!(
                        "Rename branch '{}' to '{}'",
                        state.status.head_name, name
                    ),
                    detail: "git branch -m".to_string(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Renaming branch...", move |repo| {
                            ops::rename_branch(repo, &name)
                                .map(|l| (format!("Branch rinominato in '{}'", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
            return raw;
        }
        QuickPickKind::StashPush { include_untracked } => {
            let inc = *include_untracked;
            let msg_opt = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            let label = match &msg_opt {
                Some(m) => format!("Stash changes with message '{}'", m),
                None => "Stash changes (default message)".to_string(),
            };
            raw.push(QuickPickItem {
                icon: Some(Codicon::Archive),
                label,
                detail: if inc {
                    "git stash push -u".to_string()
                } else {
                    "git stash push".to_string()
                },
                action: Box::new(move |st| {
                    st.spawn_git_task("Stashing changes...", move |repo| {
                        ops::stash_push(repo, msg_opt.as_deref(), inc)
                            .map(|l| ("Modifiche salvate nello Stash".into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            return raw;
        }
        QuickPickKind::CreateTag => {
            if !trimmed.is_empty() {
                let tag_name = trimmed.to_string();
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Tag),
                    label: format!("Create tag '{}' at HEAD", tag_name),
                    detail: "git tag".to_string(),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Creating tag...", move |repo| {
                            ops::create_tag(repo, &tag_name, None)
                                .map(|l| (format!("Tag '{}' creato", tag_name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
            return raw;
        }
        QuickPickKind::CreateBranchFrom { preset_name } => {
            for br in &state.status.branches {
                let base_ref = br.name.clone();
                let preset = preset_name.clone();
                let ic = if br.is_current {
                    Codicon::Check
                } else if br.is_remote {
                    Codicon::CloudUpload
                } else {
                    Codicon::GitBranch
                };
                raw.push(QuickPickItem {
                    icon: Some(ic),
                    label: base_ref.clone(),
                    detail: format!("{} - {}", br.commit_short, br.subject),
                    action: Box::new(move |st| {
                        if let Some(new_name) = preset {
                            st.spawn_git_task("Creating branch...", move |repo| {
                                ops::create_branch(repo, &new_name, Some(&base_ref), true)
                                    .map(|l| {
                                        (
                                            format!(
                                                "Creato e attivato branch '{}' da '{}'",
                                                new_name, base_ref
                                            ),
                                            vec![l],
                                        )
                                    })
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        } else {
                            st.quick_pick =
                                Some(QuickPickState::new(QuickPickKind::CreateBranch {
                                    from_ref: Some(base_ref),
                                }));
                        }
                    }),
                });
            }
            for tag in &state.status.tags {
                let base_ref = tag.name.clone();
                let preset = preset_name.clone();
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Tag),
                    label: base_ref.clone(),
                    detail: format!("tag - {}", tag.commit_short),
                    action: Box::new(move |st| {
                        if let Some(new_name) = preset {
                            st.spawn_git_task("Creating branch...", move |repo| {
                                ops::create_branch(repo, &new_name, Some(&base_ref), true)
                                    .map(|l| {
                                        (
                                            format!(
                                                "Creato e attivato branch '{}' da '{}'",
                                                new_name, base_ref
                                            ),
                                            vec![l],
                                        )
                                    })
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        } else {
                            st.quick_pick =
                                Some(QuickPickState::new(QuickPickKind::CreateBranch {
                                    from_ref: Some(base_ref),
                                }));
                        }
                    }),
                });
            }
        }
        QuickPickKind::Checkout => {
            if trimmed.is_empty() {
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: "Create new branch...".to_string(),
                    detail: format!("from '{}'", state.status.head_name),
                    action: Box::new(|st| {
                        st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                            from_ref: None,
                        }));
                    }),
                });
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: "Create new branch from...".to_string(),
                    detail: "Select base branch or tag".to_string(),
                    action: Box::new(|st| {
                        st.quick_pick =
                            Some(QuickPickState::new(QuickPickKind::CreateBranchFrom {
                                preset_name: None,
                            }));
                    }),
                });
            }

            let mut ref_items: Vec<QuickPickItem> = Vec::new();
            for br in &state.status.branches {
                let name = br.name.clone();
                let is_rem = br.is_remote;
                let ic = if br.is_current {
                    Codicon::Check
                } else if br.is_remote {
                    Codicon::CloudUpload
                } else {
                    Codicon::GitBranch
                };
                ref_items.push(QuickPickItem {
                    icon: Some(ic),
                    label: name.clone(),
                    detail: format!("{} - {}", br.commit_short, br.subject),
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
                ref_items.push(QuickPickItem {
                    icon: Some(Codicon::Tag),
                    label: tname.clone(),
                    detail: format!("tag - {}", tag.commit_short),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Checking out tag...", move |repo| {
                            ops::checkout_branch(repo, &tname, false)
                                .map(|l| (format!("Checked out tag '{}'", tname), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }

            if trimmed.is_empty() {
                raw.extend(ref_items);
                return raw;
            } else {
                let exact_exists = state
                    .status
                    .branches
                    .iter()
                    .any(|b| b.name.eq_ignore_ascii_case(trimmed))
                    || state
                        .status
                        .tags
                        .iter()
                        .any(|t| t.name.eq_ignore_ascii_case(trimmed));

                let filtered_refs: Vec<QuickPickItem> = ref_items
                    .into_iter()
                    .filter(|it| it.label.to_ascii_lowercase().contains(&q))
                    .collect();

                let new_br_name = trimmed.to_string();
                let new_br_name_from = trimmed.to_string();
                let head_name = state.status.head_name.clone();

                let create_direct_item = QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: format!("Create new branch '{}'", new_br_name),
                    detail: format!("from '{}' (git checkout -b)", head_name),
                    action: Box::new(move |st| {
                        st.spawn_git_task("Creating branch...", move |repo| {
                            ops::create_branch(repo, &new_br_name, None, true)
                                .map(|l| {
                                    (format!("Creato e attivato branch '{}'", new_br_name), vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                };

                let create_from_item = QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: format!("Create new branch '{}' from...", new_br_name_from),
                    detail: "Select base branch or tag".to_string(),
                    action: Box::new(move |st| {
                        st.quick_pick =
                            Some(QuickPickState::new(QuickPickKind::CreateBranchFrom {
                                preset_name: Some(new_br_name_from),
                            }));
                    }),
                };

                if exact_exists {
                    raw.extend(filtered_refs);
                    raw.push(create_direct_item);
                    raw.push(create_from_item);
                } else {
                    raw.push(create_direct_item);
                    raw.push(create_from_item);
                    raw.extend(filtered_refs);
                }
                return raw;
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
                    icon: Some(Codicon::Trash),
                    label: name.clone(),
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
                    icon: Some(Codicon::GitBranch),
                    label: format!("Merge '{}' into '{}'", name, state.status.head_name),
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
                    icon: Some(Codicon::GitBranch),
                    label: format!("Rebase '{}' onto '{}'", state.status.head_name, name),
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
                    icon: Some(Codicon::Archive),
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
                    icon: Some(Codicon::Trash),
                    label: format!("Delete tag '{}'", tname),
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
                icon: Some(Codicon::Discard),
                label: "Yes, Discard All Changes (git checkout -- . && git clean -fd)".into(),
                detail: "Destructive".into(),
                action: Box::new(|st| st.discard_all_confirmed()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Close),
                label: "Cancel".into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::ConfirmDiscardFile { path, kind } => {
            let p = path.clone();
            let k = *kind;
            raw.push(QuickPickItem {
                icon: Some(Codicon::Discard),
                label: format!("Yes, Discard Changes in '{}'", p),
                detail: "Destructive".into(),
                action: Box::new(move |st| st.discard_file_confirmed(p, k)),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Close),
                label: "Cancel".into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::CommandPalette => {
            raw.push(QuickPickItem {
                icon: Some(Codicon::GitBranch),
                label: "Git: Checkout to...".into(),
                detail: "Switch branch or tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Plus),
                label: "Git: Create Branch...".into(),
                detail: "git checkout -b".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                        from_ref: None,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Plus),
                label: "Git: Create Branch From...".into(),
                detail: "Select base branch/tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranchFrom {
                        preset_name: None,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Check),
                label: "Git: Commit".into(),
                detail: "Ctrl+Enter".into(),
                action: Box::new(|st| st.execute_commit(CommitVariant::Normal)),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Check),
                label: "Git: Commit (Amend)".into(),
                detail: "git commit --amend".into(),
                action: Box::new(|st| {
                    st.commit_amend = true;
                    st.execute_commit(CommitVariant::Amend);
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Sync),
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
                icon: Some(Codicon::ArrowDown),
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
                icon: Some(Codicon::ArrowUp),
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
                icon: Some(Codicon::Refresh),
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
                icon: Some(Codicon::Plus),
                label: "Git: Stage All Changes".into(),
                detail: "git add -A".into(),
                action: Box::new(|st| st.stage_all()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Minus),
                label: "Git: Unstage All Changes".into(),
                detail: "git reset HEAD".into(),
                action: Box::new(|st| st.unstage_all()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Archive),
                label: "Git: Stash Changes...".into(),
                detail: "git stash push".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPush {
                        include_untracked: false,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Archive),
                label: "Git: Pop Stash...".into(),
                detail: "git stash pop".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPop));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::GitBranch),
                label: "Git: Merge Branch...".into(),
                detail: "git merge".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::MergeBranch));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::GitBranch),
                label: "Git: Rebase Branch...".into(),
                detail: "git rebase".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::RebaseBranch));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Trash),
                label: "Git: Delete Branch...".into(),
                detail: "git branch -d".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::DeleteBranch {
                        force: false,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Tag),
                label: "Git: Create Tag...".into(),
                detail: "git tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateTag));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Discard),
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
                icon: Some(Codicon::FileCode),
                label: "Git: Show Git Output".into(),
                detail: "Toggle Output Panel".into(),
                action: Box::new(|st| {
                    st.settings.show_output_panel = !st.settings.show_output_panel;
                    st.settings.save();
                }),
            });
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::status::BranchInfo;

    #[test]
    fn test_checkout_typing_new_branch_keeps_create_action_first() {
        let mut state = AppState::new(None);
        state.status.head_name = "dev".to_string();
        state.status.branches = vec![BranchInfo {
            name: "dev".to_string(),
            is_remote: false,
            is_current: true,
            commit_short: "abc1234".to_string(),
            upstream: None,
            subject: "Initial dev commit".to_string(),
        }];

        let items = build_quick_pick_items(&state, &QuickPickKind::Checkout, "feature/login");
        assert!(
            !items.is_empty(),
            "Typing a new branch name in Checkout must not result in an empty list"
        );
        assert_eq!(items[0].label, "Create new branch 'feature/login'");
        assert!(items[0].detail.contains("from 'dev'"));
        assert_eq!(items[1].label, "Create new branch 'feature/login' from...");
    }

    #[test]
    fn test_create_branch_kind_returns_action_item() {
        let mut state = AppState::new(None);
        state.status.head_name = "dev".to_string();

        let items = build_quick_pick_items(
            &state,
            &QuickPickKind::CreateBranch {
                from_ref: Some("dev".to_string()),
            },
            "feat/nuovo",
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Create and checkout branch 'feat/nuovo'");
        assert_eq!(items[0].detail, "from 'dev'");
    }
}
