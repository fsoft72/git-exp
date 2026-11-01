use crate::git::ops;
use crate::i18n::{Language, tr};
use crate::state::{AppState, QuickPickKind, QuickPickState};
use crate::theme::{Codicon, VsCodeTheme, codicon_text_button, draw_inline_codicon};
use egui::{Color32, CornerRadius, RichText, Stroke, Vec2};
use std::time::Duration;

pub fn render_status_bar(ctx: &egui::Context, state: &mut AppState) {
    let lang = state.settings.language;
    egui::TopBottomPanel::bottom("vscode_status_bar")
        .exact_height(24.0)
        .frame(
            egui::Frame::NONE
                .fill(VsCodeTheme::STATUS_BAR_BG)
                .stroke(Stroke::new(1.0_f32, VsCodeTheme::BORDER))
                .inner_margin(egui::Margin::symmetric(8, 2)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                if state.repo_path.is_some() {
                    // 1. Pulsante Branch con icona vettoriale GitBranch
                    let dirty_mark = if state.status.total_changes_count() > 0 {
                        "*"
                    } else {
                        ""
                    };
                    let branch_label = format!("{}{}", state.status.head_name, dirty_mark);
                    if codicon_text_button(
                        ui,
                        Codicon::GitBranch,
                        &branch_label,
                        Color32::TRANSPARENT,
                        VsCodeTheme::TEXT_PRIMARY,
                        Vec2::new(0.0, 20.0),
                        CornerRadius::same(2),
                    )
                    .on_hover_text(tr(
                        lang,
                        "Git Branch - Click to Checkout / Create Branch (Git: Checkout to...)",
                        "Branch Git - Clicca per Checkout / Crea Branch (Git: Checkout to...)",
                    ))
                    .clicked()
                    {
                        state.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
                    }

                    // 2. Pulsante Sync / Publish Branch accanto al branch
                    if state.status.upstream.is_some() {
                        let sync_lbl = format!(
                            "-{} +{}",
                            state.status.behind, state.status.ahead
                        );
                        if codicon_text_button(
                            ui,
                            Codicon::Sync,
                            &sync_lbl,
                            Color32::TRANSPARENT,
                            VsCodeTheme::TEXT_PRIMARY,
                            Vec2::new(0.0, 20.0),
                            CornerRadius::same(2),
                        )
                        .on_hover_text(tr(
                            lang,
                            "Synchronize Changes (Pull & Push)",
                            "Sincronizza Modifiche (Pull & Push)",
                        ))
                        .clicked()
                        {
                            let br = state.status.head_name.clone();
                            let busy_msg = tr(lang, "Syncing...", "Sincronizzazione...");
                            let done_msg = tr(lang, "Sync completed", "Sync completato");
                            state.spawn_git_task(busy_msg, move |repo| {
                                ops::sync_repo(repo, true, &br)
                                    .map(|logs| (done_msg.into(), logs))
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        }
                    } else if !state.status.remotes.is_empty() {
                        if codicon_text_button(
                            ui,
                            Codicon::CloudUpload,
                            tr(lang, "Publish Branch", "Pubblica Branch"),
                            Color32::TRANSPARENT,
                            VsCodeTheme::TEXT_PRIMARY,
                            Vec2::new(0.0, 20.0),
                            CornerRadius::same(2),
                        )
                        .on_hover_text(tr(
                            lang,
                            "Publish Branch to Remote",
                            "Pubblica Branch sul Remote",
                        ))
                        .clicked()
                        {
                            let br = state.status.head_name.clone();
                            let busy_msg =
                                tr(lang, "Publishing branch...", "Pubblicazione branch...");
                            state.spawn_git_task(busy_msg, move |repo| {
                                ops::push(repo, Some(&br), false)
                                    .map(|l| {
                                        let msg = match lang {
                                            Language::En => format!("Published '{}'", br),
                                            Language::It => format!("Pubblicato '{}'", br),
                                        };
                                        (msg, vec![l])
                                    })
                                    .map_err(|(e, l)| (e, vec![l]))
                            });
                        }
                    }
                } else {
                    ui.label(
                        RichText::new(tr(lang, "No Repository Open", "Nessun Repository Aperto"))
                            .size(11.5)
                            .color(VsCodeTheme::TEXT_MUTED),
                    );
                }

                // 3. Indicatore operazione in corso (rispetta il flag silent per Refreshing Git status...)
                let is_refresh_msg = state.busy_label == "Refreshing Git status..."
                    || state.busy_label == "Aggiornamento stato Git...";
                let show_busy =
                    state.is_busy && (!is_refresh_msg || state.settings.show_refresh_status_message);
                if show_busy {
                    ui.spinner();
                    ui.label(
                        RichText::new(&state.busy_label)
                            .size(11.0)
                            .color(VsCodeTheme::TEXT_PRIMARY),
                    );
                }

                // 4. Notifica Toast temporanea
                if let Some((ref msg, ts)) = state.toast_notification {
                    if ts.elapsed() < Duration::from_secs(4) {
                        draw_inline_codicon(ui, Codicon::Check, 14.0, VsCodeTheme::GIT_ADDED);
                        ui.label(
                            RichText::new(msg)
                                .size(11.5)
                                .color(VsCodeTheme::GIT_ADDED),
                        );
                    }
                }

                // 5. Eventuale errore Git con pulsante di chiusura
                if let Some(err) = state.last_error.clone() {
                    let first_line = err.lines().next().unwrap_or(&err);
                    if codicon_text_button(
                        ui,
                        Codicon::Warning,
                        first_line,
                        Color32::TRANSPARENT,
                        VsCodeTheme::ERROR_RED,
                        Vec2::new(0.0, 20.0),
                        CornerRadius::same(2),
                    )
                    .on_hover_text(format!(
                        "{}\n\n{}",
                        err,
                        tr(lang, "Click to dismiss", "Clicca per chiudere")
                    ))
                    .clicked()
                    {
                        state.last_error = None;
                    }
                }

                // Elementi allineati a destra nella Status Bar
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

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
                        .on_hover_text(tr(
                            lang,
                            "Toggle Git Output Panel",
                            "Mostra/Nascondi Pannello Output Git",
                        ))
                        .clicked()
                    {
                        state.settings.show_output_panel = !state.settings.show_output_panel;
                        state.settings.save();
                    }

                    if !state.status.stashes.is_empty() {
                        if codicon_text_button(
                            ui,
                            Codicon::Archive,
                            &format!(
                                "{}: {}",
                                tr(lang, "Stashes", "Stash"),
                                state.status.stashes.len()
                            ),
                            Color32::TRANSPARENT,
                            VsCodeTheme::TEXT_MUTED,
                            Vec2::new(0.0, 20.0),
                            CornerRadius::same(2),
                        )
                        .on_hover_text(tr(
                            lang,
                            "Click to Pop Stash",
                            "Clicca per applicare (Pop) lo Stash",
                        ))
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
                            RichText::new(folder_name)
                                .size(11.5)
                                .color(VsCodeTheme::TEXT_MUTED),
                        )
                        .on_hover_text(repo.display().to_string());
                        draw_inline_codicon(ui, Codicon::Folder, 14.0, VsCodeTheme::TEXT_MUTED);
                    }
                });
            });
        });
}
