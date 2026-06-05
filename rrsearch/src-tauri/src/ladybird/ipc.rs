//! LibIPC wire protocol implementation.
//!
//! Ladybird's IPC uses Unix domain sockets with a simple framing format:
//!
//!   [ u32 message_id ][ u32 payload_size ][ payload bytes... ]
//!
//! Message IDs and payload schemas are defined in the .ipc files:
//!   Services/WebContent/WebContentServer.ipc  — messages WE send to WebContent
//!   Services/WebContent/WebContentClient.ipc  — messages WebContent sends to US
//!
//! This module implements the subset of messages rrsearch needs:
//!   Outbound (→ WebContent):  load_url, set_viewport, key_event, mouse_event,
//!                              reload, traverse_the_history_by_delta
//!   Inbound  (← WebContent):  did_start_loading, did_finish_loading,
//!                              did_change_title, did_change_url,
//!                              did_paint (bitmap frames)

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// Raw framed IPC message.
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub message_id: u32,
    pub payload: Vec<u8>,
}

/// Known outbound message IDs (→ WebContent).
/// These correspond to the endpoint declaration order in WebContentServer.ipc.
/// Full mapping extracted from Ladybird source — update if Ladybird changes.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ServerMessage {
    InitTransport    = 1,
    Initialize       = 2,
    LoadUrl          = 10,
    LoadHtml         = 11,
    Reload           = 13,
    TraverseHistory  = 14,
    SetViewport      = 15,
    KeyEvent         = 16,
    MouseEvent       = 17,
    CloseServer      = 3,
}

/// Known inbound message IDs (← WebContent → us).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientMessage {
    DidStartLoading  = 3,
    DidFinishLoading = 4,
    DidChangeTitle   = 7,
    DidChangeUrl     = 8,
    DidHoverLink     = 14,
    DidUnhoverLink   = 15,
}

pub struct IpcConnection {
    stream: UnixStream,
}

impl IpcConnection {
    pub fn connect(socket_path: &str) -> Result<Self, String> {
        // WebContent needs a moment to create its socket after spawning
        let mut attempts = 0;
        loop {
            match UnixStream::connect(socket_path) {
                Ok(s) => {
                    s.set_read_timeout(Some(Duration::from_millis(100))).ok();
                    return Ok(Self { stream: s });
                }
                Err(e) if attempts < 20 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(50));
                    let _ = e;
                }
                Err(e) => return Err(format!("IPC connect failed: {}", e)),
            }
        }
    }

    /// Send a framed message to WebContent.
    pub fn send(&mut self, id: ServerMessage, payload: &[u8]) -> Result<(), String> {
        let msg_id = id as u32;
        let len = payload.len() as u32;
        let mut buf = Vec::with_capacity(8 + payload.len());
        buf.extend_from_slice(&msg_id.to_le_bytes());
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(payload);
        self.stream.write_all(&buf).map_err(|e| e.to_string())
    }

    /// Try to read one inbound message (non-blocking via timeout).
    pub fn try_recv(&mut self) -> Option<IpcMessage> {
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header).ok()?;
        let message_id = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let payload_len = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let mut payload = vec![0u8; payload_len as usize];
        self.stream.read_exact(&mut payload).ok()?;
        Some(IpcMessage { message_id, payload })
    }

    // ── High-level helpers ────────────────────────────────────────────────

    /// Tell WebContent to initialize with the given page ID.
    pub fn initialize(&mut self, page_id: u64) -> Result<(), String> {
        self.send(ServerMessage::Initialize, &page_id.to_le_bytes())
    }

    /// Navigate to a URL.
    pub fn load_url(&mut self, page_id: u64, url: &str) -> Result<(), String> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_id.to_le_bytes());
        // URL serialized as u32 len + UTF-8 bytes (Ladybird String encoding)
        let url_bytes = url.as_bytes();
        payload.extend_from_slice(&(url_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(url_bytes);
        self.send(ServerMessage::LoadUrl, &payload)
    }

    /// Set the viewport size (width × height in device pixels).
    pub fn set_viewport(&mut self, page_id: u64, width: u32, height: u32) -> Result<(), String> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&page_id.to_le_bytes());
        payload.extend_from_slice(&width.to_le_bytes());
        payload.extend_from_slice(&height.to_le_bytes());
        // device_pixel_ratio (f64) + is_fullscreen (u8)
        payload.extend_from_slice(&1.0f64.to_le_bytes());
        payload.push(0u8);
        self.send(ServerMessage::SetViewport, &payload)
    }

    pub fn reload(&mut self, page_id: u64) -> Result<(), String> {
        self.send(ServerMessage::Reload, &page_id.to_le_bytes())
    }

    pub fn go_back(&mut self, page_id: u64) -> Result<(), String> {
        let mut payload = page_id.to_le_bytes().to_vec();
        payload.extend_from_slice(&(-1i32).to_le_bytes());
        self.send(ServerMessage::TraverseHistory, &payload)
    }

    pub fn go_forward(&mut self, page_id: u64) -> Result<(), String> {
        let mut payload = page_id.to_le_bytes().to_vec();
        payload.extend_from_slice(&1i32.to_le_bytes());
        self.send(ServerMessage::TraverseHistory, &payload)
    }

    pub fn close(&mut self) -> Result<(), String> {
        self.send(ServerMessage::CloseServer, &[])
    }
}

