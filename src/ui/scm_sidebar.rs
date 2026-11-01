use crate::git::ops::{self, CommitVariant};
use crate::git::status::{ChangeGroup, FileChange, FileStatusKind};
use crate::settings::ScmFileViewMode;
use crate::state::{ActiveSidebarPanel, AppState, QuickPickKind, QuickPickState};
use crate::theme::{
    Codicon, VsCodeTheme, codicon_button, codicon_text_button, draw_count_badge,
    draw_inline_codicon, paint_codicon,
};
use crate::ui::graph_view::render_graph_list;
use egui::{Color32, CornerRadius, FontFamily, FontId, RichText, Sense, Stroke, Vec2};
use std::collections::BTreeMap;

pub fn render_sidebar(ctx: &egui::Context, state: &mut AppState) {
    egui::SidePanel::left("vscode_scm_sidebar")
        .default_width(320.0)
        .width_range(220.0..=620.0)
        .resizable(true)
        .frame(
            egui::Frame::NONE
                .fill(VsCodeTheme::SIDEBAR_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER)),
        )
        .show(ctx, |ui| match state.active_sidebar {
            ActiveSidebarPanel::SourceControl => render_source_control_panel(ui, state),
            ActiveSidebarPanel::GraphFull => render_full_graph_sidebar(ui, state),
            ActiveSidebarPanel::Repositories => render_repositories_sidebar(ui, ctx, state),
            ActiveSidebarPanel::Settings => render_settings_sidebar(ui, state),
        });
}

fn render_source_control_panel(ui: &mut egui::Ui, state: &mut AppState) {
    // 1. Top Header "SOURCE CONTROL" + Toolbar icone vettoriali Codicon
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new("SOURCE CONTROL")
                .size(11.0)
                .strong()
                .color(VsCodeTheme::TEXT_PRIMARY),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);

            // Menu `...` (More Actions) con icona vettoriale Codicon::MoreHorizontal
            let more_id = ui.make_persistent_id("scm_more_actions_popup");
            let (more_rect, more_resp) =
                ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::click());
            if ui.is_rect_visible(more_rect) {
                if more_resp.hovered() {
                    ui.painter().rect_filled(
                        more_rect,
                        CornerRadius::same(3),
                        VsCodeTheme::LIST_HOVER_BG,
                    );
                }
                paint_codicon(
                    ui.painter(),
                    more_rect,
                    Codicon::MoreHorizontal,
                    if more_resp.hovered() {
                        VsCodeTheme::TEXT_BRIGHT
                    } else {
                        VsCodeTheme::TEXT_PRIMARY
                    },
                );
            }
            let more_resp = more_resp.on_hover_text("Views and More Actions...");
            if more_resp.clicked() {
                ui.memory_mut(|m| m.toggle_popup(more_id));
            }
            egui::popup::popup_below_widget(
                ui,
                more_id,
                &more_resp,
                egui::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    render_more_actions_menu(ui, state);
                },
            );

            // Refresh button
            if codicon_button(ui, Codicon::Refresh, "Refresh (Git Status)") {
                state.trigger_refresh();
            }

            // Commit shortcut button
            if codicon_button(ui, Codicon::Check, "Commit (Ctrl+Enter)") {
                let variant = state.commit_action_variant;
                state.execute_commit(variant);
            }

            // Toggle List / Tree View
            let (mode_icon, mode_tip) = match state.settings.file_view_mode {
                ScmFileViewMode::List => (Codicon::ListTree, "View as Tree"),
                ScmFileViewMode::Tree => (Codicon::ListFlat, "View as List"),
            };
            if codicon_button(ui, mode_icon, mode_tip) {
                state.settings.file_view_mode = match state.settings.file_view_mode {
                    ScmFileViewMode::List => ScmFileViewMode::Tree,
                    ScmFileViewMode::Tree => ScmFileViewMode::List,
                };
                state.settings.save();
            }
        });
    });
    ui.add_space(4.0);

    if state.repo_path.is_none() {
        render_no_repo_welcome(ui, state);
        return;
    }

    // 2. Commit Input Box + Bottone Commit / Sync
    render_commit_box_area(ui, state);

    ui.add_space(4.0);
    ui.separator();

    // 3. Area scrollabile con MERGE CHANGES, STAGED CHANGES, CHANGES e SOURCE CONTROL GRAPH
    egui::ScrollArea::vertical()
        .id_salt("scm_sidebar_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Merge Conflicts (se presenti)
            if !state.status.merge_changes.is_empty() {
                let merge_list = state.status.merge_changes.clone();
                render_change_group_section(
                    ui,
                    state,
                    "MERGE CHANGES",
                    ChangeGroup::Merge,
                    &merge_list,
                );
            }

            // Staged Changes
            if !state.status.staged_changes.is_empty() {
                let staged_list = state.status.staged_changes.clone();
                render_change_group_section(
                    ui,
                    state,
                    "STAGED CHANGES",
                    ChangeGroup::Staged,
                    &staged_list,
                );
            }

            // Working Tree Changes
            let working_list = state.status.working_changes.clone();
            render_change_group_section(
                ui,
                state,
                "CHANGES",
                ChangeGroup::WorkingTree,
                &working_list,
            );

            ui.add_space(6.0);

            // Sezione collassabile SOURCE CONTROL GRAPH
            render_collapsible_graph_section(ui, state);
        });
}

