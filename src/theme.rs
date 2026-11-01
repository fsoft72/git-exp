use egui::{
    Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Style,
    TextStyle, Vec2, Visuals,
};

/// Palette fedele a Visual Studio Code "Dark Modern" + estensione Git nativa.
pub struct VsCodeTheme;

#[allow(dead_code)]
impl VsCodeTheme {
    // --- Superfici principali ---
    pub const ACTIVITY_BAR_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const SIDEBAR_HEADER_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const EDITOR_BG: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
    pub const EDITOR_TAB_BAR_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const EDITOR_TAB_ACTIVE_BG: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
    pub const EDITOR_TAB_INACTIVE_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const STATUS_BAR_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const QUICK_PICK_BG: Color32 = Color32::from_rgb(0x25, 0x25, 0x26);

    // --- Bordi e separatori ---
    pub const BORDER: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
    pub const FOCUS_BORDER: Color32 = Color32::from_rgb(0x00, 0x78, 0xD4);
    pub const INPUT_BG: Color32 = Color32::from_rgb(0x31, 0x31, 0x31);
    pub const INPUT_BORDER: Color32 = Color32::from_rgb(0x3C, 0x3C, 0x3C);

    // --- Selezione e Hover nelle liste ---
    pub const LIST_HOVER_BG: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x2E);
    pub const LIST_ACTIVE_SELECTION_BG: Color32 = Color32::from_rgb(0x04, 0x39, 0x5E);
    pub const LIST_INACTIVE_SELECTION_BG: Color32 = Color32::from_rgb(0x37, 0x37, 0x3D);

    // --- Bottoni primari (Commit / Sync) ---
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x00, 0x78, 0xD4);
    pub const ACCENT_BLUE_HOVER: Color32 = Color32::from_rgb(0x02, 0x6E, 0xC1);
    pub const BADGE_BG: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);
    pub const ACTIVITY_BADGE_BG: Color32 = Color32::from_rgb(0x00, 0x78, 0xD4);

    // --- Testo ---
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xCC, 0xCC, 0xCC);
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x85, 0x85, 0x85);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);

    // --- Colori Git ufficiali di VS Code (gitDecoration.*) ---
    pub const GIT_MODIFIED: Color32 = Color32::from_rgb(0xE2, 0xC0, 0x8D);
    pub const GIT_ADDED: Color32 = Color32::from_rgb(0x73, 0xC9, 0x91);
    pub const GIT_UNTRACKED: Color32 = Color32::from_rgb(0x73, 0xC9, 0x91);
    pub const GIT_DELETED: Color32 = Color32::from_rgb(0xC7, 0x4E, 0x39);
    pub const GIT_CONFLICT: Color32 = Color32::from_rgb(0xE4, 0x67, 0x6B);
    pub const GIT_IGNORED: Color32 = Color32::from_rgb(0x8C, 0x8C, 0x8C);

    // --- Colori Diff Viewer (diffEditor.*) ---
    pub const DIFF_INSERT_LINE_BG: Color32 = Color32::from_rgba_premultiplied(35, 78, 42, 95);
    pub const DIFF_DELETE_LINE_BG: Color32 = Color32::from_rgba_premultiplied(90, 30, 30, 95);
    pub const DIFF_INSERT_GUTTER_BG: Color32 = Color32::from_rgba_premultiplied(46, 160, 67, 70);
    pub const DIFF_DELETE_GUTTER_BG: Color32 = Color32::from_rgba_premultiplied(248, 81, 73, 70);
    pub const DIFF_HUNK_HEADER_BG: Color32 = Color32::from_rgb(0x26, 0x29, 0x2E);
    pub const DIFF_EMPTY_FILL_BG: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
    pub const DIFF_SELECTED_LINE_BG: Color32 = Color32::from_rgba_premultiplied(0, 120, 212, 75);

    // --- Avvisi / Errori ---
    pub const WARNING_YELLOW: Color32 = Color32::from_rgb(0xCC, 0xA7, 0x00);
    pub const ERROR_RED: Color32 = Color32::from_rgb(0xF1, 0x4C, 0x4C);

    // --- Corsie del Source Control Graph ---
    pub const GRAPH_LANES: [Color32; 6] = [
        Color32::from_rgb(0xFF, 0xB0, 0x00), // Oro
        Color32::from_rgb(0xDC, 0x26, 0x7F), // Magenta
        Color32::from_rgb(0x99, 0x4F, 0x00), // Ambra scuro
        Color32::from_rgb(0x40, 0xB0, 0xA6), // Teal
        Color32::from_rgb(0xB6, 0x6D, 0xFF), // Viola
        Color32::from_rgb(0x00, 0x78, 0xD4), // Blu VS Code
    ];

    /// Applica il tema VS Code Dark Modern al contesto `egui`.
    pub fn apply(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();

        visuals.panel_fill = Self::SIDEBAR_BG;
        visuals.window_fill = Self::QUICK_PICK_BG;
        visuals.faint_bg_color = Color32::from_rgb(0x22, 0x22, 0x22);
        visuals.extreme_bg_color = Self::INPUT_BG;
        visuals.code_bg_color = Self::EDITOR_BG;

        visuals.window_stroke = Stroke::new(1.0_f32, Self::BORDER);
        visuals.window_corner_radius = CornerRadius::same(4);

        visuals.selection.bg_fill = Self::LIST_ACTIVE_SELECTION_BG;
        visuals.selection.stroke = Stroke::new(1.0_f32, Self::FOCUS_BORDER);

        visuals.widgets.noninteractive.bg_fill = Self::SIDEBAR_BG;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Self::BORDER);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(3);

        visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_PRIMARY);
        visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        visuals.widgets.inactive.corner_radius = CornerRadius::same(3);

        visuals.widgets.hovered.bg_fill = Self::LIST_HOVER_BG;
        visuals.widgets.hovered.weak_bg_fill = Self::LIST_HOVER_BG;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_BRIGHT);
        visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        visuals.widgets.hovered.corner_radius = CornerRadius::same(3);

        visuals.widgets.active.bg_fill = Self::LIST_ACTIVE_SELECTION_BG;
        visuals.widgets.active.weak_bg_fill = Self::LIST_ACTIVE_SELECTION_BG;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Self::TEXT_BRIGHT);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Self::FOCUS_BORDER);
        visuals.widgets.active.corner_radius = CornerRadius::same(3);

        let mut style = Style {
            visuals,
            ..Default::default()
        };

        style.spacing.item_spacing = egui::vec2(6.0, 2.0);
        style.spacing.window_margin = egui::Margin::same(8);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
        style.spacing.indent = 14.0;
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.spacing.scroll.bar_width = 8.0;

        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(12.5, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(11.0, FontFamily::Proportional),
        );

        ctx.set_style(style);
    }
}

