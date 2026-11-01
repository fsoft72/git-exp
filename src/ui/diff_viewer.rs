use crate::git::diff::{DiffHunk, DiffLineKind, DiffTarget, FileDiff};
use crate::git::graph::CommitDetails;
use crate::settings::DiffViewMode;
use crate::state::{AppState, CenterTab, QuickPickKind, QuickPickState};
use crate::theme::{Codicon, VsCodeTheme, codicon_button, codicon_text_button};
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2,
    text::LayoutJob,
};

pub fn render_center_panel(ctx: &egui::Context, state: &mut AppState) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(VsCodeTheme::EDITOR_BG))
        .show(ctx, |ui| {
            if state.open_tabs.is_empty() {
                render_empty_editor_watermark(ui, state);
                return;
            }

            // 1. Barra dei Tab in alto (stile VS Code Editor Tabs)
            render_editor_tab_bar(ui, state);

            // 2. Contenuto del Tab attivo
            if let Some(active_tab) = state.open_tabs.get(state.active_tab_idx).cloned() {
                match active_tab {
                    CenterTab::Diff(diff) => render_diff_tab(ui, state, &diff),
                    CenterTab::CommitView(details) => {
                        render_commit_inspector_tab(ui, state, &details)
                    }
                    CenterTab::FileEditor { path, .. } => {
                        let idx = state.active_tab_idx;
                        render_file_editor_tab(ui, state, idx, &path);
                    }
                }
            }
        });
}

fn render_editor_tab_bar(ui: &mut egui::Ui, state: &mut AppState) {
    let tab_bar_h = 35.0;
    egui::Frame::NONE
        .fill(VsCodeTheme::EDITOR_TAB_BAR_BG)
        .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
        .show(ui, |ui| {
            ui.set_height(tab_bar_h);
            egui::ScrollArea::horizontal()
                .id_salt("editor_tabs_scroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 1.0;
                        let mut tab_to_close: Option<usize> = None;
                        let mut tab_to_activate: Option<usize> = None;

                        for (idx, tab) in state.open_tabs.iter().enumerate() {
                            let is_active = idx == state.active_tab_idx;
                            let title = tab.title();
                            let tab_w = (title.len() as f32 * 7.2 + 44.0).clamp(120.0, 260.0);
                            let (rect, resp) = ui.allocate_exact_size(
                                Vec2::new(tab_w, tab_bar_h),
                                Sense::click(),
                            );
                            let hovered = resp.hovered() || ui.rect_contains_pointer(rect);

                            let bg = if is_active {
                                VsCodeTheme::EDITOR_TAB_ACTIVE_BG
                            } else if hovered {
                                VsCodeTheme::LIST_HOVER_BG
                            } else {
                                VsCodeTheme::EDITOR_TAB_INACTIVE_BG
                            };
                            ui.painter().rect_filled(rect, CornerRadius::ZERO, bg);

                            // Bordo destro tra tab
                            ui.painter().line_segment(
                                [
                                    Pos2::new(rect.right(), rect.top()),
                                    Pos2::new(rect.right(), rect.bottom()),
                                ],
                                Stroke::new(1.0_f32, VsCodeTheme::BORDER),
                            );

                            // Linea superiore blu di tab attivo
                            if is_active {
                                let top_bar = Rect::from_min_max(
                                    rect.min,
                                    Pos2::new(rect.right(), rect.top() + 2.0),
                                );
                                ui.painter().rect_filled(
                                    top_bar,
                                    CornerRadius::ZERO,
                                    VsCodeTheme::FOCUS_BORDER,
                                );
                            }

                            let mut tab_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect)
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            tab_ui.add_space(10.0);
                            tab_ui.add(
                                egui::Label::new(
                                    RichText::new(&title).size(12.0).color(if is_active {
                                        VsCodeTheme::TEXT_BRIGHT
                                    } else {
                                        VsCodeTheme::TEXT_MUTED
                                    }),
                                )
                                .truncate(),
                            );

                            let mut close_clicked = false;
                            tab_ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(6.0);
                                    if (is_active || hovered)
                                        && codicon_button(
                                            ui,
                                            Codicon::Close,
                                            "Close Tab (Middle-Click)",
                                        )
                                    {
                                        close_clicked = true;
                                    }
                                },
                            );

                            if close_clicked || resp.middle_clicked() {
                                tab_to_close = Some(idx);
                            } else if resp.clicked() {
                                tab_to_activate = Some(idx);
                            }
                        }

                        if let Some(act) = tab_to_activate {
                            state.active_tab_idx = act;
                            state.selected_hunk_lines.clear();
                        }
                        if let Some(cls) = tab_to_close {
                            state.close_tab(cls);
                        }
                    });
                });
        });
}