fn render_commit_box_area(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let avail_w = (ui.available_width() - 8.0).max(120.0);
        ui.vertical(|ui| {
            ui.set_width(avail_w);

            // Calcolo lunghezza prima riga (VS Code avvisa a >50 caratteri)
            let first_line_len = state
                .commit_message
                .lines()
                .next()
                .map(|l| l.chars().count())
                .unwrap_or(0);
            let remaining_chars = 50isize - (first_line_len as isize);

            let border_color = if first_line_len > 72 {
                VsCodeTheme::ERROR_RED
            } else if first_line_len > 50 {
                VsCodeTheme::WARNING_YELLOW
            } else {
                VsCodeTheme::INPUT_BORDER
            };

            egui::Frame::NONE
                .fill(VsCodeTheme::INPUT_BG)
                .stroke(Stroke::new(1.0_f32, border_color))
                .corner_radius(CornerRadius::same(3))
                .inner_margin(egui::Margin::symmetric(6, 5))
                .show(ui, |ui| {
                    let hint = format!(
                        "Message (Ctrl+Enter to commit on '{}')",
                        state.status.head_name
                    );
                    let te = egui::TextEdit::multiline(&mut state.commit_message)
                        .hint_text(
                            RichText::new(hint)
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_MUTED),
                        )
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .frame(false)
                        .font(FontId::new(12.5, FontFamily::Proportional));

                    let resp = ui.add(te);

                    // Scorciatoia Ctrl+Enter dentro il Commit Box
                    if resp.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command)
                    {
                        let variant = state.commit_action_variant;
                        state.execute_commit(variant);
                    }

                    // Barra inferiore dentro l'input box: checkbox Amend + contatore caratteri prima riga
                    ui.horizontal(|ui| {
                        let mut amend = state.commit_amend;
                        if ui
                            .checkbox(
                                &mut amend,
                                RichText::new("Amend")
                                    .size(11.0)
                                    .color(VsCodeTheme::TEXT_MUTED),
                            )
                            .on_hover_text("Amend previous commit (git commit --amend)")
                            .changed()
                        {
                            state.commit_amend = amend;
                            if amend
                                && state.commit_message.trim().is_empty()
                                && !state.status.head_commit_message.is_empty()
                            {
                                state.commit_message = state.status.head_commit_message.clone();
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let counter_color = if remaining_chars < -22 {
                                VsCodeTheme::ERROR_RED
                            } else if remaining_chars < 0 {
                                VsCodeTheme::WARNING_YELLOW
                            } else {
                                VsCodeTheme::TEXT_MUTED
                            };
                            ui.label(
                                RichText::new(format!("{}", remaining_chars))
                                    .size(11.0)
                                    .color(counter_color),
                            )
                            .on_hover_text(
                                "Remaining characters on subject line (50 characters recommended)",
                            );
                        });
                    });
                });

            ui.add_space(6.0);

            // Split Button blu primario: Commit (con icona Check vettoriale) + Dropdown ChevronDown vettoriale
            let dropdown_w = 26.0;
            let main_btn_w = (avail_w - dropdown_w - 2.0).max(80.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;

                let btn_label = match state.commit_action_variant {
                    CommitVariant::Normal if state.commit_amend => "Commit (Amend)",
                    CommitVariant::Normal => "Commit",
                    CommitVariant::Amend => "Commit (Amend)",
                    CommitVariant::SignOff => "Commit (Signed Off)",
                    CommitVariant::CommitAndPush => "Commit & Push",
                    CommitVariant::CommitAndSync => "Commit & Sync",
                };

                if codicon_text_button(
                    ui,
                    Codicon::Check,
                    btn_label,
                    VsCodeTheme::ACCENT_BLUE,
                    Color32::WHITE,
                    Vec2::new(main_btn_w, 26.0),
                    CornerRadius {
                        nw: 3,
                        sw: 3,
                        ne: 0,
                        se: 0,
                    },
                )
                .clicked()
                {
                    let variant = state.commit_action_variant;
                    state.execute_commit(variant);
                }

                // Bottone freccia dropdown con ChevronDown disegnato via Painter
                let dd_id = ui.make_persistent_id("commit_split_dropdown_popup");
                let (dd_rect, dd_resp) =
                    ui.allocate_exact_size(Vec2::new(dropdown_w, 26.0), Sense::click());
                if ui.is_rect_visible(dd_rect) {
                    let bg = if dd_resp.hovered() {
                        VsCodeTheme::ACCENT_BLUE_HOVER
                    } else {
                        VsCodeTheme::ACCENT_BLUE
                    };
                    ui.painter().rect_filled(
                        dd_rect,
                        CornerRadius {
                            nw: 0,
                            sw: 0,
                            ne: 3,
                            se: 3,
                        },
                        bg,
                    );
                    paint_codicon(
                        ui.painter(),
                        dd_rect,
                        Codicon::ChevronDown,
                        Color32::WHITE,
                    );
                }
                let dd_resp = dd_resp.on_hover_text("More Commit Actions...");
                if dd_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(dd_id));
                }
                egui::popup::popup_below_widget(
                    ui,
                    dd_id,
                    &dd_resp,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(185.0);
                        if ui.button("Commit").clicked() {
                            state.commit_action_variant = CommitVariant::Normal;
                            state.commit_amend = false;
                            state.execute_commit(CommitVariant::Normal);
                        }
                        if ui.button("Commit (Amend)").clicked() {
                            state.commit_action_variant = CommitVariant::Amend;
                            state.commit_amend = true;
                            state.execute_commit(CommitVariant::Amend);
                        }
                        if ui.button("Commit (Signed Off)").clicked() {
                            state.commit_action_variant = CommitVariant::SignOff;
                            state.execute_commit(CommitVariant::SignOff);
                        }
                        ui.separator();
                        if ui.button("Commit & Push").clicked() {
                            state.commit_action_variant = CommitVariant::CommitAndPush;
                            state.execute_commit(CommitVariant::CommitAndPush);
                        }
                        if ui.button("Commit & Sync").clicked() {
                            state.commit_action_variant = CommitVariant::CommitAndSync;
                            state.execute_commit(CommitVariant::CommitAndSync);
                        }
                    },
                );
            });

            // Se ci sono commit da sincronizzare o il branch non è pubblicato su remote,
            // mostra il pulsante "Sync Changes" / "Publish Branch" con icona vettoriale!
            if state.status.upstream.is_none() && !state.status.remotes.is_empty() {
                ui.add_space(4.0);
                if codicon_text_button(
                    ui,
                    Codicon::CloudUpload,
                    "Publish Branch",
                    VsCodeTheme::ACCENT_BLUE,
                    Color32::WHITE,
                    Vec2::new(avail_w, 25.0),
                    CornerRadius::same(3),
                )
                .on_hover_text("Publish current branch to origin")
                .clicked()
                {
                    let br = state.status.head_name.clone();
                    state.spawn_git_task("Publishing branch...", move |repo| {
                        ops::push(repo, Some(&br), false)
                            .map(|l| (format!("Branch '{}' pubblicato su origin", br), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }
            } else if state.status.ahead > 0 || state.status.behind > 0 {
                ui.add_space(4.0);
                let sync_text = format!(
                    "Sync Changes ({} down, {} up)",
                    state.status.behind, state.status.ahead
                );
                if codicon_text_button(
                    ui,
                    Codicon::Sync,
                    &sync_text,
                    VsCodeTheme::ACCENT_BLUE,
                    Color32::WHITE,
                    Vec2::new(avail_w, 25.0),
                    CornerRadius::same(3),
                )
                .on_hover_text("Pull and Push commits with upstream")
                .clicked()
                {
                    let has_up = state.status.upstream.is_some();
                    let br = state.status.head_name.clone();
                    state.spawn_git_task("Syncing changes...", move |repo| {
                        ops::sync_repo(repo, has_up, &br)
                            .map(|logs| ("Sync completato".to_string(), logs))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }
            }
        });
    });
}

fn render_change_group_section(
    ui: &mut egui::Ui,
    state: &mut AppState,
    title: &str,
    group: ChangeGroup,
    changes: &[FileChange],
) {
    let id = ui.make_persistent_id(format!("scm_group_{}", title));
    let mut collapsing =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);

    let header_height = 24.0;
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, header_height), Sense::click());
    let is_hovered = resp.hovered() || ui.rect_contains_pointer(rect);

    if is_hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::LIST_HOVER_BG);
    }

    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child_ui.add_space(4.0);
    let chevron = if collapsing.is_open() {
        Codicon::ChevronDown
    } else {
        Codicon::ChevronRight
    };
    draw_inline_codicon(&mut child_ui, chevron, 16.0, VsCodeTheme::TEXT_PRIMARY);
    child_ui.label(
        RichText::new(title)
            .size(11.0)
            .strong()
            .color(VsCodeTheme::TEXT_PRIMARY),
    );

    let mut action_triggered = false;
    child_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        draw_count_badge(ui, changes.len());

        if is_hovered && !changes.is_empty() {
            match group {
                ChangeGroup::Staged => {
                    if codicon_button(ui, Codicon::Minus, "Unstage All Changes") {
                        state.unstage_all();
                        action_triggered = true;
                    }
                }
                ChangeGroup::WorkingTree | ChangeGroup::Merge => {
                    if codicon_button(ui, Codicon::Plus, "Stage All Changes") {
                        state.stage_all();
                        action_triggered = true;
                    }
                    if codicon_button(ui, Codicon::Discard, "Discard All Changes") {
                        state.quick_pick =
                            Some(QuickPickState::new(QuickPickKind::ConfirmDiscardAll));
                        action_triggered = true;
                    }
                }
            }
        }
    });

    if resp.clicked() && !action_triggered {
        collapsing.toggle(ui);
    }
    collapsing.store(ui.ctx());

    if collapsing.is_open() {
        if changes.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                ui.label(
                    RichText::new("No changes")
                        .size(12.0)
                        .italics()
                        .color(VsCodeTheme::TEXT_MUTED),
                );
            });
        } else {
            match state.settings.file_view_mode {
                ScmFileViewMode::List => {
                    for change in changes {
                        render_file_change_row(ui, state, change, 18.0, true);
                    }
                }
                ScmFileViewMode::Tree => {
                    render_file_tree_group(ui, state, group, changes);
                }
            }
        }
    }
}

