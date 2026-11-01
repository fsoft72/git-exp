use crate::state::{ActiveSidebarPanel, AppState, QuickPickKind, QuickPickState};
use crate::theme::VsCodeTheme;
use egui::{Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};

pub fn render_activity_bar(ctx: &egui::Context, state: &mut AppState) {
    egui::SidePanel::left("vscode_activity_bar")
        .exact_width(48.0)
        .resizable(false)
        .frame(
            egui::Frame::NONE
                .fill(VsCodeTheme::ACTIVITY_BAR_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);

                // 1. Source Control Icon (con badge contatore modifiche)
                let scm_active = state.active_sidebar == ActiveSidebarPanel::SourceControl;
                if draw_activity_item(
                    ui,
                    scm_active,
                    ActivityIcon::SourceControl,
                    Some(state.status.total_changes_count()),
                    "Source Control (Ctrl+Shift+G)",
                ) {
                    state.active_sidebar = ActiveSidebarPanel::SourceControl;
                }

                // 2. Source Control Graph Icon
                let graph_active = state.active_sidebar == ActiveSidebarPanel::GraphFull;
                if draw_activity_item(
                    ui,
                    graph_active,
                    ActivityIcon::CommitGraph,
                    None,
                    "Source Control Graph (Commit History)",
                ) {
                    state.active_sidebar = ActiveSidebarPanel::GraphFull;
                }

                // 3. Repositories / Explorer Icon
                let repo_active = state.active_sidebar == ActiveSidebarPanel::Repositories;
                if draw_activity_item(
                    ui,
                    repo_active,
                    ActivityIcon::Repositories,
                    None,
                    "Repositories & Workspace",
                ) {
                    state.active_sidebar = ActiveSidebarPanel::Repositories;
                }

                // Spazio elastico fino al fondo
                let remaining = (ui.available_height() - 104.0).max(8.0);
                ui.add_space(remaining);

                // 4. Git Output Panel toggle
                if draw_activity_item(
                    ui,
                    state.settings.show_output_panel,
                    ActivityIcon::OutputTerminal,
                    None,
                    "Toggle Git Output Log (Ctrl+Shift+U)",
                ) {
                    state.settings.show_output_panel = !state.settings.show_output_panel;
                    state.settings.save();
                }

                // 5. Command Palette button
                if draw_activity_item(
                    ui,
                    state.quick_pick.is_some(),
                    ActivityIcon::CommandPalette,
                    None,
                    "Git Command Palette (Ctrl+Shift+P)",
                ) {
                    state.quick_pick = Some(QuickPickState::new(QuickPickKind::CommandPalette));
                }
            });
        });
}

enum ActivityIcon {
    SourceControl,
    CommitGraph,
    Repositories,
    OutputTerminal,
    CommandPalette,
}

