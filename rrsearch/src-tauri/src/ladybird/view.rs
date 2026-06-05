//! LadybirdView — one WebContent process per tab.
//!
//! This is rrsearch's equivalent of Ladybird's `OutOfProcessWebView` widget.
//! It owns the WebContent child process and the IPC connection to it,
//! and exposes a clean Tauri command interface for the Svelte frontend.

use super::{
    ipc::{IpcConnection, WebContentEvent, parse_event},
    process::LadybirdSession,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    process::Child,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TabState {
    pub page_id: u64,
    pub url: String,
    pub title: String,
    pub loading: bool,
}

/// Global registry of active Ladybird views, keyed by page_id.
pub struct ViewRegistry {
    pub session: Option<LadybirdSession>,
    pub views: HashMap<u64, ActiveView>,
    pub next_page_id: u64,
}

pub struct ActiveView {
    pub process: Child,
    pub conn: IpcConnection,
    pub state: TabState,
}

impl ViewRegistry {
    pub fn new() -> Self {
        Self { session: None, views: HashMap::new(), next_page_id: 1 }
    }

    /// Start the shared session (RequestServer + ImageDecoder).
    pub fn start_session(&mut self) -> Result<(), String> {
        if self.session.is_some() { return Ok(()); }
        let sid = uuid::Uuid::new_v4().to_string();
        self.session = Some(LadybirdSession::start(&sid)?);
        Ok(())
    }

    /// Open a new tab — spawns a WebContent process and connects IPC.
    pub fn open_tab(&mut self) -> Result<u64, String> {
        let session = self.session.as_ref().ok_or("session not started")?;
        let page_id = self.next_page_id;
        self.next_page_id += 1;

        let (process, socket_path) = session.spawn_webcontent(page_id)?;

        let mut conn = IpcConnection::connect(&socket_path)?;
        conn.initialize(page_id)?;

        self.views.insert(page_id, ActiveView {
            process,
            conn,
            state: TabState {
                page_id,
                url: String::new(),
                title: "new tab".to_string(),
                loading: false,
            },
        });

        Ok(page_id)
    }

    pub fn close_tab(&mut self, page_id: u64) {
        if let Some(mut view) = self.views.remove(&page_id) {
            view.conn.close().ok();
            view.process.kill().ok();
        }
    }

    pub fn navigate(&mut self, page_id: u64, url: &str) -> Result<(), String> {
        let view = self.views.get_mut(&page_id).ok_or("tab not found")?;
        view.conn.load_url(page_id, url)?;
        view.state.url = url.to_string();
        view.state.loading = true;
        Ok(())
    }

    pub fn reload(&mut self, page_id: u64) -> Result<(), String> {
        let view = self.views.get_mut(&page_id).ok_or("tab not found")?;
        view.conn.reload(page_id)
    }

    pub fn go_back(&mut self, page_id: u64) -> Result<(), String> {
        let view = self.views.get_mut(&page_id).ok_or("tab not found")?;
        view.conn.go_back(page_id)
    }

    pub fn go_forward(&mut self, page_id: u64) -> Result<(), String> {
        let view = self.views.get_mut(&page_id).ok_or("tab not found")?;
        view.conn.go_forward(page_id)
    }

    pub fn resize(&mut self, page_id: u64, width: u32, height: u32) -> Result<(), String> {
        let view = self.views.get_mut(&page_id).ok_or("tab not found")?;
        view.conn.set_viewport(page_id, width, height)
    }

    /// Poll all active WebContent processes for inbound events.
    /// Call this on a background thread; emits Tauri events to the frontend.
    pub fn poll_events(&mut self, app: &AppHandle) {
        for (page_id, view) in self.views.iter_mut() {
            while let Some(msg) = view.conn.try_recv() {
                let event = parse_event(msg);

                // update local state
                match &event {
                    WebContentEvent::FinishLoading { url, .. } => {
                        view.state.url = url.clone();
                        view.state.loading = false;
                    }
                    WebContentEvent::StartLoading { .. } => {
                        view.state.loading = true;
                    }
                    WebContentEvent::TitleChanged { title, .. } => {
                        view.state.title = title.clone();
                    }
                    WebContentEvent::UrlChanged { url, .. } => {
                        view.state.url = url.clone();
                    }
                    _ => {}
                }

                // forward to Svelte frontend
                app.emit("ladybird-event", &event).ok();
            }
        }
    }

    pub fn tab_states(&self) -> Vec<TabState> {
        self.views.values().map(|v| v.state.clone()).collect()
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

pub type ViewState = Arc<Mutex<ViewRegistry>>;

#[tauri::command]
pub fn lb_start(state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().start_session()
}

#[tauri::command]
pub fn lb_open_tab(state: tauri::State<ViewState>) -> Result<u64, String> {
    state.lock().unwrap().open_tab()
}

#[tauri::command]
pub fn lb_close_tab(page_id: u64, state: tauri::State<ViewState>) {
    state.lock().unwrap().close_tab(page_id);
}

#[tauri::command]
pub fn lb_navigate(page_id: u64, url: String, state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().navigate(page_id, &url)
}

#[tauri::command]
pub fn lb_reload(page_id: u64, state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().reload(page_id)
}

#[tauri::command]
pub fn lb_back(page_id: u64, state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().go_back(page_id)
}

#[tauri::command]
pub fn lb_forward(page_id: u64, state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().go_forward(page_id)
}

#[tauri::command]
pub fn lb_resize(page_id: u64, width: u32, height: u32, state: tauri::State<ViewState>) -> Result<(), String> {
    state.lock().unwrap().resize(page_id, width, height)
}

#[tauri::command]
pub fn lb_tab_states(state: tauri::State<ViewState>) -> Vec<TabState> {
    state.lock().unwrap().tab_states()
}