/// Icone vettoriali Codicons di VS Code disegnate direttamente con `egui::Painter`
/// (nessuna dipendenza da font Unicode di sistema, zero quadratini "tofu").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codicon {
    Check,
    Refresh,
    MoreHorizontal,
    ListFlat,
    ListTree,
    ChevronDown,
    ChevronRight,
    Plus,
    Minus,
    Discard,
    OpenFile,
    Folder,
    FolderOpened,
    FileCode,
    GitBranch,
    CloudUpload,
    Sync,
    ArrowUp,
    ArrowDown,
    SplitHorizontal,
    DiffInline,
    Tag,
    Archive,
    Trash,
    Close,
    Save,
    Warning,
}

/// Disegna un'icona vettoriale `Codicon` centrata dentro `rect`.
pub fn paint_codicon(painter: &egui::Painter, rect: Rect, icon: Codicon, color: Color32) {
    let c = rect.center();
    let stroke = Stroke::new(1.5_f32, color);
    let bold_stroke = Stroke::new(1.8_f32, color);

    match icon {
        Codicon::Check => {
            // Spunta `✓` stile VS Code Codicon `check`
            let p1 = Pos2::new(c.x - 5.0, c.y + 0.5);
            let p2 = Pos2::new(c.x - 1.5, c.y + 4.0);
            let p3 = Pos2::new(c.x + 5.5, c.y - 4.0);
            painter.line_segment([p1, p2], bold_stroke);
            painter.line_segment([p2, p3], bold_stroke);
        }
        Codicon::Refresh | Codicon::Sync => {
            // Freccia circolare Refresh
            let r = 5.0_f32;
            let n = 14;
            let start_ang = -std::f32::consts::FRAC_PI_4;
            let end_ang = std::f32::consts::PI * 1.35;
            let mut pts = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let a = start_ang + t * (end_ang - start_ang);
                pts.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
            }
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            // Punta della freccia in alto a destra
            let tip = Pos2::new(c.x + r * start_ang.cos(), c.y + r * start_ang.sin());
            painter.line_segment([tip, Pos2::new(tip.x - 3.5, tip.y - 0.5)], stroke);
            painter.line_segment([tip, Pos2::new(tip.x + 0.5, tip.y - 3.5)], stroke);
        }
        Codicon::MoreHorizontal => {
            // Tre puntini `...`
            painter.circle_filled(Pos2::new(c.x - 4.5, c.y), 1.3, color);
            painter.circle_filled(Pos2::new(c.x, c.y), 1.3, color);
            painter.circle_filled(Pos2::new(c.x + 4.5, c.y), 1.3, color);
        }
        Codicon::ListFlat => {
            // Icona List View (3 righe orizzontali con bullet)
            for dy in [-4.0_f32, 0.0, 4.0] {
                painter.circle_filled(Pos2::new(c.x - 5.5, c.y + dy), 1.0, color);
                painter.line_segment(
                    [
                        Pos2::new(c.x - 2.5, c.y + dy),
                        Pos2::new(c.x + 6.0, c.y + dy),
                    ],
                    stroke,
                );
            }
        }
        Codicon::ListTree => {
            // Icona Tree View (struttura gerarchica ad albero)
            painter.line_segment(
                [Pos2::new(c.x - 6.0, c.y - 4.5), Pos2::new(c.x + 2.0, c.y - 4.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x - 4.0, c.y - 4.5), Pos2::new(c.x - 4.0, c.y + 4.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x + 5.5, c.y)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x - 4.0, c.y + 4.5), Pos2::new(c.x + 5.5, c.y + 4.5)],
                stroke,
            );
        }
        Codicon::ChevronDown => {
            // Freccia `▾`
            let p1 = Pos2::new(c.x - 4.0, c.y - 1.8);
            let p2 = Pos2::new(c.x, c.y + 2.2);
            let p3 = Pos2::new(c.x + 4.0, c.y - 1.8);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
        }
        Codicon::ChevronRight => {
            // Freccia `▸`
            let p1 = Pos2::new(c.x - 1.8, c.y - 4.0);
            let p2 = Pos2::new(c.x + 2.2, c.y);
            let p3 = Pos2::new(c.x - 1.8, c.y + 4.0);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
        }
        Codicon::Plus => {
            painter.line_segment(
                [Pos2::new(c.x - 4.5, c.y), Pos2::new(c.x + 4.5, c.y)],
                bold_stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y - 4.5), Pos2::new(c.x, c.y + 4.5)],
                bold_stroke,
            );
        }
        Codicon::Minus => {
            painter.line_segment(
                [Pos2::new(c.x - 4.5, c.y), Pos2::new(c.x + 4.5, c.y)],
                bold_stroke,
            );
        }
        Codicon::Discard => {
            // Freccia curva Undo / Discard Changes di VS Code
            let tip = Pos2::new(c.x - 4.5, c.y - 2.0);
            painter.line_segment([tip, Pos2::new(tip.x + 3.2, tip.y - 2.8)], stroke);
            painter.line_segment([tip, Pos2::new(tip.x + 3.2, tip.y + 2.8)], stroke);
            painter.line_segment([tip, Pos2::new(c.x + 2.0, tip.y)], stroke);
            painter.line_segment(
                [Pos2::new(c.x + 2.0, tip.y), Pos2::new(c.x + 4.5, c.y + 1.0)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 4.5, c.y + 1.0), Pos2::new(c.x + 2.0, c.y + 4.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 2.0, c.y + 4.5), Pos2::new(c.x - 3.0, c.y + 4.5)],
                stroke,
            );
        }
        Codicon::OpenFile | Codicon::FileCode => {
            // Pagina documento con angolo piegato
            let l = c.x - 4.5;
            let r = c.x + 4.5;
            let t = c.y - 5.5;
            let b = c.y + 5.5;
            let fold = 3.0;
            painter.line_segment([Pos2::new(l, t), Pos2::new(r - fold, t)], stroke);
            painter.line_segment([Pos2::new(r - fold, t), Pos2::new(r, t + fold)], stroke);
            painter.line_segment([Pos2::new(r, t + fold), Pos2::new(r, b)], stroke);
            painter.line_segment([Pos2::new(r, b), Pos2::new(l, b)], stroke);
            painter.line_segment([Pos2::new(l, b), Pos2::new(l, t)], stroke);
        }
        Codicon::Folder | Codicon::FolderOpened => {
            let r = Rect::from_center_size(Pos2::new(c.x, c.y + 0.5), Vec2::new(11.0, 8.5));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(r.left(), r.top() - 1.5),
                    Pos2::new(r.left() + 4.5, r.top() - 1.5),
                ],
                stroke,
            );
        }
        Codicon::GitBranch => {
            // Icona Git Branch per Status Bar e Quick Pick
            let top_n = Pos2::new(c.x - 3.5, c.y - 4.5);
            let bot_n = Pos2::new(c.x - 3.5, c.y + 4.5);
            let right_n = Pos2::new(c.x + 3.5, c.y - 1.5);
            painter.circle_stroke(top_n, 1.7, stroke);
            painter.circle_stroke(bot_n, 1.7, stroke);
            painter.circle_stroke(right_n, 1.7, stroke);
            painter.line_segment(
                [Pos2::new(top_n.x, top_n.y + 1.8), Pos2::new(bot_n.x, bot_n.y - 1.8)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(bot_n.x, c.y + 1.5), Pos2::new(right_n.x, c.y + 1.5)],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(right_n.x, c.y + 1.5),
                    Pos2::new(right_n.x, right_n.y + 1.8),
                ],
                stroke,
            );
        }
        Codicon::CloudUpload => {
            // Nuvola / Freccia su
            let base = Rect::from_center_size(Pos2::new(c.x, c.y + 1.0), Vec2::new(11.0, 6.0));
            painter.rect_stroke(base, CornerRadius::same(3), stroke, egui::StrokeKind::Inside);
            painter.circle_stroke(Pos2::new(c.x - 1.0, c.y - 2.0), 3.0, stroke);
        }
        Codicon::ArrowUp => {
            let top = Pos2::new(c.x, c.y - 4.5);
            let bot = Pos2::new(c.x, c.y + 4.5);
            painter.line_segment([top, bot], stroke);
            painter.line_segment([top, Pos2::new(c.x - 3.5, c.y - 1.0)], stroke);
            painter.line_segment([top, Pos2::new(c.x + 3.5, c.y - 1.0)], stroke);
        }
        Codicon::ArrowDown => {
            let top = Pos2::new(c.x, c.y - 4.5);
            let bot = Pos2::new(c.x, c.y + 4.5);
            painter.line_segment([top, bot], stroke);
            painter.line_segment([bot, Pos2::new(c.x - 3.5, c.y + 1.0)], stroke);
            painter.line_segment([bot, Pos2::new(c.x + 3.5, c.y + 1.0)], stroke);
        }
        Codicon::SplitHorizontal => {
            let r = Rect::from_center_size(c, Vec2::new(11.0, 9.0));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom())],
                stroke,
            );
        }
        Codicon::DiffInline => {
            let r = Rect::from_center_size(c, Vec2::new(11.0, 9.0));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [Pos2::new(r.left() + 2.0, c.y - 1.5), Pos2::new(r.right() - 2.0, c.y - 1.5)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(r.left() + 2.0, c.y + 1.5), Pos2::new(r.right() - 2.0, c.y + 1.5)],
                stroke,
            );
        }
        Codicon::Tag => {
            let r = Rect::from_center_size(c, Vec2::new(9.0, 7.0));
            painter.rect_stroke(r, CornerRadius::same(2), stroke, egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(r.left() + 2.5, c.y), 1.0, color);
        }
        Codicon::Archive => {
            let r = Rect::from_center_size(Pos2::new(c.x, c.y + 1.0), Vec2::new(10.0, 7.0));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            let lid = Rect::from_center_size(Pos2::new(c.x, c.y - 3.5), Vec2::new(11.5, 2.5));
            painter.rect_stroke(lid, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
        }
        Codicon::Trash => {
            let r = Rect::from_center_size(Pos2::new(c.x, c.y + 1.0), Vec2::new(8.0, 8.0));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [Pos2::new(c.x - 5.0, c.y - 3.5), Pos2::new(c.x + 5.0, c.y - 3.5)],
                stroke,
            );
        }
        Codicon::Close => {
            painter.line_segment(
                [Pos2::new(c.x - 4.0, c.y - 4.0), Pos2::new(c.x + 4.0, c.y + 4.0)],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 4.0, c.y - 4.0), Pos2::new(c.x - 4.0, c.y + 4.0)],
                stroke,
            );
        }
        Codicon::Save => {
            let r = Rect::from_center_size(c, Vec2::new(10.0, 10.0));
            painter.rect_stroke(r, CornerRadius::same(1), stroke, egui::StrokeKind::Inside);
            let inner = Rect::from_center_size(Pos2::new(c.x, c.y - 2.0), Vec2::new(5.0, 3.5));
            painter.rect_stroke(inner, CornerRadius::ZERO, stroke, egui::StrokeKind::Inside);
        }
        Codicon::Warning => {
            let p1 = Pos2::new(c.x, c.y - 5.0);
            let p2 = Pos2::new(c.x - 5.0, c.y + 4.5);
            let p3 = Pos2::new(c.x + 5.0, c.y + 4.5);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p1], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - 1.5), Pos2::new(c.x, c.y + 1.5)], stroke);
            painter.circle_filled(Pos2::new(c.x, c.y + 3.2), 0.9, color);
        }
    }
}