fn draw_activity_item(
    ui: &mut egui::Ui,
    active: bool,
    icon: ActivityIcon,
    badge_count: Option<usize>,
    tooltip: &str,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), Sense::click());
    let hovered = resp.hovered();

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Barra verticale sinistra di selezione (2px stile VS Code)
        if active {
            let bar_rect = Rect::from_min_max(
                Pos2::new(rect.left(), rect.top() + 8.0),
                Pos2::new(rect.left() + 2.5, rect.bottom() - 8.0),
            );
            painter.rect_filled(bar_rect, CornerRadius::ZERO, VsCodeTheme::FOCUS_BORDER);
        }

        let fg = if active || hovered {
            VsCodeTheme::TEXT_BRIGHT
        } else {
            VsCodeTheme::TEXT_MUTED
        };

        let c = rect.center();
        let stroke = Stroke::new(1.7_f32, fg);

        match icon {
            ActivityIcon::SourceControl => {
                // Disegna l'icona ufficiale Git Branch di VS Code (Codicon `source-control`)
                let top_node = Pos2::new(c.x - 5.0, c.y - 8.0);
                let bot_node = Pos2::new(c.x - 5.0, c.y + 8.0);
                let right_node = Pos2::new(c.x + 6.0, c.y - 3.0);

                painter.circle_stroke(top_node, 2.7, stroke);
                painter.circle_stroke(bot_node, 2.7, stroke);
                painter.circle_stroke(right_node, 2.7, stroke);

                // Linea principale verticale
                painter.line_segment(
                    [
                        Pos2::new(top_node.x, top_node.y + 2.8),
                        Pos2::new(bot_node.x, bot_node.y - 2.8),
                    ],
                    stroke,
                );
                // Curva di diramazione dal basso verso il nodo destro
                painter.line_segment(
                    [
                        Pos2::new(bot_node.x, c.y + 2.0),
                        Pos2::new(right_node.x, c.y + 2.0),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(right_node.x, c.y + 2.0),
                        Pos2::new(right_node.x, right_node.y + 2.8),
                    ],
                    stroke,
                );
            }
            ActivityIcon::CommitGraph => {
                // Icona grafo con 3 nodi collegati
                let n1 = Pos2::new(c.x - 6.0, c.y - 7.0);
                let n2 = Pos2::new(c.x - 6.0, c.y + 7.0);
                let n3 = Pos2::new(c.x + 5.0, c.y);
                painter.line_segment([n1, n2], stroke);
                painter.line_segment([n1, n3], stroke);
                painter.line_segment([n3, n2], stroke);
                painter.circle_filled(n1, 3.0, fg);
                painter.circle_filled(n2, 3.0, fg);
                painter.circle_filled(n3, 3.0, fg);
            }
            ActivityIcon::Repositories => {
                // Icona cartella / repo
                let r = Rect::from_center_size(c, Vec2::new(18.0, 14.0));
                painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
                painter.line_segment(
                    [
                        Pos2::new(r.left() + 1.0, r.top() + 4.0),
                        Pos2::new(r.right() - 1.0, r.top() + 4.0),
                    ],
                    stroke,
                );
            }
            ActivityIcon::OutputTerminal => {
                // Icona `>_` terminale / output
                let r = Rect::from_center_size(c, Vec2::new(18.0, 14.0));
                painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
                painter.line_segment(
                    [
                        Pos2::new(c.x - 5.0, c.y - 3.0),
                        Pos2::new(c.x - 1.0, c.y),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(c.x - 5.0, c.y + 3.0),
                        Pos2::new(c.x - 1.0, c.y),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(c.x + 1.0, c.y + 3.0),
                        Pos2::new(c.x + 5.0, c.y + 3.0),
                    ],
                    stroke,
                );
            }
            ActivityIcon::CommandPalette => {
                // Icona ingranaggio / comandi
                let r = Rect::from_center_size(c, Vec2::new(16.0, 16.0));
                painter.rect_stroke(r, CornerRadius::same(3), stroke, egui::StrokeKind::Inside);
                painter.circle_stroke(c, 3.5, stroke);
            }
        }

        // Badge numerico blu in basso a destra (stile VS Code Activity Bar)
        if let Some(count) = badge_count {
            if count > 0 {
                let badge_text = if count > 99 {
                    "99+".to_string()
                } else {
                    count.to_string()
                };
                let galley = painter.layout_no_wrap(
                    badge_text,
                    FontId::new(9.5, FontFamily::Proportional),
                    Color32::WHITE,
                );
                let bw = (galley.size().x + 8.0).max(16.0);
                let bh = 15.0;
                let badge_rect = Rect::from_min_size(
                    Pos2::new(rect.right() - bw - 5.0, rect.bottom() - bh - 6.0),
                    Vec2::new(bw, bh),
                );
                painter.rect_filled(
                    badge_rect,
                    CornerRadius::same(8),
                    VsCodeTheme::ACTIVITY_BADGE_BG,
                );
                painter.rect_stroke(
                    badge_rect,
                    CornerRadius::same(8),
                    Stroke::new(1.0_f32, VsCodeTheme::ACTIVITY_BAR_BG),
                    egui::StrokeKind::Outside,
                );
                let tpos = Pos2::new(
                    badge_rect.center().x - galley.size().x * 0.5,
                    badge_rect.center().y - galley.size().y * 0.5,
                );
                painter.galley(tpos, galley, Color32::WHITE);
            }
        }
    }

    resp.on_hover_text(tooltip).clicked()
}
