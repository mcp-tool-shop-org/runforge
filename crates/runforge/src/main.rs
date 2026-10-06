// The Store package is a window. A console subsystem opens a second window.
#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;

use anyhow::Context;
use runforge_core::{VERSION, choose_prefs_dir};

mod app;
mod instrument;
mod launch;
mod sidecar;
mod store;

fn main() -> eframe::Result {
    let prefs_dir = prefs_dir();
    let _ = std::fs::create_dir_all(&prefs_dir);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1480.0, 960.0])
            .with_title(format!("RunForge {VERSION}")),
        persistence_path: Some(prefs_dir.clone()),
        ..Default::default()
    };
    eframe::run_native(
        "RunForge",
        options,
        Box::new(move |_creation| Ok(Box::new(app::RunForgeApp::open(prefs_dir)))),
    )
}

fn prefs_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .context("the executable directory")
        .unwrap_or_else(|_| std::env::temp_dir());
    choose_prefs_dir(store::packaged_local_state().as_deref(), &exe_dir)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_prefs_dir_is_absolute() {
        let dir = super::prefs_dir();
        assert!(dir.is_absolute());
    }
}