fn render_file_change_row(
    ui: &mut egui::Ui,
    state: &mut AppState,
    change: &FileChange,
    indent_px: f32,
    show_parent_dir: bool,
) {
    let row_h = 22.0;
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, row_h), Sense::click());

    let is_selected = state
        .selected_change_path
        .as_ref()
        .map(|(p, g)| p == &change.path && *g == change.group)
        .unwrap_or(false);
    let is_hovered = resp.hovered() || ui.rect_contains_pointer(rect);

    if is_selected {
        ui.painter().rect_filled(
            rect,
            CornerRadius::ZERO,
            VsCodeTheme::LIST_ACTIVE_SELECTION_BG,
        );
    } else if is_hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::LIST_HOVER_BG);
    }

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );

    row_ui.add_space(indent_px);

    // Badge estensione file (solo caratteri ASCII sicuri o Codicon FileCode)
    let (icon_txt, icon_col) = file_extension_badge(change.file_name());
    if icon_txt.is_empty() {
        draw_inline_codicon(&mut row_ui, Codicon::FileCode, 14.0, icon_col);
    } else {
        row_ui.label(
            RichText::new(icon_txt)
                .monospace()
                .size(10.0)
                .strong()
                .color(icon_col),
        );
    }

    // Nome del file (barrato se Deleted)
    let mut name_rt = RichText::new(change.file_name())
        .size(12.5)
        .color(if is_selected {
            VsCodeTheme::TEXT_BRIGHT
        } else {
            change.kind.color()
        });
    if change.kind == FileStatusKind::Deleted {
        name_rt = name_rt.strikethrough();
    }
    row_ui.label(name_rt);

    // Percorso cartella relativo attenuato (in List View)
    if show_parent_dir {
        let dir = change.parent_dir();
        if !dir.is_empty() {
            row_ui.label(
                RichText::new(dir)
                    .size(11.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        }
    }

    // Azioni Hover a destra + lettera di stato M / U / A / D / R
    let mut button_clicked = false;
    row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);

        // Lettera stato Git (`M`, `U`, `A`, `D`, `R`, `!`)
        ui.label(
            RichText::new(change.kind.badge_letter())
                .size(11.5)
                .strong()
                .color(change.kind.color()),
        )
        .on_hover_text(change.kind.label(change.group == ChangeGroup::Staged));

        if is_hovered {
            match change.group {
                ChangeGroup::Staged => {
                    if codicon_button(ui, Codicon::Minus, "Unstage Changes") {
                        state.unstage_file(change.path.clone());
                        button_clicked = true;
                    }
                }
                ChangeGroup::WorkingTree | ChangeGroup::Merge => {
                    if codicon_button(ui, Codicon::Plus, "Stage Changes") {
                        state.stage_file(change.path.clone());
                        button_clicked = true;
                    }
                    if codicon_button(ui, Codicon::Discard, "Discard Changes") {
                        state.quick_pick = Some(QuickPickState::new(
                            QuickPickKind::ConfirmDiscardFile {
                                path: change.path.clone(),
                                kind: change.kind,
                            },
                        ));
                        button_clicked = true;
                    }
                }
            }

            if change.kind != FileStatusKind::Deleted
                && codicon_button(ui, Codicon::OpenFile, "Open File")
            {
                state.open_file_in_editor(&change.path);
                button_clicked = true;
            }
        }
    });

    if resp.clicked() && !button_clicked {
        state.open_change_diff(change);
    }

    // Context menu al click destro sulla riga del file
    resp.context_menu(|ui| {
        ui.set_min_width(180.0);
        if ui.button("Open Changes (Diff)").clicked() {
            state.open_change_diff(change);
            ui.close_menu();
        }
        if change.kind != FileStatusKind::Deleted && ui.button("Open File (Editor)").clicked() {
            state.open_file_in_editor(&change.path);
            ui.close_menu();
        }
        if let Some(ref repo) = state.repo_path {
            let full_path = repo.join(&change.path);
            if full_path.exists() && ui.button("Open in System App").clicked() {
                let _ = open::that(full_path);
                ui.close_menu();
            }
        }
        ui.separator();
        match change.group {
            ChangeGroup::Staged => {
                if ui.button("- Unstage Changes").clicked() {
                    state.unstage_file(change.path.clone());
                    ui.close_menu();
                }
            }
            ChangeGroup::WorkingTree | ChangeGroup::Merge => {
                if ui.button("+ Stage Changes").clicked() {
                    state.stage_file(change.path.clone());
                    ui.close_menu();
                }
                if ui.button("Discard Changes...").clicked() {
                    state.quick_pick = Some(QuickPickState::new(
                        QuickPickKind::ConfirmDiscardFile {
                            path: change.path.clone(),
                            kind: change.kind,
                        },
                    ));
                    ui.close_menu();
                }
            }
        }
        ui.separator();
        if ui.button("Copy Relative Path").clicked() {
            ui.ctx().copy_text(change.path.clone());
            ui.close_menu();
        }
    });
}