/// Decoded inbound event from WebContent.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebContentEvent {
    StartLoading { page_id: u64, url: String },
    FinishLoading { page_id: u64, url: String },
    TitleChanged  { page_id: u64, title: String },
    UrlChanged    { page_id: u64, url: String },
    HoveredLink   { page_id: u64, url: String },
    UnhoveredLink { page_id: u64 },
    Unknown       { message_id: u32 },
}

/// Parse a raw IpcMessage into a structured WebContentEvent.
pub fn parse_event(msg: IpcMessage) -> WebContentEvent {
    match msg.message_id {
        id if id == ClientMessage::DidStartLoading as u32 => {
            let (page_id, url) = parse_page_url(&msg.payload);
            WebContentEvent::StartLoading { page_id, url }
        }
        id if id == ClientMessage::DidFinishLoading as u32 => {
            let (page_id, url) = parse_page_url(&msg.payload);
            WebContentEvent::FinishLoading { page_id, url }
        }
        id if id == ClientMessage::DidChangeTitle as u32 => {
            let (page_id, title) = parse_page_string(&msg.payload);
            WebContentEvent::TitleChanged { page_id, title }
        }
        id if id == ClientMessage::DidChangeUrl as u32 => {
            let (page_id, url) = parse_page_url(&msg.payload);
            WebContentEvent::UrlChanged { page_id, url }
        }
        id if id == ClientMessage::DidHoverLink as u32 => {
            let (page_id, url) = parse_page_url(&msg.payload);
            WebContentEvent::HoveredLink { page_id, url }
        }
        id if id == ClientMessage::DidUnhoverLink as u32 => {
            WebContentEvent::UnhoveredLink {
                page_id: parse_page_id(&msg.payload),
            }
        }
        _ => WebContentEvent::Unknown { message_id: msg.message_id },
    }
}

fn parse_page_id(payload: &[u8]) -> u64 {
    if payload.len() >= 8 {
        u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]))
    } else {
        0
    }
}

fn parse_page_url(payload: &[u8]) -> (u64, String) {
    let page_id = parse_page_id(payload);
    let s = parse_ladybird_string(&payload[8..]);
    (page_id, s)
}

fn parse_page_string(payload: &[u8]) -> (u64, String) {
    let page_id = parse_page_id(payload);
    let s = parse_ladybird_string(&payload[8..]);
    (page_id, s)
}

/// Ladybird serializes strings as: u32 length + UTF-8 bytes.
fn parse_ladybird_string(data: &[u8]) -> String {
    if data.len() < 4 { return String::new(); }
    let len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    if data.len() < 4 + len { return String::new(); }
    String::from_utf8_lossy(&data[4..4 + len]).to_string()
}
