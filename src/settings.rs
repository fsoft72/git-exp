use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScmFileViewMode {
    #[default]
    List,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DiffViewMode {
    #[default]
    SideBySide,
    Inline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    pub recent_repos: Vec<String>,
    pub file_view_mode: ScmFileViewMode,
    pub diff_view_mode: DiffViewMode,
    pub show_graph_section: bool,
    pub show_output_panel: bool,
    pub commit_signoff: bool,
    /// Intervallo di "Refresh git status" automatico in secondi (0 = disabilitato).
    pub refresh_interval_secs: u64,
    /// Se `true`, mostra lo spinner e il messaggio "Refreshing Git status..." nella Status Bar;
    /// se `false` (silent), esegue il refresh in background senza mostrare il messaggio nella Status Bar.
    pub show_refresh_status_message: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            recent_repos: Vec::new(),
            file_view_mode: ScmFileViewMode::List,
            diff_view_mode: DiffViewMode::SideBySide,
            show_graph_section: true,
            show_output_panel: false,
            commit_signoff: false,
            refresh_interval_secs: 5,
            show_refresh_status_message: true,
        }
    }
}

impl UserSettings {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("vscode-git-egui").join("settings.json"))
    }

    pub fn load() -> Self {
        if let Some(p) = Self::config_path() {
            if let Ok(content) = fs::read_to_string(p) {
                if let Ok(parsed) = serde_json::from_str::<Self>(&content) {
                    return parsed;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::config_path() {
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(p, json);
            }
        }
    }

    pub fn push_recent_repo(&mut self, path: &str) {
        self.recent_repos.retain(|r| r != path);
        self.recent_repos.insert(0, path.to_string());
        if self.recent_repos.len() > 12 {
            self.recent_repos.truncate(12);
        }
        self.save();
    }
}