#[derive(Default)]
struct FolderTreeNode {
    subfolders: BTreeMap<String, FolderTreeNode>,
    files: Vec<FileChange>,
}

impl FolderTreeNode {
    fn collect_all_paths(&self, out: &mut Vec<String>) {
        for f in &self.files {
            out.push(f.path.clone());
        }
        for sub in self.subfolders.values() {
            sub.collect_all_paths(out);
        }
    }
}

fn render_file_tree_group(
    ui: &mut egui::Ui,
    state: &mut AppState,
    group: ChangeGroup,
    changes: &[FileChange],
) {
    let mut root = FolderTreeNode::default();
    for ch in changes {
        let parts: Vec<&str> = ch.path.split('/').collect();
        let mut curr = &mut root;
        for folder in &parts[..parts.len().saturating_sub(1)] {
            curr = curr.subfolders.entry((*folder).to_string()).or_default();
        }
        curr.files.push(ch.clone());
    }

    render_tree_node_recursive(ui, state, group, &root, "", 12.0);
}

fn render_tree_node_recursive(
    ui: &mut egui::Ui,
    state: &mut AppState,
    group: ChangeGroup,
    node: &FolderTreeNode,
    prefix: &str,
    indent: f32,
) {
    for (folder_name, subnode) in &node.subfolders {
        let full_folder_key = if prefix.is_empty() {
            format!("{:?}:{}", group, folder_name)
        } else {
            format!("{}/{}", prefix, folder_name)
        };
        let is_collapsed = state.tree_collapsed_folders.contains(&full_folder_key);

        let row_h = 22.0;
        let avail_w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, row_h), Sense::click());
        let is_hovered = resp.hovered() || ui.rect_contains_pointer(rect);
        if is_hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::LIST_HOVER_BG);
        }

        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        row_ui.add_space(indent);
        let chevron = if is_collapsed {
            Codicon::ChevronRight
        } else {
            Codicon::ChevronDown
        };
        draw_inline_codicon(&mut row_ui, chevron, 15.0, VsCodeTheme::TEXT_PRIMARY);
        draw_inline_codicon(
            &mut row_ui,
            if is_collapsed {
                Codicon::Folder
            } else {
                Codicon::FolderOpened
            },
            15.0,
            Color32::from_rgb(0xDC, 0xB6, 0x7A),
        );
        row_ui.label(
            RichText::new(folder_name)
                .size(12.5)
                .color(VsCodeTheme::TEXT_PRIMARY),
        );

        let mut folder_action = false;
        if is_hovered {
            row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                let mut folder_paths = Vec::new();
                subnode.collect_all_paths(&mut folder_paths);
                match group {
                    ChangeGroup::Staged => {
                        if codicon_button(ui, Codicon::Minus, "Unstage Folder") {
                            state.unstage_paths_batch(folder_paths);
                            folder_action = true;
                        }
                    }
                    ChangeGroup::WorkingTree | ChangeGroup::Merge => {
                        if codicon_button(ui, Codicon::Plus, "Stage Folder") {
                            state.stage_paths_batch(folder_paths);
                            folder_action = true;
                        }
                    }
                }
            });
        }

        if resp.clicked() && !folder_action {
            if is_collapsed {
                state.tree_collapsed_folders.remove(&full_folder_key);
            } else {
                state.tree_collapsed_folders.insert(full_folder_key.clone());
            }
        }

        if !is_collapsed {
            render_tree_node_recursive(ui, state, group, subnode, &full_folder_key, indent + 14.0);
        }
    }

    for file in &node.files {
        render_file_change_row(ui, state, file, indent + 8.0, false);
    }
}