/// Alloca e disegna un'icona `Codicon` inline (non cliccabile, es. chevron o decorazione).
pub fn draw_inline_codicon(ui: &mut egui::Ui, icon: Codicon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_codicon(ui.painter(), rect, icon, color);
    }
}

/// Bottone icona 20x20 per le toolbar e le righe della sidebar SCM.
pub fn codicon_button(ui: &mut egui::Ui, icon: Codicon, tooltip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::click());
    let hovered = resp.hovered();
    if ui.is_rect_visible(rect) {
        if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(3), VsCodeTheme::LIST_HOVER_BG);
        }
        let fg = if hovered {
            VsCodeTheme::TEXT_BRIGHT
        } else {
            VsCodeTheme::TEXT_PRIMARY
        };
        paint_codicon(ui.painter(), rect, icon, fg);
    }
    resp.on_hover_text(tooltip).clicked()
}

/// Bottone con icona Codicon a sinistra + testo (es. bottone primario Commit / Sync / Publish).
pub fn codicon_text_button(
    ui: &mut egui::Ui,
    icon: Codicon,
    label: &str,
    bg_fill: Color32,
    fg_color: Color32,
    min_size: Vec2,
    corner_radius: CornerRadius,
) -> egui::Response {
    let font_id = FontId::new(12.5, FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font_id, fg_color);
    let content_w = 16.0 + 5.0 + galley.size().x;
    let desired = Vec2::new(min_size.x.max(content_w + 14.0), min_size.y);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    if ui.is_rect_visible(rect) {
        let fill = if resp.hovered() && bg_fill == VsCodeTheme::ACCENT_BLUE {
            VsCodeTheme::ACCENT_BLUE_HOVER
        } else if resp.hovered() && bg_fill == Color32::TRANSPARENT {
            VsCodeTheme::LIST_HOVER_BG
        } else {
            bg_fill
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, corner_radius, fill);
        }

        let start_x = rect.center().x - content_w * 0.5;
        let icon_rect = Rect::from_center_size(
            Pos2::new(start_x + 8.0, rect.center().y),
            Vec2::new(16.0, 16.0),
        );
        paint_codicon(ui.painter(), icon_rect, icon, fg_color);

        let text_pos = Pos2::new(
            start_x + 16.0 + 5.0,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, fg_color);
    }

    let _ = RichText::new(label); // keep import used
    resp
}

/// Disegna un badge circolare/arrotondato numerico stile VS Code (es. contatore modifiche).
pub fn draw_count_badge(ui: &mut egui::Ui, count: usize) {
    let text = count.to_string();
    let font_id = FontId::new(10.5, FontFamily::Proportional);
    let galley = ui
        .painter()
        .layout_no_wrap(text, font_id, VsCodeTheme::TEXT_BRIGHT);
    let width = (galley.size().x + 10.0).max(18.0);
    let height = 16.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), VsCodeTheme::BADGE_BG);
        let text_pos = egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter()
            .galley(text_pos, galley, VsCodeTheme::TEXT_BRIGHT);
    }
}
