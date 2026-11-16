use crate::i18n::tr;
use crate::state::AppState;
use crate::theme::VsCodeTheme;
use crate::ui::graph_view::{draw_graph_cell, graph_column_width, max_lane_of};
use egui::{Align2, CornerRadius, FontFamily, FontId, Rect, RichText, Sense, Vec2, pos2};

/// Height of a commit row in the details table.
const ROW_HEIGHT: f32 = 22.0;
/// Fixed widths of the right aligned columns.
const DATE_COL_W: f32 = 132.0;
const AUTHOR_COL_W: f32 = 96.0;
const COMMIT_COL_W: f32 = 64.0;
/// Horizontal padding inside the table.
const PAD: f32 = 10.0;

/// Formats git's `%ci` date (`2026-10-13 13:25:07 +0200`) as `YYYY-MM-DD HH:MM`.
fn format_date(iso_date: &str) -> &str {
    iso_date.get(..16).unwrap_or(iso_date)
}

/// Renders the Graph Details tab content: toolbar plus the commits table.
pub fn render_details_tab(ui: &mut egui::Ui, state: &mut AppState) {
    render_toolbar(ui, state);
    ui.separator();
    render_table(ui, state);
}

/// Top bar: branch selector and the "show remote branches" flag.
fn render_toolbar(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.settings.language;
    let mut changed = false;

    // Without the remote flag a previously chosen remote branch is no longer selectable
    let selected = state.details_effective_branch();
    let branches: Vec<String> = state
        .status
        .branches
        .iter()
        .filter(|b| state.details_show_remote || !b.is_remote)
        .map(|b| b.name.clone())
        .collect();

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            RichText::new("Branch:")
                .size(12.0)
                .color(VsCodeTheme::TEXT_PRIMARY),
        );
        egui::ComboBox::from_id_salt("graph_details_branch")
            .width(220.0)
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for name in &branches {
                    if ui.selectable_label(*name == selected, name).clicked() {
                        state.details_branch = Some(name.clone());
                        changed = true;
                    }
                }
            });
        ui.add_space(8.0);
        let mut remote = state.details_show_remote;
        if ui
            .checkbox(
                &mut remote,
                tr(lang, "Show Remote Branches", "Mostra branch remoti"),
            )
            .changed()
        {
            state.details_show_remote = remote;
            // Fall back to the current branch if the selected remote branch is now hidden
            if !remote {
                let is_remote_sel = state
                    .status
                    .branches
                    .iter()
                    .any(|b| b.is_remote && Some(&b.name) == state.details_branch.as_ref());
                if is_remote_sel {
                    state.details_branch = None;
                }
            }
            changed = true;
        }
    });
    ui.add_space(6.0);

    if changed {
        state.reload_graph_details();
    }
}

/// Column header plus the virtualized list of commits (description, date, author, commit id).
fn render_table(ui: &mut egui::Ui, state: &mut AppState) {
    let lang = state.settings.language;
    let font = FontId::new(12.0, FontFamily::Proportional);
    let mono = FontId::new(12.0, FontFamily::Monospace);

    // Header row
    let (hdr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let graph_w = graph_column_width(max_lane_of(&state.details_commits));
    let cols = column_rects(hdr, graph_w);
    let hdr_font = FontId::new(11.5, FontFamily::Proportional);
    let hdr_painter = ui.painter();
    for (rect, label) in [
        (cols.graph, "Graph"),
        (cols.subject, tr(lang, "Description", "Descrizione")),
        (cols.date, tr(lang, "Date", "Data")),
        (cols.author, tr(lang, "Author", "Autore")),
        (cols.commit, tr(lang, "Commit", "Commit")),
    ] {
        hdr_painter.with_clip_rect(rect).text(
            pos2(rect.left(), rect.center().y),
            Align2::LEFT_CENTER,
            label,
            hdr_font.clone(),
            VsCodeTheme::TEXT_MUTED,
        );
    }
    ui.separator();

    if state.details_commits.is_empty() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(PAD);
            ui.label(
                RichText::new(tr(lang, "No commits to show", "Nessun commit da mostrare"))
                    .size(12.0)
                    .italics()
                    .color(VsCodeTheme::TEXT_MUTED),
            );
        });
        return;
    }

    let total = state.details_commits.len();
    let mut to_open: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("graph_details_scroll")
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, total, |ui, range| {
            let start = range.start;
            for (offset, commit) in state.details_commits[range].iter().enumerate() {
                let prev = (start + offset)
                    .checked_sub(1)
                    .map(|i| &state.details_commits[i]);
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ROW_HEIGHT),
                    Sense::click(),
                );
                let selected = state.selected_commit_sha.as_deref() == Some(commit.sha.as_str());
                if selected {
                    ui.painter().rect_filled(
                        rect,
                        CornerRadius::ZERO,
                        VsCodeTheme::LIST_ACTIVE_SELECTION_BG,
                    );
                } else if resp.hovered() {
                    ui.painter()
                        .rect_filled(rect, CornerRadius::ZERO, VsCodeTheme::LIST_HOVER_BG);
                }

                let cols = column_rects(rect, graph_w);
                let painter = ui.painter();
                draw_graph_cell(&painter.with_clip_rect(cols.graph), rect, prev, commit);
                let cells = [
                    (cols.subject, commit.subject.as_str(), &font, VsCodeTheme::TEXT_PRIMARY),
                    (cols.date, format_date(&commit.iso_date), &font, VsCodeTheme::TEXT_MUTED),
                    (cols.author, commit.author.as_str(), &font, VsCodeTheme::TEXT_MUTED),
                    (cols.commit, commit.short_sha.as_str(), &mono, VsCodeTheme::ACCENT_BLUE_HOVER),
                ];
                for (cell, text, f, color) in cells {
                    painter.with_clip_rect(cell).text(
                        pos2(cell.left(), cell.center().y),
                        Align2::LEFT_CENTER,
                        text,
                        f.clone(),
                        color,
                    );
                }
                if resp.clicked() {
                    to_open = Some(commit.sha.clone());
                }
            }
        });

    if let Some(sha) = to_open {
        state.open_commit_inspector(&sha);
    }
}

/// Column rectangles of a table row: description takes the remaining width, the others are fixed.
struct Columns {
    graph: Rect,
    subject: Rect,
    date: Rect,
    author: Rect,
    commit: Rect,
}

fn column_rects(row: Rect, graph_w: f32) -> Columns {
    let right = row.right() - PAD;
    let commit_l = right - COMMIT_COL_W;
    let author_l = commit_l - AUTHOR_COL_W;
    let date_l = author_l - DATE_COL_W;
    let (top, bot) = (row.top(), row.bottom());
    let cell = |l: f32, r: f32| Rect::from_min_max(pos2(l, top), pos2(r - 6.0, bot));
    let graph = Rect::from_min_max(row.left_top(), pos2(row.left() + graph_w, bot));
    Columns {
        graph,
        subject: cell(graph.right(), date_l.max(graph.right() + 40.0)),
        date: cell(date_l, author_l),
        author: cell(author_l, commit_l),
        commit: Rect::from_min_max(pos2(commit_l, top), pos2(right, bot)),
    }
}
