use crate::state::AppState;
use crate::theme::VsCodeTheme;
use egui::{RichText, Stroke};

pub fn render_output_panel(ctx: &egui::Context, state: &mut AppState) {
    if !state.settings.show_output_panel {
        return;
    }

    egui::TopBottomPanel::bottom("vscode_git_output_drawer")
        .resizable(true)
        .default_height(185.0)
        .height_range(90.0..=450.0)
        .frame(
            egui::Frame::NONE
                .fill(VsCodeTheme::SIDEBAR_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
                .inner_margin(egui::Margin::symmetric(10, 4)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("OUTPUT — Git")
                        .size(11.0)
                        .strong()
                        .color(VsCodeTheme::TEXT_BRIGHT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button(
                            RichText::new("×")
                                .size(13.0)
                                .color(VsCodeTheme::TEXT_PRIMARY),
                        )
                        .on_hover_text("Close Panel")
                        .clicked()
                    {
                        state.settings.show_output_panel = false;
                        state.settings.save();
                    }
                    if ui
                        .small_button(
                            RichText::new("Clear")
                                .size(11.0)
                                .color(VsCodeTheme::TEXT_MUTED),
                        )
                        .on_hover_text("Clear Git Output Log")
                        .clicked()
                    {
                        state.git_logs.clear();
                    }
                });
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("git_output_log_scroll")
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in &state.git_logs {
                        let status_col = if entry.exit_code == 0 {
                            VsCodeTheme::TEXT_MUTED
                        } else {
                            VsCodeTheme::ERROR_RED
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("[{}]", entry.timestamp))
                                    .monospace()
                                    .size(11.0)
                                    .color(VsCodeTheme::TEXT_DIM),
                            );
                            ui.label(
                                RichText::new(format!("> {}", entry.command_line))
                                    .monospace()
                                    .size(11.5)
                                    .color(VsCodeTheme::TEXT_PRIMARY),
                            )
                            .on_hover_text(&entry.stdout_preview);
                            ui.label(
                                RichText::new(format!(
                                    "[{}ms, exit={}]",
                                    entry.duration_ms, entry.exit_code
                                ))
                                .monospace()
                                .size(11.0)
                                .color(status_col),
                            );
                        });
                        if !entry.stderr.trim().is_empty() {
                            ui.label(
                                RichText::new(entry.stderr.trim())
                                    .monospace()
                                    .size(11.0)
                                    .color(VsCodeTheme::ERROR_RED),
                            );
                        }
                    }
                });
        });
}
