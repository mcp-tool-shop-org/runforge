use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Where preferences are written. A packaged run passes its LocalState directory.
/// An unpackaged run passes the directory that contains the executable.
pub fn choose_prefs_dir(packaged_local_state: Option<&Path>, exe_dir: &Path) -> PathBuf {
    packaged_local_state.unwrap_or(exe_dir).to_path_buf()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            _ => None,
        }
    }
}

/// Last folder and theme. Unknown keys in the file are kept.
#[derive(Clone, Debug)]
pub struct Prefs {
    pub last_folder: Option<PathBuf>,
    pub theme: Theme,
    raw: Map<String, Value>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            last_folder: None,
            theme: Theme::Dark,
            raw: Map::new(),
        }
    }
}

impl Prefs {
    pub fn set_folder(&mut self, folder: &Path) {
        self.last_folder = Some(folder.to_path_buf());
        self.raw.insert(
            "last_folder".to_string(),
            Value::String(folder.to_string_lossy().into_owned()),
        );
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.raw.insert(
            "theme".to_string(),
            Value::String(theme.as_str().to_string()),
        );
    }
}

pub const PREFS_FILE: &str = "preferences.json";

pub fn read_prefs(directory: &Path) -> Prefs {
    let Ok(bytes) = fs::read(directory.join(PREFS_FILE)) else {
        return Prefs::default();
    };
    let Ok(Value::Object(raw)) = serde_json::from_slice(&bytes) else {
        return Prefs::default();
    };
    let last_folder = raw
        .get("last_folder")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(PathBuf::from);
    let theme = raw
        .get("theme")
        .and_then(Value::as_str)
        .and_then(Theme::parse)
        .unwrap_or(Theme::Dark);
    Prefs {
        last_folder,
        theme,
        raw,
    }
}

pub fn write_prefs(directory: &Path, prefs: &Prefs) -> Result<(), std::io::Error> {
    fs::create_dir_all(directory)?;
    let mut bytes = serde_json::to_vec_pretty(&prefs.raw).expect("preferences should serialize");
    bytes.push(b'\n');
    fs::write(directory.join(PREFS_FILE), bytes)
}
