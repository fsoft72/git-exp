use crate::git::ops;
use crate::state::{AppState, QuickPickKind, QuickPickState};
use crate::theme::VsCodeTheme;
use egui::{Color32, RichText, Stroke};
use std::time::Duration;

pub fn render_status_bar(ctx: &egui::Context, state: &mut AppState) {
    egui::TopBottomPanel::bottom("vscode_status_bar")
        .exact_height(23.0)
        .frame(
            egui::Frame::NONE
                .fill(VsCodeTheme::STATUS_BAR_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
                .inner_margin(egui::Margin::symmetric(8, 2)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                if state.repo_path.is_some() {
                    // 1. Pulsante Branch (apre il Quick Pick di Checkout come in VS Code!)
                    let dirty_mark = if state.status.total_changes_count() > 0 {
                        "*"
                    } else {
                        ""
                    };
                    let branch_label = format!("⎇ {}{}", state.status.head_name, dirty_mark);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(branch_label)
                                    .size(11.5)
                                    .color(VsCodeTheme::TEXT_PRIMARY),
                            )
                            .frame(false),
                        )
                        .on_hover_text(
                            "Git Branch — Click to Checkout / Create Branch (Git: Checkout to...)",
                        )
                        .clicked()
                    {
                        state.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
                    }

                    // 2. Pulsante Sync / Publish Branch accanto al branch (come in VS Code!)
                    if state.status.upstream.is_some() {
                        let sync_lbl = format!(
                            "↻ {}↓ {}↑",
                            state.status.behind, state.status.ahead
                        );
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(sync_lbl)
                                        .size(11.5)
                                        .color(VsCodeTheme::TEXT_PRIMARY),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Synchronize Changes (Pull & Push)")
                            .clicked()
                        {
                            let br = state.status.head_name.clone();
                            state.spawn_git_task("Syncing...", move |repo| {
                                ops::sync_repo(repo, true, &br)
                                    .map(|logs| ("Sync completato".into(), logs))
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        }
                    } else if !state.status.remotes.is_empty() {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("☁ Publish Branch")
                                        .size(11.5)
                                        .color(VsCodeTheme::TEXT_PRIMARY),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Publish Branch to Remote")
                            .clicked()
                        {
                            let br = state.status.head_name.clone();
                            state.spawn_git_task("Publishing branch...", move |repo| {
                                ops::push(repo, Some(&br), false)
                                    .map(|l| (format!("Pubblicato '{}'", br), vec![l]))
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        }
                    }
                } else {
                    ui.label(
                        RichText::new("No Repository Open")
                            .size(11.5)
                            .color(VsCodeTheme::TEXT_MUTED),
                    );
                }

                // 3. Indicatore operazione in corso
                if state.is_busy {
                    ui.spinner();
                    ui.label(
                        RichText::new(&state.busy_label)
                            .size(11.0)
                            .color(VsCodeTheme::TEXT_PRIMARY),
                    );
                }

                // 4. Notifica Toast temporanea (sparisce dopo 4 secondi)
                if let Some((ref msg, ts)) = state.toast_notification {
                    if ts.elapsed() < Duration::from_secs(4) {
                        ui.label(
                            RichText::new(format!("✓ {}", msg))
                                .size(11.5)
                                .color(VsCodeTheme::GIT_ADDED),
                        );
                    }
                }

                // 5. Eventuale errore Git con pulsante di chiusura
                if let Some(err) = state.last_error.clone() {
                    let first_line = err.lines().next().unwrap_or(&err);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!("⚠ {} (×)", first_line))
                                    .size(11.5)
                                    .color(VsCodeTheme::ERROR_RED),
                            )
                            .frame(false),
                        )
                        .on_hover_text(format!("{}\n\nClick to dismiss", err))
                        .clicked()
                    {
                        state.last_error = None;
                    }
                }

                // Elementi allineati a destra nella Status Bar
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(format!("Output ({})", state.git_logs.len()))
                                    .size(11.5)
                                    .color(if state.settings.show_output_panel {
                                        Color32::WHITE
                                    } else {
                                        VsCodeTheme::TEXT_MUTED
                                    }),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Toggle Git Output Panel")
                        .clicked()
                    {
                        state.settings.show_output_panel = !state.settings.show_output_panel;
                        state.settings.save();
                    }

                    if !state.status.stashes.is_empty() {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!(
                                        "📦 Stashes: {}",
                                        state.status.stashes.len()
                                    ))
                                    .size(11.5)
                                    .color(VsCodeTheme::TEXT_MUTED),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Click to Pop Stash")
                            .clicked()
                        {
                            state.quick_pick = Some(QuickPickState::new(QuickPickKind::StashPop));
                        }
                    }

                    if let Some(ref repo) = state.repo_path {
                        let folder_name = repo
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("repo");
                        ui.label(
                            RichText::new(format!("📁 {}", folder_name))
                                .size(11.5)
                                .color(VsCodeTheme::TEXT_MUTED),
                        )
                        .on_hover_text(repo.display().to_string());
                    }
                });
            });
        });
}
