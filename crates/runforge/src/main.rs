use std::path::PathBuf;

use anyhow::Context;
use runforge_core::{VERSION, choose_prefs_dir};

mod app;
mod store;

fn main() -> eframe::Result {
    let prefs_dir = prefs_dir();
    let _ = std::fs::create_dir_all(&prefs_dir);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 780.0])
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
