use rusqlite::Connection;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

mod extensions;

mod db {
    use rusqlite::{Connection, Result as SqlResult};
    pub fn init(conn: &Connection) -> SqlResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL DEFAULT '#00ff41',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS research_sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                workspace_id TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                decision_note TEXT
            );
            CREATE TABLE IF NOT EXISTS session_sites (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                url TEXT NOT NULL,
                title TEXT NOT NULL,
                visited_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS clips (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                url TEXT NOT NULL,
                page_title TEXT NOT NULL,
                text TEXT NOT NULL,
                clipped_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }
}

mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Workspace {
        pub id: String,
        pub name: String,
        pub color: String,
        pub created_at: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ResearchSession {
        pub id: String,
        pub name: String,
        pub workspace_id: Option<String>,
        pub started_at: String,
        pub ended_at: Option<String>,
        pub decision_note: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct SessionSite {
        pub id: String,
        pub session_id: String,
        pub url: String,
        pub title: String,
        pub visited_at: String,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Clip {
        pub id: String,
        pub session_id: Option<String>,
        pub url: String,
        pub page_title: String,
        pub text: String,
        pub clipped_at: String,
    }
}

mod commands {
    use super::{types::*, DbState};
    use rusqlite::params;
    use tauri::State;

    fn now() -> String {
        chrono::Local::now().to_rfc3339()
    }

    fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    #[tauri::command]
    pub fn create_workspace(
        name: String,
        color: String,
        db: State<DbState>,
    ) -> Result<Workspace, String> {
        let conn = db.0.lock().unwrap();
        let ws = Workspace { id: new_id(), name, color, created_at: now() };
        conn.execute(
            "INSERT INTO workspaces (id, name, color, created_at) VALUES (?1,?2,?3,?4)",
            params![ws.id, ws.name, ws.color, ws.created_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(ws)
    }

    #[tauri::command]
    pub fn list_workspaces(db: State<DbState>) -> Result<Vec<Workspace>, String> {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id,name,color,created_at FROM workspaces ORDER BY created_at ASC")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            out.push(Workspace {
                id: r.get(0).map_err(|e| e.to_string())?,
                name: r.get(1).map_err(|e| e.to_string())?,
                color: r.get(2).map_err(|e| e.to_string())?,
                created_at: r.get(3).map_err(|e| e.to_string())?,
            });
        }
        Ok(out)
    }

    #[tauri::command]
    pub fn create_session(
        name: String,
        workspace_id: Option<String>,
        db: State<DbState>,
    ) -> Result<ResearchSession, String> {
        let conn = db.0.lock().unwrap();
        let s = ResearchSession {
            id: new_id(),
            name,
            workspace_id,
            started_at: now(),
            ended_at: None,
            decision_note: None,
        };
        conn.execute(
            "INSERT INTO research_sessions (id,name,workspace_id,started_at) VALUES (?1,?2,?3,?4)",
            params![s.id, s.name, s.workspace_id, s.started_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(s)
    }

    #[tauri::command]
    pub fn end_session(
        session_id: String,
        decision_note: Option<String>,
        db: State<DbState>,
    ) -> Result<(), String> {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "UPDATE research_sessions SET ended_at=?1, decision_note=?2 WHERE id=?3",
            params![now(), decision_note, session_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[tauri::command]
    pub fn list_sessions(db: State<DbState>) -> Result<Vec<ResearchSession>, String> {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id,name,workspace_id,started_at,ended_at,decision_note FROM research_sessions ORDER BY started_at DESC")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            out.push(ResearchSession {
                id: r.get(0).map_err(|e| e.to_string())?,
                name: r.get(1).map_err(|e| e.to_string())?,
                workspace_id: r.get(2).map_err(|e| e.to_string())?,
                started_at: r.get(3).map_err(|e| e.to_string())?,
                ended_at: r.get(4).map_err(|e| e.to_string())?,
                decision_note: r.get(5).map_err(|e| e.to_string())?,
            });
        }
        Ok(out)
    }

    #[tauri::command]
    pub fn log_site(
        session_id: String,
        url: String,
        title: String,
        db: State<DbState>,
    ) -> Result<(), String> {
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO session_sites (id,session_id,url,title,visited_at) VALUES (?1,?2,?3,?4,?5)",
            params![new_id(), session_id, url, title, now()],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[tauri::command]
    pub fn get_session_sites(
        session_id: String,
        db: State<DbState>,
    ) -> Result<Vec<SessionSite>, String> {
        let conn = db.0.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id,session_id,url,title,visited_at FROM session_sites WHERE session_id=?1 ORDER BY visited_at ASC")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![session_id]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            out.push(SessionSite {
                id: r.get(0).map_err(|e| e.to_string())?,
                session_id: r.get(1).map_err(|e| e.to_string())?,
                url: r.get(2).map_err(|e| e.to_string())?,
                title: r.get(3).map_err(|e| e.to_string())?,
                visited_at: r.get(4).map_err(|e| e.to_string())?,
            });
        }
        Ok(out)
    }

    #[tauri::command]
    pub fn save_clip(
        session_id: Option<String>,
        url: String,
        page_title: String,
        text: String,
        db: State<DbState>,
    ) -> Result<Clip, String> {
        let conn = db.0.lock().unwrap();
        let clip = Clip { id: new_id(), session_id, url, page_title, text, clipped_at: now() };
        conn.execute(
            "INSERT INTO clips (id,session_id,url,page_title,text,clipped_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![clip.id, clip.session_id, clip.url, clip.page_title, clip.text, clip.clipped_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(clip)
    }

    #[tauri::command]
    pub fn get_clips(
        session_id: Option<String>,
        db: State<DbState>,
    ) -> Result<Vec<Clip>, String> {
        let conn = db.0.lock().unwrap();
        let sql = if session_id.is_some() {
            "SELECT id,session_id,url,page_title,text,clipped_at FROM clips WHERE session_id=?1 ORDER BY clipped_at DESC"
        } else {
            "SELECT id,session_id,url,page_title,text,clipped_at FROM clips ORDER BY clipped_at DESC"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let mut rows = if let Some(ref sid) = session_id {
            stmt.query(params![sid]).map_err(|e| e.to_string())?
        } else {
            stmt.query([]).map_err(|e| e.to_string())?
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            out.push(Clip {
                id: r.get(0).map_err(|e| e.to_string())?,
                session_id: r.get(1).map_err(|e| e.to_string())?,
                url: r.get(2).map_err(|e| e.to_string())?,
                page_title: r.get(3).map_err(|e| e.to_string())?,
                text: r.get(4).map_err(|e| e.to_string())?,
                clipped_at: r.get(5).map_err(|e| e.to_string())?,
            });
        }
        Ok(out)
    }

    #[tauri::command]
    pub fn resolve_url(input: String) -> String {
        let trimmed = input.trim();
        // already a full URL — clean it
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return clean_url(trimmed);
        }
        // looks like a domain
        if !trimmed.contains(' ') && trimmed.contains('.') && !trimmed.contains('?') {
            return format!("https://{}", trimmed);
        }
        format!("https://search.brave.com/search?q={}", url_encode(trimmed))
    }

    /// Strip known tracking params from a URL string.
    pub fn clean_url(url: &str) -> String {
        let Ok(mut parsed) = url::Url::parse(url) else {
            return url.to_string();
        };

        const STRIP: &[&str] = &[
            "utm_source","utm_medium","utm_campaign","utm_term","utm_content",
            "utm_id","utm_reader","utm_name","utm_brand","utm_network","utm_device",
            "gclid","gclsrc","gbraid","wbraid","dclid",
            "fbclid","fb_action_ids","fb_source","fb_ref",
            "msclkid","twclid",
            "hsa_acc","hsa_cam","hsa_grp","hsa_ad","hsa_src","hsa_tgt",
            "hsa_kw","hsa_mt","hsa_net","hsa_ver","_hsenc","_hsmi",
            "mc_cid","mc_eid","mkt_tok","icid","ncid","cmpid",
        ];

        let dirty: Vec<String> = parsed
            .query_pairs()
            .filter(|(k, _)| {
                !STRIP.contains(&k.as_ref()) && !k.starts_with("utm_")
            })
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        if dirty.is_empty() {
            parsed.set_query(None);
        } else {
            parsed.set_query(Some(&dirty.join("&")));
        }

        parsed.to_string()
    }

    /// Unwrap common redirect wrappers (google.com/url?q=, t.co, etc.)
    #[tauri::command]
    pub fn unwrap_redirect(url: String) -> String {
        let Ok(parsed) = url::Url::parse(&url) else {
            return url;
        };
        let host = parsed.host_str().unwrap_or("");
        let path = parsed.path();

        // Google redirect
        if host.contains("google.") && (path == "/url" || path == "/search") {
            if let Some(target) = parsed.query_pairs().find(|(k,_)| k == "q" || k == "url").map(|(_,v)| v.to_string()) {
                return clean_url(&target);
            }
        }
        // Facebook
        if host.contains("facebook.com") || host == "l.facebook.com" {
            if let Some(target) = parsed.query_pairs().find(|(k,_)| k == "u").map(|(_,v)| v.to_string()) {
                if let Ok(decoded) = urlencoding::decode(&target) {
                    return clean_url(&decoded);
                }
            }
        }
        // Reddit outbound
        if host == "out.reddit.com" {
            if let Some(target) = parsed.query_pairs().find(|(k,_)| k == "url").map(|(_,v)| v.to_string()) {
                return clean_url(&target);
            }
        }
        // Generic /redirect?url= , /go?url= , /click?url=
        if matches!(path, "/redirect" | "/go" | "/out" | "/click" | "/l" | "/link") {
            if let Some(target) = parsed.query_pairs().find(|(k,_)| k == "url" || k == "to" || k == "href").map(|(_,v)| v.to_string()) {
                if let Ok(decoded) = urlencoding::decode(&target) {
                    return clean_url(&decoded);
                }
            }
        }
        clean_url(&url)
    }

    /// Returns the cookie killer injection script.
    #[tauri::command]
    pub fn get_cookie_killer_script() -> String {
        include_str!("scripts/cookie_killer.js").to_string()
    }

    /// Returns the URL cleaner injection script.
    #[tauri::command]
    pub fn get_url_cleaner_script() -> String {
        include_str!("scripts/url_cleaner.js").to_string()
    }

    /// Returns the forum mode injection script.
    #[tauri::command]
    pub fn get_forum_mode_script() -> String {
        include_str!("scripts/forum_mode.js").to_string()
    }

    /// Returns the notification blocker injection script.
    #[tauri::command]
    pub fn get_notification_blocker_script() -> String {
        include_str!("scripts/notification_blocker.js").to_string()
    }

    fn url_encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u32),
            })
            .collect()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = format!("{}/.rrsearch", home);
    std::fs::create_dir_all(&dir).ok();
    let conn = Connection::open(format!("{}/data.db", dir)).expect("db open failed");
    db::init(&conn).expect("db init failed");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            commands::create_workspace,
            commands::list_workspaces,
            commands::create_session,
            commands::end_session,
            commands::list_sessions,
            commands::log_site,
            commands::get_session_sites,
            commands::save_clip,
            commands::get_clips,
            commands::resolve_url,
            commands::unwrap_redirect,
            commands::get_cookie_killer_script,
            commands::get_url_cleaner_script,
            commands::get_forum_mode_script,
            commands::get_notification_blocker_script,
            extensions::list_extensions,
            extensions::toggle_extension,
            extensions::install_extension,
            extensions::remove_extension,
            extensions::get_extensions_dir,
        ])
        .run(tauri::generate_context!())
        .expect("rrsearch crashed");
}
