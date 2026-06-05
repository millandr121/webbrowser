use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub kind: ExtensionKind,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionKind {
    Builtin,  // shipped with rrsearch (ublock rules, fastforward)
    Loaded,   // user-loaded from disk
}

fn extensions_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let p = PathBuf::from(format!("{}/.rrsearch/extensions", home));
    fs::create_dir_all(&p).ok();
    p
}

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}/.rrsearch/extensions.json", home))
}

fn load_state() -> Vec<Extension> {
    let path = state_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(exts) = serde_json::from_str(&data) {
            return exts;
        }
    }
    // default built-ins
    vec![
        Extension {
            id: "rrsearch-ublock".to_string(),
            name: "uBlock Origin".to_string(),
            version: "1.0.0".to_string(),
            description: "Wide-spectrum content blocker. Blocks ads, trackers, malware domains.".to_string(),
            enabled: true,
            kind: ExtensionKind::Builtin,
        },
        Extension {
            id: "rrsearch-fastforward".to_string(),
            name: "FastForward".to_string(),
            version: "1.0.0".to_string(),
            description: "Bypasses link shorteners and skip-timers automatically.".to_string(),
            enabled: true,
            kind: ExtensionKind::Builtin,
        },
        Extension {
            id: "rrsearch-bitwarden".to_string(),
            name: "Bitwarden".to_string(),
            version: "1.0.0".to_string(),
            description: "Open source password manager. Load your Bitwarden extension to enable.".to_string(),
            enabled: false,
            kind: ExtensionKind::Builtin,
        },
    ]
}

fn save_state(exts: &[Extension]) {
    if let Ok(data) = serde_json::to_string_pretty(exts) {
        fs::write(state_path(), data).ok();
    }
}

#[tauri::command]
pub fn list_extensions() -> Vec<Extension> {
    load_state()
}

#[tauri::command]
pub fn toggle_extension(id: String) -> Result<Vec<Extension>, String> {
    let mut exts = load_state();
    if let Some(ext) = exts.iter_mut().find(|e| e.id == id) {
        ext.enabled = !ext.enabled;
    } else {
        return Err(format!("extension not found: {}", id));
    }
    save_state(&exts);
    Ok(exts)
}

#[tauri::command]
pub fn install_extension(name: String, description: String) -> Result<Vec<Extension>, String> {
    let mut exts = load_state();
    let id = name.to_lowercase().replace(' ', "-");
    if exts.iter().any(|e| e.id == id) {
        return Err("extension already installed".to_string());
    }
    exts.push(Extension {
        id,
        name,
        version: "1.0.0".to_string(),
        description,
        enabled: true,
        kind: ExtensionKind::Loaded,
    });
    save_state(&exts);
    Ok(exts)
}

#[tauri::command]
pub fn remove_extension(id: String) -> Result<Vec<Extension>, String> {
    let mut exts = load_state();
    let before = exts.len();
    exts.retain(|e| e.id != id || matches!(e.kind, ExtensionKind::Builtin));
    if exts.len() == before {
        return Err("cannot remove built-in or not found".to_string());
    }
    save_state(&exts);
    Ok(exts)
}

/// Returns the extensions directory path for the user to drop .xpi/.crx files
#[tauri::command]
pub fn get_extensions_dir() -> String {
    extensions_dir().to_string_lossy().to_string()
}