fn render_diff_tab(ui: &mut egui::Ui, state: &mut AppState, diff: &FileDiff) {
    // 1. Breadcrumb & Toolbar del Diff Editor
    egui::Frame::NONE
        .fill(VsCodeTheme::EDITOR_BG)
        .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Percorso breadcrumb stile VS Code
                let breadcrumb = diff.target.path().replace('/', " > ");
                ui.label(
                    RichText::new(breadcrumb)
                        .size(12.0)
                        .color(VsCodeTheme::TEXT_MUTED),
                );

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("+{}", diff.additions))
                        .size(11.5)
                        .strong()
                        .color(VsCodeTheme::GIT_ADDED),
                );
                ui.label(
                    RichText::new(format!("-{}", diff.deletions))
                        .size(11.5)
                        .strong()
                        .color(VsCodeTheme::GIT_DELETED),
                );

                // Controlli a destra nella toolbar del Diff Editor
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Toggle Side-by-Side / Inline con icone vettoriali Codicon
                    let is_sbs = state.settings.diff_view_mode == DiffViewMode::SideBySide;
                    if codicon_text_button(
                        ui,
                        Codicon::DiffInline,
                        "Inline",
                        if !is_sbs {
                            VsCodeTheme::LIST_ACTIVE_SELECTION_BG
                        } else {
                            Color32::TRANSPARENT
                        },
                        VsCodeTheme::TEXT_PRIMARY,
                        Vec2::new(62.0, 20.0),
                        CornerRadius::same(3),
                    )
                    .on_hover_text("Inline Diff View")
                    .clicked()
                    {
                        state.settings.diff_view_mode = DiffViewMode::Inline;
                        state.settings.save();
                    }
                    if codicon_text_button(
                        ui,
                        Codicon::SplitHorizontal,
                        "Side-by-Side",
                        if is_sbs {
                            VsCodeTheme::LIST_ACTIVE_SELECTION_BG
                        } else {
                            Color32::TRANSPARENT
                        },
                        VsCodeTheme::TEXT_PRIMARY,
                        Vec2::new(98.0, 20.0),
                        CornerRadius::same(3),
                    )
                    .on_hover_text("Side-by-Side Diff View")
                    .clicked()
                    {
                        state.settings.diff_view_mode = DiffViewMode::SideBySide;
                        state.settings.save();
                    }

                    ui.separator();

                    // Azioni sull'intero file con icone vettoriali Codicon
                    let path_str = diff.target.path().to_string();
                    if codicon_button(ui, Codicon::OpenFile, "Open File in Editor") {
                        state.open_file_in_editor(&path_str);
                    }
                    if diff.target.can_stage()
                        && codicon_button(ui, Codicon::Plus, "Stage Entire File")
                    {
                        state.stage_file(path_str.clone());
                    }
                    if diff.target.can_unstage()
                        && codicon_button(ui, Codicon::Minus, "Unstage Entire File")
                    {
                        state.unstage_file(path_str.clone());
                    }
                    if diff.target.can_revert()
                        && codicon_button(ui, Codicon::Discard, "Discard All Changes in File")
                    {
                        if let DiffTarget::WorkingTree { path, kind } = &diff.target {
                            state.quick_pick = Some(QuickPickState::new(
                                QuickPickKind::ConfirmDiscardFile {
                                    path: path.clone(),
                                    kind: *kind,
                                },
                            ));
                        }
                    }

                    ui.separator();

                    // Navigazione Hunk: Next / Previous Change
                    let hunk_cnt = diff.hunks.len();
                    if codicon_button(ui, Codicon::ArrowDown, "Next Change (F7)") && hunk_cnt > 0 {
                        state.current_hunk_cursor = (state.current_hunk_cursor + 1) % hunk_cnt;
                        state.scroll_to_hunk = Some(state.current_hunk_cursor);
                    }
                    if codicon_button(ui, Codicon::ArrowUp, "Previous Change (Shift+F7)")
                        && hunk_cnt > 0
                    {
                        state.current_hunk_cursor = if state.current_hunk_cursor == 0 {
                            hunk_cnt - 1
                        } else {
                            state.current_hunk_cursor - 1
                        };
                        state.scroll_to_hunk = Some(state.current_hunk_cursor);
                    }
                    if hunk_cnt > 0 {
                        ui.label(
                            RichText::new(format!(
                                "Hunk {}/{}",
                                (state.current_hunk_cursor + 1).min(hunk_cnt),
                                hunk_cnt
                            ))
                            .size(11.0)
                            .color(VsCodeTheme::TEXT_MUTED),
                        );
                    }
                });
            });
        });

    if diff.is_binary {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("Binary file modified - cannot display text diff.")
                    .size(13.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        return;
    }

    if diff.hunks.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new("No textual changes to display in this view.")
                    .size(13.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        return;
    }

    // Banner informativo per la selezione di singole righe
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x1B, 0x22, 0x28))
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(
                        "Tip: Click any added (+) or removed (-) line to select individual lines for partial Staging / Unstaging / Reverting.",
                    )
                    .size(11.0)
                    .color(VsCodeTheme::TEXT_MUTED),
                );
            });
        });

    // 2. Area Scrollabile degli Hunk
    egui::ScrollArea::both()
        .id_salt("diff_viewer_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (hunk_idx, hunk) in diff.hunks.iter().enumerate() {
                render_hunk_block(ui, state, diff, hunk_idx, hunk);
                ui.add_space(6.0);
            }
        });
}

