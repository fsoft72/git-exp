use crate::git::ops::{self, CommitVariant};
use crate::i18n::{Language, tr};
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
    let lang = state.settings.language;
    let title = match &qp.kind {
        QuickPickKind::CommandPalette => {
            tr(lang, "Git Command Palette", "Palette Comandi Git").to_string()
        }
        QuickPickKind::Checkout => tr(
            lang,
            "Git: Checkout to... (Select a ref or type a new branch name)",
            "Git: Checkout su... (Seleziona un ref o digita il nome di un nuovo branch)",
        )
        .to_string(),
        QuickPickKind::CreateBranch { from_ref: None } => match lang {
            Language::En => format!(
                "Git: Create Branch from '{}' (Enter new branch name and press Enter)",
                state.status.head_name
            ),
            Language::It => format!(
                "Git: Crea Branch da '{}' (Inserisci il nome del nuovo branch e premi Invio)",
                state.status.head_name
            ),
        },
        QuickPickKind::CreateBranch {
            from_ref: Some(r),
        } => match lang {
            Language::En => format!(
                "Git: Create Branch from '{}' (Enter branch name and press Enter)",
                r
            ),
            Language::It => format!(
                "Git: Crea Branch da '{}' (Inserisci il nome del branch e premi Invio)",
                r
            ),
        },
        QuickPickKind::CreateBranchFrom { preset_name: None } => tr(
            lang,
            "Git: Create Branch From... (Select base branch or tag)",
            "Git: Crea Branch Da... (Seleziona branch o tag di base)",
        )
        .to_string(),
        QuickPickKind::CreateBranchFrom {
            preset_name: Some(name),
        } => match lang {
            Language::En => format!(
                "Git: Create Branch '{}' From... (Select base branch or tag)",
                name
            ),
            Language::It => format!(
                "Git: Crea Branch '{}' Da... (Seleziona branch o tag di base)",
                name
            ),
        },
        QuickPickKind::DeleteBranch { .. } => tr(
            lang,
            "Git: Delete Branch (Select a branch to delete)",
            "Git: Elimina Branch (Seleziona un branch da eliminare)",
        )
        .to_string(),
        QuickPickKind::RenameBranch => tr(
            lang,
            "Git: Rename Current Branch (Enter new name)",
            "Git: Rinomina Branch Corrente (Inserisci il nuovo nome)",
        )
        .to_string(),
        QuickPickKind::MergeBranch => tr(
            lang,
            "Git: Merge Branch into current branch",
            "Git: Esegui Merge di un Branch nel branch corrente",
        )
        .to_string(),
        QuickPickKind::RebaseBranch => tr(
            lang,
            "Git: Rebase Current Branch onto...",
            "Git: Esegui Rebase del Branch Corrente su...",
        )
        .to_string(),
        QuickPickKind::StashPush {
            include_untracked: false,
        } => tr(
            lang,
            "Git: Stash Changes (Optional message, press Enter to stash)",
            "Git: Salva Modifiche nello Stash (Messaggio opzionale, premi Invio)",
        )
        .to_string(),
        QuickPickKind::StashPush {
            include_untracked: true,
        } => tr(
            lang,
            "Git: Stash Including Untracked (Optional message, press Enter)",
            "Git: Salva nello Stash Includendo Non Tracciati (Messaggio opzionale, premi Invio)",
        )
        .to_string(),
        QuickPickKind::StashPop => tr(
            lang,
            "Git: Pop Stash (Select a stash to pop)",
            "Git: Pop Stash (Seleziona uno stash da estrarre)",
        )
        .to_string(),
        QuickPickKind::StashApply => tr(
            lang,
            "Git: Apply Stash (Select a stash to apply)",
            "Git: Applica Stash (Seleziona uno stash da applicare)",
        )
        .to_string(),
        QuickPickKind::StashDrop => tr(
            lang,
            "Git: Drop Stash (Select a stash to delete)",
            "Git: Elimina Stash (Seleziona uno stash da eliminare)",
        )
        .to_string(),
        QuickPickKind::CreateTag => tr(
            lang,
            "Git: Create Tag at HEAD (Enter tag name and press Enter)",
            "Git: Crea Tag su HEAD (Inserisci il nome del tag e premi Invio)",
        )
        .to_string(),
        QuickPickKind::DeleteTag => tr(
            lang,
            "Git: Delete Tag (Select a tag to delete)",
            "Git: Elimina Tag (Seleziona un tag da eliminare)",
        )
        .to_string(),
        QuickPickKind::ConfirmDiscardAll => tr(
            lang,
            "Discard ALL working tree & untracked changes? (Cannot be undone)",
            "Annullare TUTTE le modifiche nel working tree e non tracciate? (Irreversibile)",
        )
        .to_string(),
        QuickPickKind::ConfirmDiscardFile { path, .. } => match lang {
            Language::En => format!("Discard changes in '{}'?", path),
            Language::It => format!("Annullare le modifiche in '{}'?", path),
        },
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&title)
                .size(11.5)
                .strong()
                .color(VsCodeTheme::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if codicon_button(ui, Codicon::Close, tr(lang, "Close (Esc)", "Chiudi (Esc)")) {
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
                QuickPickKind::CreateBranch { .. } => tr(
                    lang,
                    "New branch name (e.g. feat/new-ui)...",
                    "Nome del nuovo branch (es. feat/nuova-ui)...",
                ),
                QuickPickKind::CreateBranchFrom { .. } => tr(
                    lang,
                    "Select base branch or tag...",
                    "Seleziona branch o tag di base...",
                ),
                QuickPickKind::RenameBranch => {
                    tr(lang, "New branch name...", "Nuovo nome del branch...")
                }
                QuickPickKind::StashPush { .. } => tr(
                    lang,
                    "Optional stash message (press Enter)...",
                    "Messaggio stash opzionale (premi Invio)...",
                ),
                QuickPickKind::CreateTag => {
                    tr(lang, "Tag name (e.g. v0.2.0)...", "Nome del tag (es. v0.2.0)...")
                }
                _ => tr(
                    lang,
                    "Type to filter commands or branches...",
                    "Digita per filtrare comandi o branch...",
                ),
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
            current_qp.scroll_to_selected = true;
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
                QuickPickKind::CreateBranch { .. } => tr(
                    lang,
                    "Type the new branch name above and press Enter to create and checkout.",
                    "Digita il nome del nuovo branch sopra e premi Invio per crearlo e attivarlo.",
                ),
                QuickPickKind::RenameBranch => tr(
                    lang,
                    "Type the new branch name above and press Enter to rename.",
                    "Digita il nuovo nome del branch sopra e premi Invio per rinominarlo.",
                ),
                QuickPickKind::CreateTag => tr(
                    lang,
                    "Type the tag name above and press Enter to create the tag.",
                    "Digita il nome del tag sopra e premi Invio per crearlo.",
                ),
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
        let down_pressed = ui.input(|i| {
            i.key_pressed(Key::ArrowDown)
                || i.events.iter().any(|ev| {
                    matches!(
                        ev,
                        egui::Event::Key {
                            key: Key::ArrowDown,
                            pressed: true,
                            ..
                        }
                    )
                })
        });
        let up_pressed = ui.input(|i| {
            i.key_pressed(Key::ArrowUp)
                || i.events.iter().any(|ev| {
                    matches!(
                        ev,
                        egui::Event::Key {
                            key: Key::ArrowUp,
                            pressed: true,
                            ..
                        }
                    )
                })
        });
        let page_down = ui.input(|i| i.key_pressed(Key::PageDown));
        let page_up = ui.input(|i| i.key_pressed(Key::PageUp));

        let mut changed = false;
        if down_pressed {
            selected_idx = (selected_idx + 1) % item_count;
            changed = true;
        }
        if up_pressed {
            selected_idx = if selected_idx == 0 {
                item_count - 1
            } else {
                selected_idx - 1
            };
            changed = true;
        }
        if page_down {
            selected_idx = (selected_idx + 8).min(item_count - 1);
            changed = true;
        }
        if page_up {
            selected_idx = selected_idx.saturating_sub(8);
            changed = true;
        }

        if changed {
            if let Some(ref mut current_qp) = state.quick_pick {
                current_qp.selected_index = selected_idx;
                current_qp.scroll_to_selected = true;
            }
            ui.ctx().request_repaint();
        }
    }

    let should_scroll = state
        .quick_pick
        .as_ref()
        .map(|q| q.scroll_to_selected)
        .unwrap_or(false);

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

                if is_sel && should_scroll {
                    resp.scroll_to_me(None);
                }

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

    if should_scroll {
        if let Some(ref mut current_qp) = state.quick_pick {
            current_qp.scroll_to_selected = false;
        }
    }

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
    let lang = state.settings.language;
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
                let label = match lang {
                    Language::En => format!("Create and checkout branch '{}'", name),
                    Language::It => format!("Crea e attiva il branch '{}'", name),
                };
                let detail = match lang {
                    Language::En => format!("from '{}'", base_label),
                    Language::It => format!("da '{}'", base_label),
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label,
                    detail,
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Creating branch...", "Creazione branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::create_branch(repo, &name, base.as_deref(), true)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => {
                                            format!("Created and checked out branch '{}'", name)
                                        }
                                        Language::It => {
                                            format!("Creato e attivato branch '{}'", name)
                                        }
                                    };
                                    (msg, vec![l])
                                })
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
                let label = match lang {
                    Language::En => format!(
                        "Rename branch '{}' to '{}'",
                        state.status.head_name, name
                    ),
                    Language::It => format!(
                        "Rinomina branch '{}' in '{}'",
                        state.status.head_name, name
                    ),
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::GitBranch),
                    label,
                    detail: "git branch -m".to_string(),
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Renaming branch...", "Rinomina branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::rename_branch(repo, &name)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Renamed branch to '{}'", name),
                                        Language::It => format!("Branch rinominato in '{}'", name),
                                    };
                                    (msg, vec![l])
                                })
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
            let label = match (&msg_opt, lang) {
                (Some(m), Language::En) => format!("Stash changes with message '{}'", m),
                (Some(m), Language::It) => format!("Salva nello stash con messaggio '{}'", m),
                (None, Language::En) => "Stash changes (default message)".to_string(),
                (None, Language::It) => "Salva modifiche nello stash (messaggio predefinito)".to_string(),
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
                    let busy_msg = tr(lang, "Stashing changes...", "Salvataggio nello stash...");
                    let done_msg = tr(
                        lang,
                        "Changes saved to Stash",
                        "Modifiche salvate nello Stash",
                    );
                    st.spawn_git_task(busy_msg, move |repo| {
                        ops::stash_push(repo, msg_opt.as_deref(), inc)
                            .map(|l| (done_msg.into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            return raw;
        }
        QuickPickKind::CreateTag => {
            if !trimmed.is_empty() {
                let tag_name = trimmed.to_string();
                let label = match lang {
                    Language::En => format!("Create tag '{}' at HEAD", tag_name),
                    Language::It => format!("Crea tag '{}' su HEAD", tag_name),
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Tag),
                    label,
                    detail: "git tag".to_string(),
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Creating tag...", "Creazione tag...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::create_tag(repo, &tag_name, None)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Tag '{}' created", tag_name),
                                        Language::It => format!("Tag '{}' creato", tag_name),
                                    };
                                    (msg, vec![l])
                                })
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
                            let busy_msg = tr(lang, "Creating branch...", "Creazione branch...");
                            st.spawn_git_task(busy_msg, move |repo| {
                                ops::create_branch(repo, &new_name, Some(&base_ref), true)
                                    .map(|l| {
                                        let msg = match lang {
                                            Language::En => format!(
                                                "Created and checked out branch '{}' from '{}'",
                                                new_name, base_ref
                                            ),
                                            Language::It => format!(
                                                "Creato e attivato branch '{}' da '{}'",
                                                new_name, base_ref
                                            ),
                                        };
                                        (msg, vec![l])
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
                            let busy_msg = tr(lang, "Creating branch...", "Creazione branch...");
                            st.spawn_git_task(busy_msg, move |repo| {
                                ops::create_branch(repo, &new_name, Some(&base_ref), true)
                                    .map(|l| {
                                        let msg = match lang {
                                            Language::En => format!(
                                                "Created and checked out branch '{}' from '{}'",
                                                new_name, base_ref
                                            ),
                                            Language::It => format!(
                                                "Creato e attivato branch '{}' da '{}'",
                                                new_name, base_ref
                                            ),
                                        };
                                        (msg, vec![l])
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
                    label: tr(lang, "Create new branch...", "Crea nuovo branch...").to_string(),
                    detail: match lang {
                        Language::En => format!("from '{}'", state.status.head_name),
                        Language::It => format!("da '{}'", state.status.head_name),
                    },
                    action: Box::new(|st| {
                        st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                            from_ref: None,
                        }));
                    }),
                });
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: tr(
                        lang,
                        "Create new branch from...",
                        "Crea nuovo branch da...",
                    )
                    .to_string(),
                    detail: tr(
                        lang,
                        "Select base branch or tag",
                        "Seleziona branch o tag di base",
                    )
                    .to_string(),
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
                        let busy_msg = tr(lang, "Checking out branch...", "Checkout del branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::checkout_branch(repo, &name, is_rem)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Switched to '{}'", name),
                                        Language::It => format!("Attivato '{}'", name),
                                    };
                                    (msg, vec![l])
                                })
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
                        let busy_msg = tr(lang, "Checking out tag...", "Checkout del tag...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::checkout_branch(repo, &tname, false)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Checked out tag '{}'", tname),
                                        Language::It => format!("Checkout su tag '{}'", tname),
                                    };
                                    (msg, vec![l])
                                })
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
                    label: match lang {
                        Language::En => format!("Create new branch '{}'", new_br_name),
                        Language::It => format!("Crea nuovo branch '{}'", new_br_name),
                    },
                    detail: match lang {
                        Language::En => format!("from '{}' (git checkout -b)", head_name),
                        Language::It => format!("da '{}' (git checkout -b)", head_name),
                    },
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Creating branch...", "Creazione branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::create_branch(repo, &new_br_name, None, true)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!(
                                            "Created and checked out branch '{}'",
                                            new_br_name
                                        ),
                                        Language::It => {
                                            format!("Creato e attivato branch '{}'", new_br_name)
                                        }
                                    };
                                    (msg, vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                };

                let create_from_item = QuickPickItem {
                    icon: Some(Codicon::Plus),
                    label: match lang {
                        Language::En => format!("Create new branch '{}' from...", new_br_name_from),
                        Language::It => format!("Crea nuovo branch '{}' da...", new_br_name_from),
                    },
                    detail: tr(
                        lang,
                        "Select base branch or tag",
                        "Seleziona branch o tag di base",
                    )
                    .to_string(),
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
                        let busy_msg = tr(lang, "Deleting branch...", "Eliminazione branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::delete_branch(repo, &name, f)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Branch '{}' deleted", name),
                                        Language::It => format!("Branch '{}' eliminato", name),
                                    };
                                    (msg, vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::MergeBranch => {
            for br in state.status.branches.iter().filter(|b| !b.is_current) {
                let name = br.name.clone();
                let label = match lang {
                    Language::En => format!("Merge '{}' into '{}'", name, state.status.head_name),
                    Language::It => {
                        format!("Merge di '{}' in '{}'", name, state.status.head_name)
                    }
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::GitBranch),
                    label,
                    detail: br.commit_short.clone(),
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Merging branch...", "Merge del branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::merge_branch(repo, &name)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Merge of '{}' completed", name),
                                        Language::It => format!("Merge di '{}' completato", name),
                                    };
                                    (msg, vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::RebaseBranch => {
            for br in state.status.branches.iter().filter(|b| !b.is_current) {
                let name = br.name.clone();
                let label = match lang {
                    Language::En => format!("Rebase '{}' onto '{}'", state.status.head_name, name),
                    Language::It => {
                        format!("Rebase di '{}' su '{}'", state.status.head_name, name)
                    }
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::GitBranch),
                    label,
                    detail: br.commit_short.clone(),
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Rebasing branch...", "Rebase del branch...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::rebase_branch(repo, &name)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Rebase onto '{}' completed", name),
                                        Language::It => format!("Rebase su '{}' completato", name),
                                    };
                                    (msg, vec![l])
                                })
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
                        let busy_msg = tr(
                            lang,
                            "Running stash command...",
                            "Esecuzione comando stash...",
                        );
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::stash_action(repo, &verb, &r)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Stash {} ({}) completed", verb, r),
                                        Language::It => format!("Stash {} ({}) completato", verb, r),
                                    };
                                    (msg, vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::DeleteTag => {
            for tag in &state.status.tags {
                let tname = tag.name.clone();
                let label = match lang {
                    Language::En => format!("Delete tag '{}'", tname),
                    Language::It => format!("Elimina tag '{}'", tname),
                };
                raw.push(QuickPickItem {
                    icon: Some(Codicon::Trash),
                    label,
                    detail: tag.commit_short.clone(),
                    action: Box::new(move |st| {
                        let busy_msg = tr(lang, "Deleting tag...", "Eliminazione tag...");
                        st.spawn_git_task(busy_msg, move |repo| {
                            ops::delete_tag(repo, &tname)
                                .map(|l| {
                                    let msg = match lang {
                                        Language::En => format!("Tag '{}' deleted", tname),
                                        Language::It => format!("Tag '{}' eliminato", tname),
                                    };
                                    (msg, vec![l])
                                })
                                .map_err(|(e, l)| (e, vec![l]))
                        });
                    }),
                });
            }
        }
        QuickPickKind::ConfirmDiscardAll => {
            raw.push(QuickPickItem {
                icon: Some(Codicon::Discard),
                label: tr(
                    lang,
                    "Yes, Discard All Changes (git checkout -- . && git clean -fd)",
                    "Sì, Annulla Tutte le Modifiche (git checkout -- . && git clean -fd)",
                )
                .into(),
                detail: tr(lang, "Destructive", "Distruttivo").into(),
                action: Box::new(|st| st.discard_all_confirmed()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Close),
                label: tr(lang, "Cancel", "Annulla").into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::ConfirmDiscardFile { path, kind } => {
            let p = path.clone();
            let k = *kind;
            let label = match lang {
                Language::En => format!("Yes, Discard Changes in '{}'", p),
                Language::It => format!("Sì, Annulla le Modifiche in '{}'", p),
            };
            raw.push(QuickPickItem {
                icon: Some(Codicon::Discard),
                label,
                detail: tr(lang, "Destructive", "Distruttivo").into(),
                action: Box::new(move |st| st.discard_file_confirmed(p, k)),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Close),
                label: tr(lang, "Cancel", "Annulla").into(),
                detail: "Esc".into(),
                action: Box::new(|_| {}),
            });
        }
        QuickPickKind::CommandPalette => {
            raw.push(QuickPickItem {
                icon: Some(Codicon::GitBranch),
                label: tr(lang, "Git: Checkout to...", "Git: Checkout su...").into(),
                detail: tr(lang, "Switch branch or tag", "Cambia branch o tag").into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::Checkout));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Plus),
                label: tr(lang, "Git: Create Branch...", "Git: Crea Branch...").into(),
                detail: "git checkout -b".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranch {
                        from_ref: None,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Plus),
                label: tr(
                    lang,
                    "Git: Create Branch From...",
                    "Git: Crea Branch Da...",
                )
                .into(),
                detail: tr(
                    lang,
                    "Select base branch/tag",
                    "Seleziona branch/tag di base",
                )
                .into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateBranchFrom {
                        preset_name: None,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Check),
                label: "Git: Commit".into(),
                detail: tr(lang, "Ctrl+Enter", "Ctrl+Invio").into(),
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
                label: tr(lang, "Git: Sync", "Git: Sincronizza (Sync)").into(),
                detail: "Pull & Push".into(),
                action: Box::new(move |st| {
                    let has_up = st.status.upstream.is_some();
                    let br = st.status.head_name.clone();
                    let busy_msg = tr(lang, "Syncing...", "Sincronizzazione...");
                    let done_msg = tr(lang, "Sync completed", "Sync completato");
                    st.spawn_git_task(busy_msg, move |repo| {
                        ops::sync_repo(repo, has_up, &br)
                            .map(|logs| (done_msg.into(), logs))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::ArrowDown),
                label: "Git: Pull".into(),
                detail: "git pull".into(),
                action: Box::new(move |st| {
                    let busy_msg = tr(lang, "Pulling...", "Pull in corso...");
                    let done_msg = tr(lang, "Pull completed", "Pull completato");
                    st.spawn_git_task(busy_msg, move |repo| {
                        ops::pull(repo, false)
                            .map(|l| (done_msg.into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::ArrowUp),
                label: "Git: Push".into(),
                detail: "git push".into(),
                action: Box::new(move |st| {
                    let has_up = st.status.upstream.is_some();
                    let br = st.status.head_name.clone();
                    let busy_msg = tr(lang, "Pushing...", "Push in corso...");
                    let done_msg = tr(lang, "Push completed", "Push completato");
                    st.spawn_git_task(busy_msg, move |repo| {
                        let up = if has_up { None } else { Some(br.as_str()) };
                        ops::push(repo, up, false)
                            .map(|l| (done_msg.into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Refresh),
                label: "Git: Fetch All (Prune)".into(),
                detail: "git fetch --all --prune".into(),
                action: Box::new(move |st| {
                    let busy_msg = tr(lang, "Fetching...", "Fetch in corso...");
                    let done_msg = tr(lang, "Fetch completed", "Fetch completato");
                    st.spawn_git_task(busy_msg, move |repo| {
                        ops::fetch_all(repo)
                            .map(|l| (done_msg.into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Plus),
                label: tr(
                    lang,
                    "Git: Stage All Changes",
                    "Git: Aggiungi Tutto allo Stage",
                )
                .into(),
                detail: "git add -A".into(),
                action: Box::new(|st| st.stage_all()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Minus),
                label: tr(
                    lang,
                    "Git: Unstage All Changes",
                    "Git: Rimuovi Tutto dallo Stage",
                )
                .into(),
                detail: "git reset HEAD".into(),
                action: Box::new(|st| st.unstage_all()),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Archive),
                label: tr(
                    lang,
                    "Git: Stash Changes...",
                    "Git: Salva Modifiche nello Stash...",
                )
                .into(),
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
                label: tr(lang, "Git: Delete Branch...", "Git: Elimina Branch...").into(),
                detail: "git branch -d".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::DeleteBranch {
                        force: false,
                    }));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Tag),
                label: tr(lang, "Git: Create Tag...", "Git: Crea Tag...").into(),
                detail: "git tag".into(),
                action: Box::new(|st| {
                    st.quick_pick = Some(QuickPickState::new(QuickPickKind::CreateTag));
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::Discard),
                label: tr(
                    lang,
                    "Git: Undo Last Commit",
                    "Git: Annulla Ultimo Commit",
                )
                .into(),
                detail: "git reset --soft HEAD~1".into(),
                action: Box::new(move |st| {
                    let busy_msg = tr(
                        lang,
                        "Undoing last commit...",
                        "Annullamento ultimo commit...",
                    );
                    let done_msg = tr(lang, "Last commit undone", "Ultimo commit annullato");
                    st.spawn_git_task(busy_msg, move |repo| {
                        ops::undo_last_commit(repo)
                            .map(|l| (done_msg.into(), vec![l]))
                            .map_err(|(e, l)| (e, vec![l]))
                    });
                }),
            });
            raw.push(QuickPickItem {
                icon: Some(Codicon::FileCode),
                label: tr(lang, "Git: Show Git Output", "Git: Mostra Output Git").into(),
                detail: tr(
                    lang,
                    "Toggle Output Panel",
                    "Mostra/Nascondi Pannello Output",
                )
                .into(),
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
        state.settings.language = Language::En;
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
        state.settings.language = Language::En;
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

    #[test]
    fn test_multilanguage_default_en_and_it_translation() {
        let default_settings = crate::settings::UserSettings::default();
        assert_eq!(default_settings.language, Language::En);

        let mut state = AppState::new(None);
        state.settings.language = Language::It;
        state.status.head_name = "dev".to_string();

        let items = build_quick_pick_items(
            &state,
            &QuickPickKind::CreateBranch {
                from_ref: Some("dev".to_string()),
            },
            "feat/nuovo",
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "Crea e attiva il branch 'feat/nuovo'");
        assert_eq!(items[0].detail, "da 'dev'");
    }
}
