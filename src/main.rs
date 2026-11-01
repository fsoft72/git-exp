mod git;
mod settings;
mod state;
mod theme;
mod ui;
mod watcher;

use crate::state::AppState;
use crate::theme::VsCodeTheme;
use std::path::PathBuf;
use std::sync::Arc;

fn load_app_icon() -> Option<egui::IconData> {
    let svg_bytes = include_bytes!("../assets/images/logo.svg");
    let color_image = egui_extras::image::load_svg_bytes_with_size(
        svg_bytes,
        Some(egui::load::SizeHint::Size(256, 256)),
    )
    .ok()?;
    let width = color_image.size[0] as u32;
    let height = color_image.size[1] as u32;
    let rgba = color_image
        .pixels
        .into_iter()
        .flat_map(|c| c.to_array())
        .collect();
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

struct VsCodeGitApp {
    state: AppState,
    theme_applied: bool,
}

impl VsCodeGitApp {
    fn new(cc: &eframe::CreationContext<'_>, initial_repo: Option<PathBuf>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            state: AppState::new(initial_repo),
            theme_applied: false,
        }
    }
}

impl eframe::App for VsCodeGitApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            VsCodeTheme::apply(ctx);
            self.theme_applied = true;
        }

        // Poll background worker & filesystem watcher
        self.state.poll_background(ctx);

        // Ordine di rendering dei pannelli in egui:
        // 1. Status Bar in fondo
        ui::status_bar::render_status_bar(ctx, &mut self.state);
        // 2. Activity Bar a sinistra
        ui::activity_bar::render_activity_bar(ctx, &mut self.state);
        // 3. Sidebar SCM a sinistra
        ui::scm_sidebar::render_sidebar(ctx, &mut self.state);
        // 4. Git Output Drawer in basso (sopra la status bar)
        ui::output_panel::render_output_panel(ctx, &mut self.state);
        // 5. Area Centrale Diff Viewer / Editor / Commit Inspector
        ui::diff_viewer::render_center_panel(ctx, &mut self.state);
        // 6. Modale Quick Pick / Command Palette sopra tutto
        ui::quick_pick::render_quick_pick_modal(ctx, &mut self.state);
    }
}

fn main() -> eframe::Result<()> {
    let initial_repo = std::env::args().nth(1).map(PathBuf::from);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Git-Exp")
        .with_inner_size([1360.0, 860.0])
        .with_min_inner_size([840.0, 520.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "vscode-git-egui",
        native_options,
        Box::new(move |cc| Ok(Box::new(VsCodeGitApp::new(cc, initial_repo)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_app_icon() {
        let icon = load_app_icon().expect("App icon should load and rasterize from logo.svg");
        assert_eq!(icon.width, 256);
        assert_eq!(icon.height, 256);
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        assert!(!icon.is_empty());
    }
}