fn render_hunk_block(
    ui: &mut egui::Ui,
    state: &mut AppState,
    diff: &FileDiff,
    hunk_idx: usize,
    hunk: &DiffHunk,
) {
    let selected_count = state
        .selected_hunk_lines
        .get(&hunk_idx)
        .map(|s| s.len())
        .unwrap_or(0);

    // 1. Barra Header dell'Hunk
    let header_frame = egui::Frame::NONE
        .fill(VsCodeTheme::DIFF_HUNK_HEADER_BG)
        .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
        .inner_margin(egui::Margin::symmetric(10, 4));

    let resp = header_frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                ))
                .monospace()
                .size(12.0)
                .color(Color32::from_rgb(0x56, 0x9C, 0xD6)),
            );

            if !hunk.section_context.is_empty() {
                ui.label(
                    RichText::new(&hunk.section_context)
                        .monospace()
                        .size(12.0)
                        .color(VsCodeTheme::TEXT_MUTED),
                );
            }

            // Pulsanti interattivi di Hunk e Righe Selezionate
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !diff.is_untracked {
                    if diff.target.can_stage() {
                        if hunk_pill_button(
                            ui,
                            "+ Stage Hunk",
                            VsCodeTheme::ACCENT_BLUE,
                            "Stage this entire hunk (git apply --cached)",
                        ) {
                            if let Some(patch) = diff.build_hunk_patch(hunk_idx) {
                                state.apply_diff_patch(patch, true, false, "Staging hunk...");
                            }
                        }
                    }

                    if diff.target.can_unstage() {
                        if hunk_pill_button(
                            ui,
                            "- Unstage Hunk",
                            VsCodeTheme::ACCENT_BLUE,
                            "Unstage this entire hunk (git apply --cached --reverse)",
                        ) {
                            if let Some(patch) = diff.build_hunk_patch(hunk_idx) {
                                state.apply_diff_patch(patch, true, true, "Unstaging hunk...");
                            }
                        }
                    }

                    if diff.target.can_revert() {
                        if hunk_pill_button(
                            ui,
                            "Revert Hunk",
                            Color32::from_rgb(0x8E, 0x34, 0x2B),
                            "Discard changes in this hunk from Working Tree (git apply --reverse)",
                        ) {
                            if let Some(patch) = diff.build_hunk_patch(hunk_idx) {
                                state.apply_diff_patch(patch, false, true, "Reverting hunk...");
                            }
                        }
                    }

                    // Se ci sono righe selezionate in questo hunk, mostra i bottoni per le righe selezionate!
                    if selected_count > 0 {
                        ui.separator();
                        if ui
                            .small_button(
                                RichText::new("Clear")
                                    .size(11.0)
                                    .color(VsCodeTheme::TEXT_MUTED),
                            )
                            .clicked()
                        {
                            state.selected_hunk_lines.remove(&hunk_idx);
                        }

                        if diff.target.can_revert() {
                            let lbl = format!("Revert Selected ({})", selected_count);
                            if hunk_pill_button(
                                ui,
                                &lbl,
                                Color32::from_rgb(0xA1, 0x3A, 0x2E),
                                "Revert only the selected lines in Working Tree",
                            ) {
                                if let Some(sel) = state.selected_hunk_lines.get(&hunk_idx) {
                                    if let Some(patch) =
                                        diff.build_selected_lines_patch(hunk_idx, sel, true)
                                    {
                                        state.apply_diff_patch(
                                            patch,
                                            false,
                                            true,
                                            "Reverting selected lines...",
                                        );
                                    }
                                }
                            }
                        }

                        if diff.target.can_stage() {
                            let lbl = format!("+ Stage Selected ({})", selected_count);
                            if hunk_pill_button(
                                ui,
                                &lbl,
                                Color32::from_rgb(0x1E, 0x7A, 0x3A),
                                "Stage only the selected lines (git apply --cached)",
                            ) {
                                if let Some(sel) = state.selected_hunk_lines.get(&hunk_idx) {
                                    if let Some(patch) =
                                        diff.build_selected_lines_patch(hunk_idx, sel, false)
                                    {
                                        state.apply_diff_patch(
                                            patch,
                                            true,
                                            false,
                                            "Staging selected lines...",
                                        );
                                    }
                                }
                            }
                        }

                        if diff.target.can_unstage() {
                            let lbl = format!("- Unstage Selected ({})", selected_count);
                            if hunk_pill_button(
                                ui,
                                &lbl,
                                Color32::from_rgb(0x9C, 0x64, 0x16),
                                "Unstage only the selected lines (git apply --cached --reverse)",
                            ) {
                                if let Some(sel) = state.selected_hunk_lines.get(&hunk_idx) {
                                    if let Some(patch) =
                                        diff.build_selected_lines_patch(hunk_idx, sel, true)
                                    {
                                        state.apply_diff_patch(
                                            patch,
                                            true,
                                            true,
                                            "Unstaging selected lines...",
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
    });

    if state.scroll_to_hunk == Some(hunk_idx) {
        resp.response.scroll_to_me(Some(egui::Align::TOP));
        state.scroll_to_hunk = None;
    }

    // 2. Corpo delle righe dell'Hunk (Side-by-Side oppure Inline)
    match state.settings.diff_view_mode {
        DiffViewMode::SideBySide => {
            render_hunk_side_by_side(ui, state, diff.target.path(), hunk_idx, hunk);
        }
        DiffViewMode::Inline => {
            render_hunk_inline(ui, state, diff.target.path(), hunk_idx, hunk);
        }
    }
}

fn render_hunk_inline(
    ui: &mut egui::Ui,
    state: &mut AppState,
    file_path: &str,
    hunk_idx: usize,
    hunk: &DiffHunk,
) {
    let row_h = 20.0;
    let avail_w = ui.available_width().max(600.0);

    for (line_idx, line) in hunk.lines.iter().enumerate() {
        let is_selectable = line.kind != DiffLineKind::Context;
        let is_selected = state
            .selected_hunk_lines
            .get(&hunk_idx)
            .map(|s| s.contains(&line_idx))
            .unwrap_or(false);

        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(avail_w, row_h),
            if is_selectable {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            let bg = if is_selected {
                VsCodeTheme::DIFF_SELECTED_LINE_BG
            } else {
                match line.kind {
                    DiffLineKind::Added => VsCodeTheme::DIFF_INSERT_LINE_BG,
                    DiffLineKind::Removed => VsCodeTheme::DIFF_DELETE_LINE_BG,
                    DiffLineKind::Context => VsCodeTheme::EDITOR_BG,
                }
            };
            painter.rect_filled(rect, CornerRadius::ZERO, bg);

            match line.kind {
                DiffLineKind::Added => {
                    let g =
                        Rect::from_min_max(rect.min, Pos2::new(rect.left() + 3.0, rect.bottom()));
                    painter.rect_filled(g, CornerRadius::ZERO, VsCodeTheme::GIT_ADDED);
                }
                DiffLineKind::Removed => {
                    let g =
                        Rect::from_min_max(rect.min, Pos2::new(rect.left() + 3.0, rect.bottom()));
                    painter.rect_filled(g, CornerRadius::ZERO, VsCodeTheme::GIT_DELETED);
                }
                DiffLineKind::Context => {}
            }

            let old_s = line
                .old_lineno
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".to_string());
            let new_s = line
                .new_lineno
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".to_string());
            let sign = match line.kind {
                DiffLineKind::Added => "+",
                DiffLineKind::Removed => "-",
                DiffLineKind::Context => " ",
            };

            let gutter_text = format!("{} {} {} ", old_s, new_s, sign);
            let gutter_galley = painter.layout_no_wrap(
                gutter_text,
                FontId::new(12.0, FontFamily::Monospace),
                if is_selected {
                    VsCodeTheme::TEXT_BRIGHT
                } else {
                    VsCodeTheme::TEXT_MUTED
                },
            );
            painter.galley(
                Pos2::new(rect.left() + 6.0, rect.top() + 2.0),
                gutter_galley,
                VsCodeTheme::TEXT_MUTED,
            );

            let code_x = rect.left() + 92.0;
            draw_highlighted_code_line(
                painter,
                &state.highlighter,
                file_path,
                &line.content,
                Pos2::new(code_x, rect.top() + 2.0),
            );
        }

        if is_selectable && resp.clicked() {
            let entry = state.selected_hunk_lines.entry(hunk_idx).or_default();
            if !entry.remove(&line_idx) {
                entry.insert(line_idx);
            }
        }
    }
}

fn render_hunk_side_by_side(
    ui: &mut egui::Ui,
    state: &mut AppState,
    file_path: &str,
    hunk_idx: usize,
    hunk: &DiffHunk,
) {
    let rows = hunk.build_side_by_side_rows();
    let row_h = 20.0;
    let total_w = ui.available_width().max(700.0);
    let half_w = (total_w - 2.0) * 0.5;

    for row in rows {
        let (row_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, row_h), Sense::hover());
        let left_rect = Rect::from_min_size(row_rect.min, Vec2::new(half_w, row_h));
        let right_rect = Rect::from_min_size(
            Pos2::new(row_rect.left() + half_w + 2.0, row_rect.top()),
            Vec2::new(half_w, row_h),
        );

        if ui.is_rect_visible(row_rect) {
            ui.painter().line_segment(
                [
                    Pos2::new(row_rect.left() + half_w + 1.0, row_rect.top()),
                    Pos2::new(row_rect.left() + half_w + 1.0, row_rect.bottom()),
                ],
                Stroke::new(1.0_f32, VsCodeTheme::BORDER),
            );
        }

        render_sbs_cell(
            ui,
            state,
            file_path,
            hunk_idx,
            left_rect,
            row.left.as_ref(),
        );
        render_sbs_cell(
            ui,
            state,
            file_path,
            hunk_idx,
            right_rect,
            row.right.as_ref(),
        );
    }
}

fn render_sbs_cell(
    ui: &mut egui::Ui,
    state: &mut AppState,
    file_path: &str,
    hunk_idx: usize,
    rect: Rect,
    cell: Option<&crate::git::diff::SideBySideCell>,
) {
    let Some(c) = cell else {
        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::DIFF_EMPTY_FILL_BG);
        }
        return;
    };

    let is_selectable = c.kind != DiffLineKind::Context;
    let is_selected = state
        .selected_hunk_lines
        .get(&hunk_idx)
        .map(|s| s.contains(&c.line_idx))
        .unwrap_or(false);

    let id = ui.make_persistent_id(("sbs_cell", hunk_idx, c.line_idx));
    let resp = ui.interact(
        rect,
        id,
        if is_selectable {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    if ui.is_rect_visible(rect) {
        let painter = ui.painter().with_clip_rect(rect);

        let bg = if is_selected {
            VsCodeTheme::DIFF_SELECTED_LINE_BG
        } else {
            match c.kind {
                DiffLineKind::Added => VsCodeTheme::DIFF_INSERT_LINE_BG,
                DiffLineKind::Removed => VsCodeTheme::DIFF_DELETE_LINE_BG,
                DiffLineKind::Context => VsCodeTheme::EDITOR_BG,
            }
        };
        painter.rect_filled(rect, CornerRadius::ZERO, bg);

        match c.kind {
            DiffLineKind::Added => {
                painter.rect_filled(
                    Rect::from_min_max(rect.min, Pos2::new(rect.left() + 3.0, rect.bottom())),
                    CornerRadius::ZERO,
                    VsCodeTheme::GIT_ADDED,
                );
            }
            DiffLineKind::Removed => {
                painter.rect_filled(
                    Rect::from_min_max(rect.min, Pos2::new(rect.left() + 3.0, rect.bottom())),
                    CornerRadius::ZERO,
                    VsCodeTheme::GIT_DELETED,
                );
            }
            DiffLineKind::Context => {}
        }

        let sign = match c.kind {
            DiffLineKind::Added => "+",
            DiffLineKind::Removed => "-",
            DiffLineKind::Context => " ",
        };
        let ln_text = format!("{:>4} {} ", c.lineno, sign);
        let ln_galley = painter.layout_no_wrap(
            ln_text,
            FontId::new(12.0, FontFamily::Monospace),
            if is_selected {
                VsCodeTheme::TEXT_BRIGHT
            } else {
                VsCodeTheme::TEXT_MUTED
            },
        );
        painter.galley(
            Pos2::new(rect.left() + 5.0, rect.top() + 2.0),
            ln_galley,
            VsCodeTheme::TEXT_MUTED,
        );

        draw_highlighted_code_line(
            &painter,
            &state.highlighter,
            file_path,
            &c.content,
            Pos2::new(rect.left() + 56.0, rect.top() + 2.0),
        );
    }

    if is_selectable && resp.clicked() {
        let entry = state.selected_hunk_lines.entry(hunk_idx).or_default();
        if !entry.remove(&c.line_idx) {
            entry.insert(c.line_idx);
        }
    }
}

fn draw_highlighted_code_line(
    painter: &egui::Painter,
    highlighter: &crate::state::SyntaxHighlighter,
    file_path: &str,
    content: &str,
    pos: Pos2,
) {
    let expanded = content.replace('\t', "    ");
    let tokens = highlighter.highlight_line(file_path, &expanded);
    let mut job = LayoutJob::default();
    for (color, text) in tokens {
        job.append(
            &text,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(12.5, FontFamily::Monospace),
                color,
                ..Default::default()
            },
        );
    }
    let galley = painter.layout_job(job);
    painter.galley(pos, galley, VsCodeTheme::TEXT_PRIMARY);
}

fn hunk_pill_button(ui: &mut egui::Ui, label: &str, bg: Color32, tooltip: &str) -> bool {
    let btn = egui::Button::new(RichText::new(label).size(11.0).strong().color(Color32::WHITE))
        .fill(bg)
        .corner_radius(CornerRadius::same(3))
        .min_size(Vec2::new(0.0, 19.0));
    ui.add(btn).on_hover_text(tooltip).clicked()
}

fn render_commit_inspector_tab(
    ui: &mut egui::Ui,
    state: &mut AppState,
    details: &CommitDetails,
) {
    egui::ScrollArea::vertical()
        .id_salt("commit_inspector_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("Commit {}", details.sha))
                            .monospace()
                            .size(14.0)
                            .strong()
                            .color(VsCodeTheme::TEXT_BRIGHT),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Author: {} <{}>", details.author, details.email))
                            .size(12.5)
                            .color(VsCodeTheme::TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!("Date:   {}", details.date))
                            .size(12.0)
                            .color(VsCodeTheme::TEXT_MUTED),
                    );
                    if !details.parents.is_empty() {
                        ui.label(
                            RichText::new(format!("Parents: {}", details.parents.join(", ")))
                                .size(12.0)
                                .monospace()
                                .color(VsCodeTheme::TEXT_MUTED),
                        );
                    }

                    ui.add_space(12.0);
                    egui::Frame::NONE
                        .fill(VsCodeTheme::INPUT_BG)
                        .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width() - 24.0);
                            ui.label(
                                RichText::new(&details.message)
                                    .size(13.0)
                                    .color(VsCodeTheme::TEXT_BRIGHT),
                            );
                        });

                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(format!(
                            "CHANGED FILES IN COMMIT ({})",
                            details.files.len()
                        ))
                        .size(11.5)
                        .strong()
                        .color(VsCodeTheme::TEXT_MUTED),
                    );
                    ui.add_space(6.0);

                    for file in &details.files {
                        let resp = ui.selectable_label(
                            false,
                            RichText::new(format!(
                                "[{}]  {}",
                                file.kind.badge_letter(),
                                file.path
                            ))
                            .size(12.5)
                            .color(file.kind.color()),
                        );
                        if resp
                            .on_hover_text("Click to open file diff for this commit")
                            .clicked()
                        {
                            state.open_diff_target(DiffTarget::Commit {
                                commit_sha: details.sha.clone(),
                                path: file.path.clone(),
                            });
                        }
                    }
                });
            });
        });
}

