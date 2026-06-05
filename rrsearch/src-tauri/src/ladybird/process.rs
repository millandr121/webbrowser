//! Spawn and manage Ladybird child processes.
//!
//! rrsearch needs three Ladybird processes per session:
//!   WebContent    — renders pages, runs JS (one per tab)
//!   RequestServer — handles all network I/O
//!   ImageDecoder  — decodes images out-of-process (sandboxed)

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub fn engine_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.rrsearch/engine", home)
}

/// Find the WebContent binary. Checks engine_dir first, then PATH.
pub fn find_webcontent_binary() -> Option<String> {
    // 1. rrsearch engine dir (user-installed build)
    let local = PathBuf::from(engine_dir()).join("WebContent");
    if local.exists() {
        return Some(local.to_string_lossy().to_string());
    }
    // 2. system PATH (distro packages in future)
    which("WebContent")
}

pub fn find_request_server_binary() -> Option<String> {
    let local = PathBuf::from(engine_dir()).join("RequestServer");
    if local.exists() {
        return Some(local.to_string_lossy().to_string());
    }
    which("RequestServer")
}

pub fn find_image_decoder_binary() -> Option<String> {
    let local = PathBuf::from(engine_dir()).join("ImageDecoder");
    if local.exists() {
        return Some(local.to_string_lossy().to_string());
    }
    which("ImageDecoder")
}

fn which(name: &str) -> Option<String> {
    std::env::var("PATH").ok()?.split(':').find_map(|dir| {
        let p = PathBuf::from(dir).join(name);
        if p.exists() { Some(p.to_string_lossy().to_string()) } else { None }
    })
}

pub fn get_version() -> Option<String> {
    // Ladybird doesn't expose --version yet; return the git rev of our submodule
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()?
                .join("ladybird"),
        )
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// A running set of Ladybird helper processes for one browsing session.
pub struct LadybirdSession {
    pub request_server: Arc<Mutex<Option<Child>>>,
    pub image_decoder:  Arc<Mutex<Option<Child>>>,
    pub session_id:     String,
    pub socket_base:    String,
}

impl LadybirdSession {
    /// Spawn RequestServer and ImageDecoder. WebContent processes are spawned
    /// per-tab via `spawn_webcontent()`.
    pub fn start(session_id: &str) -> Result<Self, String> {
        let socket_base = format!("/tmp/rrsearch-{}", session_id);
        std::fs::create_dir_all(&socket_base).map_err(|e| e.to_string())?;

        let rs_bin = find_request_server_binary()
            .ok_or("RequestServer binary not found — run the Ladybird build first")?;
        let id_bin = find_image_decoder_binary()
            .ok_or("ImageDecoder binary not found — run the Ladybird build first")?;

        let rs = Command::new(&rs_bin)
            .env("LADYBIRD_SESSION_SOCKET_PATH", &socket_base)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn RequestServer: {}", e))?;

        let id = Command::new(&id_bin)
            .env("LADYBIRD_SESSION_SOCKET_PATH", &socket_base)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn ImageDecoder: {}", e))?;

        Ok(Self {
            request_server: Arc::new(Mutex::new(Some(rs))),
            image_decoder:  Arc::new(Mutex::new(Some(id))),
            session_id: session_id.to_string(),
            socket_base,
        })
    }

    /// Spawn a WebContent process for a new tab. Returns the socket path
    /// that the tab's IPC connection should connect to.
    pub fn spawn_webcontent(&self, page_id: u64) -> Result<(Child, String), String> {
        let wc_bin = find_webcontent_binary()
            .ok_or("WebContent binary not found — run the Ladybird build first")?;

        let socket_path = format!("{}/webcontent-{}", self.socket_base, page_id);

        let child = Command::new(&wc_bin)
            .args(["--socket-path", &socket_path])
            .env("LADYBIRD_SESSION_SOCKET_PATH", &self.socket_base)
            .stdout(Stdio::null())
            .stderr(Stdio::piped()) // capture errors for debugging
            .spawn()
            .map_err(|e| format!("failed to spawn WebContent: {}", e))?;

        Ok((child, socket_path))
    }
}

impl Drop for LadybirdSession {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.request_server.lock() {
            if let Some(mut child) = guard.take() {
                child.kill().ok();
            }
        }
        if let Ok(mut guard) = self.image_decoder.lock() {
            if let Some(mut child) = guard.take() {
                child.kill().ok();
            }
        }
        std::fs::remove_dir_all(&self.socket_base).ok();
    }
}
