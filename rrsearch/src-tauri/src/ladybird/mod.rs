//! Ladybird engine bridge for rrsearch
//!
//! Architecture:
//!
//!   rrsearch UI (Svelte)
//!       ↕  Tauri invoke
//!   Rust LadybirdBridge
//!       ↕  Unix domain socket (LibIPC protocol)
//!   Ladybird WebContent process  ←→  RequestServer
//!                                ←→  ImageDecoder
//!
//! Each browser tab gets its own WebContent process (same model as Ladybird's
//! own Qt/GTK frontends). We communicate using the same .ipc message format
//! that Ladybird's UI layer uses internally.

pub mod ipc;
pub mod process;
pub mod view;

use serde::{Deserialize, Serialize};

/// Whether Ladybird is available on this system (binary compiled + found).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LadybirdStatus {
    pub available: bool,
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub engine: String,
}

#[tauri::command]
pub fn ladybird_status() -> LadybirdStatus {
    let binary = process::find_webcontent_binary();
    LadybirdStatus {
        available: binary.is_some(),
        binary_path: binary.clone(),
        version: binary.as_ref().and_then(|_| process::get_version()),
        engine: "Ladybird/LibWeb (independent)".to_string(),
    }
}

#[tauri::command]
pub fn ladybird_build_instructions() -> String {
    format!(
        r#"# Building Ladybird for rrsearch

## Prerequisites
```
# Ubuntu/Debian
sudo apt install build-essential cmake ninja-build libgl1-mesa-dev \
  qt6-base-dev libqt6svg6-dev qt6-multimedia-dev nasm ccache

# macOS
brew install cmake ninja nasm qt6

# Arch
sudo pacman -S cmake ninja qt6-base qt6-svg qt6-multimedia nasm
```

## Build WebContent (the only process rrsearch needs)
```bash
cd rrsearch/ladybird
cmake --preset default
ninja -C Build/ladybird WebContent RequestServer ImageDecoder
```

## Install to rrsearch
```bash
# Binaries end up at:
# Build/ladybird/bin/WebContent
# Build/ladybird/bin/RequestServer
# Build/ladybird/bin/ImageDecoder

# rrsearch looks for them at:
# ~/.rrsearch/engine/WebContent
# ~/.rrsearch/engine/RequestServer
# ~/.rrsearch/engine/ImageDecoder

mkdir -p ~/.rrsearch/engine
cp Build/ladybird/bin/{{WebContent,RequestServer,ImageDecoder}} ~/.rrsearch/engine/
```

## Verify
The rrsearch extensions panel will show "Ladybird engine: ready" once binaries
are in place. Restart rrsearch to activate.

Engine location: {}
"#,
        process::engine_dir()
    )
}