fn render_file_editor_tab(
    ui: &mut egui::Ui,
    state: &mut AppState,
    tab_idx: usize,
    path: &str,
) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!("Editing: {}", path))
                .size(12.0)
                .color(VsCodeTheme::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(10.0);
            if codicon_text_button(
                ui,
                Codicon::Save,
                "Save (Ctrl+S)",
                VsCodeTheme::ACCENT_BLUE,
                Color32::WHITE,
                Vec2::new(115.0, 24.0),
                CornerRadius::same(3),
            )
            .clicked()
            {
                state.save_file_editor(tab_idx);
            }
        });
    });
    ui.separator();

    if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
        state.save_file_editor(tab_idx);
    }

    if let Some(CenterTab::FileEditor { content, dirty, .. }) = state.open_tabs.get_mut(tab_idx) {
        egui::ScrollArea::both()
            .id_salt(("file_editor_scroll", tab_idx))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let te = egui::TextEdit::multiline(content)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(36)
                    .frame(false);
                if ui.add(te).changed() {
                    *dirty = true;
                }
            });
    }
}

fn render_empty_editor_watermark(ui: &mut egui::Ui, state: &mut AppState) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            let top_pad = (ui.available_height() * 0.20).max(16.0);
            ui.add_space(top_pad);

            let title_text = RichText::new("Git-Exp")
                .size(24.0)
                .strong()
                .color(VsCodeTheme::TEXT_BRIGHT);
            let version_text = RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(13.0)
                .color(VsCodeTheme::TEXT_MUTED);
            let title_galley = egui::WidgetText::from(title_text.clone())
                .into_galley(ui, None, f32::INFINITY, egui::TextStyle::Heading);
            let version_galley = egui::WidgetText::from(version_text.clone())
                .into_galley(ui, None, f32::INFINITY, egui::TextStyle::Body);
            let icon_size = egui::vec2(28.0, 28.0);
            let spacing = 8.0;
            let row_width =
                icon_size.x + spacing + title_galley.size().x + spacing + version_galley.size().x;
            let row_height = icon_size.y.max(title_galley.size().y);

            ui.allocate_ui_with_layout(
                egui::vec2(row_width, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = spacing;
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/images/logo.svg"))
                            .fit_to_exact_size(icon_size),
                    );
                    ui.label(title_text);
                    ui.label(version_text);
                },
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("By Fabio Rotondo")
                    .size(13.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            ui.add_space(14.0);
            ui.label(
                RichText::new("This software is Open Source under the MIT license.")
                    .size(12.5)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            ui.add_space(8.0);
            let link_resp = ui.hyperlink_to(
                RichText::new("https://github.com/fsoft72/git-exp").size(12.5),
                "https://github.com/fsoft72/git-exp",
            );
            if link_resp.clicked() {
                let _ = open::that("https://github.com/fsoft72/git-exp");
            }
            ui.add_space(20.0);
            ui.label(
                RichText::new("Click any file in the Source Control sidebar to inspect its Diff")
                    .size(13.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            ui.add_space(16.0);
            ui.label(
                RichText::new("Shortcuts:")
                    .size(12.0)
                    .strong()
                    .color(VsCodeTheme::TEXT_MUTED),
            );
            ui.label(
                RichText::new("- Ctrl+Shift+P  :  Open Git Command Palette / Quick Pick")
                    .size(12.0)
                    .color(VsCodeTheme::TEXT_DIM),
            );
            ui.label(
                RichText::new("- Ctrl+Enter    :  Commit staged/working changes")
                    .size(12.0)
                    .color(VsCodeTheme::TEXT_DIM),
            );
            ui.label(
                RichText::new("- Click Branch in Status Bar  :  Checkout / Create Branch")
                    .size(12.0)
                    .color(VsCodeTheme::TEXT_DIM),
            );

            if state.repo_path.is_none() {
                ui.add_space(16.0);
                let ctx = ui.ctx().clone();
                if codicon_text_button(
                    ui,
                    Codicon::Folder,
                    "Open Repository Folder...",
                    VsCodeTheme::ACCENT_BLUE,
                    Color32::WHITE,
                    Vec2::new(220.0, 30.0),
                    CornerRadius::same(3),
                )
                .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        state.open_repository(folder, &ctx);
                    }
                }
            }
        });
    });
}