fn render_collapsible_graph_section(ui: &mut egui::Ui, state: &mut AppState) {
    let id = ui.make_persistent_id("scm_source_control_graph_section");
    let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        state.settings.show_graph_section,
    );

    let header_h = 24.0;
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, header_h), Sense::click());
    let hovered = resp.hovered() || ui.rect_contains_pointer(rect);
    if hovered {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::LIST_HOVER_BG);
    }

    let mut hdr_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    hdr_ui.add_space(4.0);
    let chevron = if collapsing.is_open() {
        Codicon::ChevronDown
    } else {
        Codicon::ChevronRight
    };
    draw_inline_codicon(&mut hdr_ui, chevron, 16.0, VsCodeTheme::TEXT_PRIMARY);
    hdr_ui.label(
        RichText::new("SOURCE CONTROL GRAPH")
            .size(11.0)
            .strong()
            .color(VsCodeTheme::TEXT_PRIMARY),
    );

    let mut btn_clicked = false;
    hdr_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        if state.status.ahead > 0 || state.status.behind > 0 {
            ui.label(
                RichText::new(format!(
                    "-{} +{}",
                    state.status.behind, state.status.ahead
                ))
                .size(11.0)
                .color(VsCodeTheme::ACCENT_BLUE),
            );
        }
        if hovered && codicon_button(ui, Codicon::Refresh, "Refresh Graph") {
            state.trigger_refresh();
            btn_clicked = true;
        }
    });

    if resp.clicked() && !btn_clicked {
        collapsing.toggle(ui);
        state.settings.show_graph_section = collapsing.is_open();
        state.settings.save();
    }
    collapsing.store(ui.ctx());

    if collapsing.is_open() {
        render_graph_list(ui, state, 28);
    }
}

fn render_full_graph_sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new("SOURCE CONTROL GRAPH")
                .size(11.0)
                .strong()
                .color(VsCodeTheme::TEXT_PRIMARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            if codicon_button(ui, Codicon::Refresh, "Refresh Graph") {
                state.trigger_refresh();
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("full_graph_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_graph_list(ui, state, 200);
        });
}

