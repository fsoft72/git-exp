use crate::git::graph::{CommitNode, CommitSyncRole, RefBadgeKind};
use crate::git::ops;
use crate::state::{AppState, QuickPickKind, QuickPickState};
use crate::theme::VsCodeTheme;
use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub fn render_graph_list(ui: &mut egui::Ui, state: &mut AppState, max_rows: usize) {
    if state.graph_commits.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.label(
                RichText::new("No commits in repository yet")
                    .size(12.0)
                    .italics()
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        return;
    }

    let commits: Vec<CommitNode> = state
        .graph_commits
        .iter()
        .take(max_rows)
        .cloned()
        .collect();

    let row_height = 24.0;
    let lane_spacing = 14.0;
    let left_pad = 12.0;

    // Calcola il numero massimo di corsie visibili per stabilire la larghezza della colonna grafo
    let max_lane = commits
        .iter()
        .map(|c| {
            let max_e = c
                .edges_to_next_row
                .iter()
                .map(|e| e.from_lane.max(e.to_lane))
                .max()
                .unwrap_or(0);
            c.node_lane.max(max_e)
        })
        .max()
        .unwrap_or(0);
    let graph_col_width = left_pad + ((max_lane + 1) as f32) * lane_spacing + 8.0;

    for (row_idx, commit) in commits.iter().enumerate() {
        let avail_w = ui.available_width();
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(avail_w, row_height), Sense::click());

        let is_selected = state
            .selected_commit_sha
            .as_ref()
            .map(|s| s == &commit.sha)
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

        // 1. Disegna il grafo (corsie, archi e nodo del commit) con `egui::Painter`
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let row_top = rect.top();
            let row_mid = rect.center().y;
            let row_bot = rect.bottom();

            let lane_x = |lane: usize| -> f32 {
                rect.left() + left_pad + (lane as f32) * lane_spacing
            };

            // Se non è la prima riga, disegna le linee in ingresso dall'alto verso il centro
            if row_idx > 0 {
                if let Some(prev) = commits.get(row_idx - 1) {
                    for edge in &prev.edges_to_next_row {
                        let col = VsCodeTheme::GRAPH_LANES
                            [edge.color_idx % VsCodeTheme::GRAPH_LANES.len()];
                        let stroke = Stroke::new(1.8_f32, col);
                        let x_from = lane_x(edge.from_lane);
                        let x_to = lane_x(edge.to_lane);
                        if edge.from_lane == edge.to_lane {
                            painter.line_segment(
                                [Pos2::new(x_to, row_top), Pos2::new(x_to, row_mid)],
                                stroke,
                            );
                        } else {
                            // Curva morbida dal bordo superiore al centro della corsia di arrivo
                            let mid_x = (x_from + x_to) * 0.5;
                            painter.line_segment(
                                [Pos2::new(mid_x, row_top), Pos2::new(x_to, row_mid)],
                                stroke,
                            );
                        }
                    }
                }
            }

            // Disegna gli archi dal centro della riga corrente verso il basso (riga successiva)
            for edge in &commit.edges_to_next_row {
                let col =
                    VsCodeTheme::GRAPH_LANES[edge.color_idx % VsCodeTheme::GRAPH_LANES.len()];
                let stroke = Stroke::new(1.8_f32, col);
                let x_from = lane_x(edge.from_lane);
                let x_to = lane_x(edge.to_lane);
                if edge.from_lane == edge.to_lane {
                    painter.line_segment(
                        [Pos2::new(x_from, row_mid), Pos2::new(x_to, row_bot)],
                        stroke,
                    );
                } else {
                    let mid_x = (x_from + x_to) * 0.5;
                    painter.line_segment(
                        [Pos2::new(x_from, row_mid), Pos2::new(mid_x, row_bot)],
                        stroke,
                    );
                }
            }

            // Disegna il nodo del commit
            let cx = lane_x(commit.node_lane);
            let center = Pos2::new(cx, row_mid);
            let node_col = VsCodeTheme::GRAPH_LANES
                [commit.node_color_idx % VsCodeTheme::GRAPH_LANES.len()];

            if commit.is_head {
                // Nodo HEAD stile VS Code: anello doppio brillante
                painter.circle_filled(center, 5.0, VsCodeTheme::SIDEBAR_BG);
                painter.circle_stroke(center, 4.5, Stroke::new(2.2_f32, node_col));
                painter.circle_filled(center, 2.0, Color32::WHITE);
            } else if commit.parents.len() > 1 {
                // Merge commit: cerchio con centro scuro
                painter.circle_filled(center, 4.2, VsCodeTheme::SIDEBAR_BG);
                painter.circle_stroke(center, 3.6, Stroke::new(1.8_f32, node_col));
            } else {
                painter.circle_filled(center, 3.6, node_col);
            }
        }

        // 2. Contenuto testuale a destra del grafo: badge Outgoing/Incoming, Ref pills, Subject e Autore
        let text_rect = Rect::from_min_max(
            Pos2::new(rect.left() + graph_col_width.min(avail_w * 0.4), rect.top()),
            rect.max,
        );
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(text_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );

        // Indicatore Outgoing (↑) o Incoming (↓)
        match commit.role {
            CommitSyncRole::Outgoing => {
                row_ui.label(
                    RichText::new("↑")
                        .size(11.0)
                        .strong()
                        .color(VsCodeTheme::GIT_ADDED),
                );
            }
            CommitSyncRole::Incoming => {
                row_ui.label(
                    RichText::new("↓")
                        .size(11.0)
                        .strong()
                        .color(VsCodeTheme::WARNING_YELLOW),
                );
            }
            CommitSyncRole::History => {}
        }

        // Badge dei branch / tag (Ref Pills stile VS Code Source Control Graph)
        for badge in &commit.refs {
            let (bg, fg, icon) = match badge.kind {
                RefBadgeKind::HeadBranch => (
                    VsCodeTheme::ACCENT_BLUE,
                    Color32::WHITE,
                    "⎇ ",
                ),
                RefBadgeKind::LocalBranch => (
                    Color32::from_rgb(0x2D, 0x4F, 0x67),
                    VsCodeTheme::TEXT_BRIGHT,
                    "⎇ ",
                ),
                RefBadgeKind::RemoteBranch => (
                    Color32::from_rgb(0x3B, 0x2E, 0x58),
                    Color32::from_rgb(0xD9, 0xC2, 0xFF),
                    "☁ ",
                ),
                RefBadgeKind::Tag => (
                    Color32::from_rgb(0x4D, 0x41, 0x23),
                    VsCodeTheme::GIT_MODIFIED,
                    "🏷 ",
                ),
            };

            egui::Frame::NONE
                .fill(bg)
                .corner_radius(CornerRadius::same(3))
                .inner_margin(egui::Margin::symmetric(4, 1))
                .show(&mut row_ui, |ui| {
                    ui.label(
                        RichText::new(format!("{}{}", icon, badge.name))
                            .size(10.5)
                            .strong()
                            .color(fg),
                    );
                });
        }

        // Messaggio subject del commit
        let subj_color = if is_selected || commit.is_head {
            VsCodeTheme::TEXT_BRIGHT
        } else {
            VsCodeTheme::TEXT_PRIMARY
        };
        row_ui.add(
            egui::Label::new(RichText::new(&commit.subject).size(12.5).color(subj_color))
                .truncate(),
        );

        // Autore / Tempo relativo a destra se c'è spazio
        row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(&commit.author)
                    .size(11.0)
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });

        let tooltip = format!(
            "Commit: {}\nAuthor: {} <{}>\nDate: {} ({})\n\n{}",
            commit.sha,
            commit.author,
            commit.author_email,
            commit.relative_time,
            commit.iso_date,
            commit.subject
        );
        let resp = resp.on_hover_text(tooltip);
        if resp.clicked() {
            state.open_commit_inspector(&commit.sha);
        }

        // Menu contestuale sul nodo del grafo
        resp.context_menu(|ui| {
            ui.set_min_width(190.0);
            if ui.button("Inspect Commit Details & Files").clicked() {
                state.open_commit_inspector(&commit.sha);
                ui.close_menu();
            }
            if ui
                .button(format!("Create Branch from {}...", commit.short_sha))
                .clicked()
            {
                state.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                    from_ref: Some(commit.sha.clone()),
                }));
                ui.close_menu();
            }
            if ui
                .button(format!("Checkout Commit ({})...", commit.short_sha))
                .clicked()
            {
                let sha = commit.sha.clone();
                state.spawn_git_task("Checking out commit...", move |repo| {
                    ops::checkout_branch(repo, &sha, false)
                        .map(|l| (format!("HEAD detached at {}", &sha[..7.min(sha.len())]), vec![l]))
                        .map_err(|(e, l)| (e, vec![l]))
                });
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Copy Commit Hash (SHA)").clicked() {
                ui.ctx().copy_text(commit.sha.clone());
                ui.close_menu();
            }
            if ui.button("Copy Commit Subject").clicked() {
                ui.ctx().copy_text(commit.subject.clone());
                ui.close_menu();
            }
        });
    }
}