fn render_repositories_sidebar(ui: &mut egui::Ui, ctx: &egui::Context, state: &mut AppState) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new("SOURCE CONTROL REPOSITORIES")
                .size(11.0)
                .strong()
                .color(VsCodeTheme::TEXT_PRIMARY),
        );
    });
    ui.add_space(6.0);
    ui.separator();

    ui.vertical(|ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            let avail_w = (ui.available_width() - 10.0).max(120.0);
            if codicon_text_button(
                ui,
                Codicon::Folder,
                "Open Repository Folder...",
                VsCodeTheme::ACCENT_BLUE,
                Color32::WHITE,
                Vec2::new(avail_w, 26.0),
                CornerRadius::same(3),
            )
            .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    state.open_repository(folder, ctx);
                    state.active_sidebar = ActiveSidebarPanel::SourceControl;
                }
            }
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new("RECENT REPOSITORIES")
                    .size(11.0)
                    .strong()
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        ui.add_space(4.0);

        let recents = state.settings.recent_repos.clone();
        for repo_str in recents {
            let is_current = state
                .repo_path
                .as_ref()
                .map(|p| p.to_string_lossy() == repo_str)
                .unwrap_or(false);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                draw_inline_codicon(
                    ui,
                    Codicon::Folder,
                    14.0,
                    Color32::from_rgb(0xDC, 0xB6, 0x7A),
                );
                let resp = ui.selectable_label(
                    is_current,
                    RichText::new(&repo_str).size(12.0).color(if is_current {
                        VsCodeTheme::TEXT_BRIGHT
                    } else {
                        VsCodeTheme::TEXT_PRIMARY
                    }),
                );
                if resp.clicked() {
                    state.open_repository(std::path::PathBuf::from(repo_str), ctx);
                    state.active_sidebar = ActiveSidebarPanel::SourceControl;
                }
            });
        }

        ui.add_space(14.0);
        ui.separator();

        // Elenco Branch, Tag, Stash e Remote rapidi
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("BRANCHES ({})", state.status.branches.len()))
                    .size(11.0)
                    .strong()
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        for br in state.status.branches.clone() {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                let icon = if br.is_current {
                    Codicon::Check
                } else if br.is_remote {
                    Codicon::CloudUpload
                } else {
                    Codicon::GitBranch
                };
                draw_inline_codicon(ui, icon, 14.0, VsCodeTheme::TEXT_PRIMARY);
                let label_text = if let Some(ref up) = br.upstream {
                    format!("{} ({}) -> {}", br.name, br.commit_short, up)
                } else {
                    format!("{} ({})", br.name, br.commit_short)
                };
                let resp = ui
                    .selectable_label(br.is_current, label_text)
                    .on_hover_text(format!("{}\nRight-click for branch actions", br.subject));
                if resp.clicked() && !br.is_current {
                    let name = br.name.clone();
                    let is_rem = br.is_remote;
                    state.spawn_git_task("Checking out branch...", move |repo| {
                        ops::checkout_branch(repo, &name, is_rem)
                            .map(|l| (format!("Checkout su '{}'", name), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }
                resp.context_menu(|ui| {
                    ui.set_min_width(190.0);
                    if ui
                        .button(format!("Create Branch from '{}'...", br.name))
                        .clicked()
                    {
                        state.quick_pick =
                            Some(QuickPickState::new(QuickPickKind::CreateBranch {
                                from_ref: Some(br.name.clone()),
                            }));
                        ui.close_menu();
                    }
                    if !br.is_current && ui.button(format!("Checkout '{}'", br.name)).clicked() {
                        let name = br.name.clone();
                        let is_rem = br.is_remote;
                        state.spawn_git_task("Checking out branch...", move |repo| {
                            ops::checkout_branch(repo, &name, is_rem)
                                .map(|l| (format!("Checkout su '{}'", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                        ui.close_menu();
                    }
                    if !br.is_current
                        && ui
                            .button(format!(
                                "Merge '{}' into '{}'",
                                br.name, state.status.head_name
                            ))
                            .clicked()
                    {
                        let name = br.name.clone();
                        state.spawn_git_task("Merging branch...", move |repo| {
                            ops::merge_branch(repo, &name)
                                .map(|l| (format!("Merge di '{}' completato", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                        ui.close_menu();
                    }
                    if !br.is_current
                        && !br.is_remote
                        && ui.button(format!("Delete Branch '{}'", br.name)).clicked()
                    {
                        let name = br.name.clone();
                        state.spawn_git_task("Deleting branch...", move |repo| {
                            ops::delete_branch(repo, &name, false)
                                .map(|l| (format!("Branch '{}' eliminato", name), vec![l]))
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                        ui.close_menu();
                    }
                });
            });
        }

        if !state.status.remotes.is_empty() {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!("REMOTES ({})", state.status.remotes.len()))
                        .size(11.0)
                        .strong()
                        .color(VsCodeTheme::TEXT_MUTED),
                );
            });
            for rem in &state.status.remotes {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    draw_inline_codicon(ui, Codicon::CloudUpload, 14.0, VsCodeTheme::TEXT_MUTED);
                    ui.label(
                        RichText::new(format!("{}  -  {}", rem.name, rem.url))
                            .size(11.5)
                            .color(VsCodeTheme::TEXT_PRIMARY),
                    );
                });
            }
        }
    });
}

fn render_settings_sidebar(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new("SETTINGS - GIT CONFIGURATION")
                .size(11.0)
                .strong()
                .color(VsCodeTheme::TEXT_PRIMARY),
        );
    });
    ui.add_space(6.0);
    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("settings_sidebar_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    let card_w = (ui.available_width() - 10.0).max(160.0);
                    ui.set_width(card_w);

                    // Sezione 1: Git Status Refresh
                    ui.label(
                        RichText::new("Git: Auto Refresh Status")
                            .size(12.5)
                            .strong()
                            .color(VsCodeTheme::TEXT_BRIGHT),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            " Tempo di 'Refresh git status' automatico (in secondi).\n Imposta 0 per disabilitare il polling a tempo.",
                        )
                        .size(11.5)
                        .color(VsCodeTheme::TEXT_MUTED),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Intervallo (secondi):")
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        );
                        let mut secs = state.settings.refresh_interval_secs;
                        let drag = egui::DragValue::new(&mut secs)
                            .range(0..=3600)
                            .speed(1.0)
                            .suffix(" s");
                        if ui.add(drag).changed() {
                            state.settings.refresh_interval_secs = secs;
                            state.settings.save();
                        }
                    });

                    ui.add_space(4.0);
                    let mut slider_secs = state.settings.refresh_interval_secs;
                    if ui
                        .add(
                            egui::Slider::new(&mut slider_secs, 0..=60)
                                .text("sec")
                                .show_value(false),
                        )
                        .changed()
                    {
                        state.settings.refresh_interval_secs = slider_secs;
                        state.settings.save();
                    }

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Sezione 2: Messaggio Status Bar vs Silent
                    ui.label(
                        RichText::new("Git: Refresh Status Bar Notification")
                            .size(12.5)
                            .strong()
                            .color(VsCodeTheme::TEXT_BRIGHT),
                    );
                    ui.add_space(6.0);

                    let mut show_msg = state.settings.show_refresh_status_message;
                    if ui
                        .checkbox(
                            &mut show_msg,
                            RichText::new("Show 'Refreshing Git status...' in Status Bar")
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        )
                        .changed()
                    {
                        state.settings.show_refresh_status_message = show_msg;
                        if !show_msg && state.busy_label == "Refreshing Git status..." {
                            state.is_busy = false;
                            state.busy_label.clear();
                        }
                        state.settings.save();
                    }

                    let mode_desc = if state.settings.show_refresh_status_message {
                        "Stato attuale: VISIBILE (mostra spinner e messaggio nella Status Bar)"
                    } else {
                        "Stato attuale: SILENT (aggiorna in background senza messaggi nella Status Bar)"
                    };
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(mode_desc)
                            .size(11.0)
                            .italics()
                            .color(if state.settings.show_refresh_status_message {
                                VsCodeTheme::ACCENT_BLUE
                            } else {
                                VsCodeTheme::TEXT_MUTED
                            }),
                    );

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Sezione 3: Altre preferenze UI
                    ui.label(
                        RichText::new("Git: View & Commit Preferences")
                            .size(12.5)
                            .strong()
                            .color(VsCodeTheme::TEXT_BRIGHT),
                    );
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Default View:")
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        );
                        if ui
                            .radio_value(
                                &mut state.settings.default_file_view_mode,
                                ScmFileViewMode::Tree,
                                RichText::new("Tree View")
                                    .size(12.0)
                                    .color(VsCodeTheme::TEXT_PRIMARY),
                            )
                            .changed()
                        {
                            state.settings.file_view_mode = state.settings.default_file_view_mode;
                            state.settings.save();
                        }
                        if ui
                            .radio_value(
                                &mut state.settings.default_file_view_mode,
                                ScmFileViewMode::List,
                                RichText::new("List View")
                                    .size(12.0)
                                    .color(VsCodeTheme::TEXT_PRIMARY),
                            )
                            .changed()
                        {
                            state.settings.file_view_mode = state.settings.default_file_view_mode;
                            state.settings.save();
                        }
                    });
                    ui.add_space(4.0);

                    if ui
                        .checkbox(
                            &mut state.settings.show_graph_section,
                            RichText::new("Show Source Control Graph in Sidebar")
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        )
                        .changed()
                    {
                        state.settings.save();
                    }

                    if ui
                        .checkbox(
                            &mut state.settings.commit_signoff,
                            RichText::new("Always Sign Off Commits (--signoff)")
                                .size(12.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        )
                        .changed()
                    {
                        state.settings.save();
                    }
                });
            });
        });
}

fn render_no_repo_welcome(ui: &mut egui::Ui, state: &mut AppState) {
    let ctx = ui.ctx().clone();
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(
            RichText::new("The folder currently open doesn't have a Git repository.")
                .size(12.5)
                .color(VsCodeTheme::TEXT_PRIMARY),
        );
        ui.add_space(12.0);
        if codicon_text_button(
            ui,
            Codicon::Folder,
            "Open Repository Folder",
            VsCodeTheme::ACCENT_BLUE,
            Color32::WHITE,
            Vec2::new(200.0, 28.0),
            CornerRadius::same(3),
        )
        .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.open_repository(folder, &ctx);
            }
        }

        ui.add_space(8.0);
        if codicon_text_button(
            ui,
            Codicon::GitBranch,
            "Initialize Repository",
            VsCodeTheme::ACCENT_BLUE,
            Color32::WHITE,
            Vec2::new(200.0, 28.0),
            CornerRadius::same(3),
        )
        .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.init_repository_at(folder, &ctx);
            }
        }
    });
}

/// Menu `...` completo identico a VS Code Source Control.
fn render_more_actions_menu(ui: &mut egui::Ui, state: &mut AppState) {
    ui.set_min_width(210.0);

    ui.menu_button("View & Sort", |ui| {
        if ui
            .radio_value(
                &mut state.settings.file_view_mode,
                ScmFileViewMode::List,
                "View as List",
            )
            .changed()
        {
            state.settings.save();
            ui.close_menu();
        }
        if ui
            .radio_value(
                &mut state.settings.file_view_mode,
                ScmFileViewMode::Tree,
                "View as Tree",
            )
            .changed()
        {
            state.settings.save();
            ui.close_menu();
        }
        ui.separator();
        if ui
            .checkbox(
                &mut state.settings.show_graph_section,
                "Show Source Control Graph",
            )
            .changed()
        {
            state.settings.save();
        }
    });

    ui.separator();

    ui.menu_button("Pull, Push", |ui| {
        if ui.button("Sync").clicked() {
            let has_up = state.status.upstream.is_some();
            let br = state.status.head_name.clone();
            state.spawn_git_task("Syncing...", move |repo| {
                ops::sync_repo(repo, has_up, &br)
                    .map(|logs| ("Sync completato".into(), logs))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
        if ui.button("Pull").clicked() {
            state.spawn_git_task("Pulling...", move |repo| {
                ops::pull(repo, false)
                    .map(|l| ("Pull completato".into(), vec![l]))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
        if ui.button("Pull (Rebase)").clicked() {
            state.spawn_git_task("Pulling (Rebase)...", move |repo| {
                ops::pull(repo, true)
                    .map(|l| ("Pull (Rebase) completato".into(), vec![l]))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
        if ui.button("Push").clicked() {
            let has_up = state.status.upstream.is_some();
            let br = state.status.head_name.clone();
            state.spawn_git_task("Pushing...", move |repo| {
                let up = if has_up { None } else { Some(br.as_str()) };
                ops::push(repo, up, false)
                    .map(|l| ("Push completato".into(), vec![l]))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
        if ui.button("Push (Force with Lease)").clicked() {
            state.spawn_git_task("Force Pushing...", move |repo| {
                ops::push(repo, None, true)
                    .map(|l| ("Push (--force-with-lease) completato".into(), vec![l]))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Fetch All (Prune)").clicked() {
            state.spawn_git_task("Fetching all remotes...", move |repo| {
                ops::fetch_all(repo)
                    .map(|l| ("Fetch completato".into(), vec![l]))
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
    });

    ui.menu_button("Commit", |ui| {
        if ui.button("Commit").clicked() {
            state.execute_commit(CommitVariant::Normal);
            ui.close_menu();
        }
        if ui.button("Commit (Amend)").clicked() {
            state.commit_amend = true;
            state.execute_commit(CommitVariant::Amend);
            ui.close_menu();
        }
        if ui.button("Commit (Signed Off)").clicked() {
            state.execute_commit(CommitVariant::SignOff);
            ui.close_menu();
        }
        if ui.button("Commit & Push").clicked() {
            state.execute_commit(CommitVariant::CommitAndPush);
            ui.close_menu();
        }
        if ui.button("Commit & Sync").clicked() {
            state.execute_commit(CommitVariant::CommitAndSync);
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Undo Last Commit (soft reset)").clicked() {
            state.spawn_git_task("Undoing last commit...", move |repo| {
                ops::undo_last_commit(repo)
                    .map(|l| {
                        (
                            "Ultimo commit annullato (modifiche mantenute in stage)".into(),
                            vec![l],
                        )
                    })
                    .map_err(|(e, l)| (e, vec![l]))
            });
            ui.close_menu();
        }
    });

    ui.menu_button("Changes", |ui| {
        if ui.button("+ Stage All Changes").clicked() {
            state.stage_all();
            ui.close_menu();
        }
        if ui.button("- Unstage All Changes").clicked() {
            state.unstage_all();
            ui.close_menu();
        }
        if ui.button("Discard All Changes...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::ConfirmDiscardAll));
            ui.close_menu();
        }
    });

    ui.menu_button("Branch", |ui| {
        if ui.button("Checkout to...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
            ui.close_menu();
        }
        if ui.button("Create Branch...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                from_ref: None,
            }));
            ui.close_menu();
        }
        if ui.button("Create Branch From...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranchFrom {
                preset_name: None,
            }));
            ui.close_menu();
        }
        if ui.button("Rename Branch...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::RenameBranch));
            ui.close_menu();
        }
        if ui.button("Delete Branch...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::DeleteBranch {
                force: false,
            }));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Merge Branch...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::MergeBranch));
            ui.close_menu();
        }
        if ui.button("Rebase Branch...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::RebaseBranch));
            ui.close_menu();
        }
    });

    ui.menu_button("Stash", |ui| {
        if ui.button("Stash Changes...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPush {
                include_untracked: false,
            }));
            ui.close_menu();
        }
        if ui.button("Stash (Include Untracked)...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPush {
                include_untracked: true,
            }));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Pop Stash...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPop));
            ui.close_menu();
        }
        if ui.button("Apply Stash...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashApply));
            ui.close_menu();
        }
        if ui.button("Drop Stash...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashDrop));
            ui.close_menu();
        }
    });

    ui.menu_button("Tags", |ui| {
        if ui.button("Create Tag...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateTag));
            ui.close_menu();
        }
        if ui.button("Delete Tag...").clicked() {
            state.quick_pick = Some(QuickPickState::new(QuickPickKind::DeleteTag));
            ui.close_menu();
        }
    });

    ui.separator();
    if ui.button("Command Palette... (Ctrl+Shift+P)").clicked() {
        state.quick_pick = Some(QuickPickState::new(QuickPickKind::CommandPalette));
        ui.close_menu();
    }
    if ui.button("Show Git Output").clicked() {
        state.settings.show_output_panel = true;
        state.settings.save();
        ui.close_menu();
    }
}

/// Restituisce un piccolo badge testuale ASCII sicuro e colore in base all'estensione del file.
/// Se ritorna `""`, viene disegnata l'icona vettoriale `Codicon::FileCode`.
fn file_extension_badge(filename: &str) -> (&'static str, Color32) {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" => ("RS", Color32::from_rgb(0xDE, 0xA5, 0x84)),
        "toml" | "yaml" | "yml" | "json" => ("{}", Color32::from_rgb(0xCB, 0xCB, 0x41)),
        "md" | "txt" => ("MD", Color32::from_rgb(0x51, 0x9A, 0xBA)),
        "ts" | "tsx" => ("TS", Color32::from_rgb(0x31, 0x78, 0xC6)),
        "js" | "jsx" => ("JS", Color32::from_rgb(0xF1, 0xE0, 0x5A)),
        "py" => ("PY", Color32::from_rgb(0x35, 0x72, 0xA5)),
        "html" | "css" => ("<>", Color32::from_rgb(0xE3, 0x4C, 0x26)),
        _ => ("", VsCodeTheme::TEXT_MUTED),
    }
}
