use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::token_store::{self, StoredToken};

fn cursor_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| {
            h.join("Library/Application Support/Cursor/User/globalStorage/state.vscdb")
        })
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|h| h.join(".config/Cursor/User/globalStorage/state.vscdb"))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|d| d.join("Cursor/User/globalStorage/state.vscdb"))
    }
}

pub struct CursorCredentials {
    pub access_token: String,
    pub machine_id: String,
}

pub fn read_cursor_credentials() -> Result<CursorCredentials> {
    let db_path = cursor_db_path()
        .context("Could not determine Cursor data directory for this platform")?;

    if !db_path.exists() {
        anyhow::bail!(
            "Cursor database not found at {}. Is Cursor IDE installed and logged in?",
            db_path.display()
        );
    }

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open Cursor database at {}", db_path.display()))?;

    let access_token: String = conn
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            ["cursorAuth/accessToken"],
            |row| row.get(0),
        )
        .context("Could not read access token from Cursor database. Make sure you are logged in to Cursor IDE.")?;

    let machine_id: String = conn
        .query_row(
            "SELECT value FROM ItemTable WHERE key = ?1",
            ["storage.serviceMachineId"],
            |row| row.get(0),
        )
        .context("Could not read machine ID from Cursor database")?;

    if access_token.is_empty() {
        anyhow::bail!("Cursor access token is empty. Please log in to Cursor IDE first.");
    }

    Ok(CursorCredentials {
        access_token,
        machine_id,
    })
}

pub async fn login() -> Result<StoredToken> {
    let creds = read_cursor_credentials()?;
    let token = StoredToken {
        access_token: format!("{}:::{}", creds.access_token, creds.machine_id),
        refresh_token: None,
        expires_at: None,
        provider: "cursor".to_string(),
    };
    token_store::store_token("cursor", &token)?;
    Ok(token)
}

pub async fn login_interactive(msg_tx: mpsc::UnboundedSender<String>) -> Result<StoredToken> {
    let _ = msg_tx.send("Reading Cursor IDE credentials...".to_string());

    match read_cursor_credentials() {
        Ok(creds) => {
            let _ = msg_tx.send("Found Cursor token and machine ID.".to_string());
            let token = StoredToken {
                access_token: format!("{}:::{}", creds.access_token, creds.machine_id),
                refresh_token: None,
                expires_at: None,
                provider: "cursor".to_string(),
            };
            token_store::store_token("cursor", &token)?;
            let _ = msg_tx.send("Cursor login successful.".to_string());
            Ok(token)
        }
        Err(e) => {
            let _ = msg_tx.send(format!("Auto-import failed: {e}"));
            let _ = msg_tx.send("Paste your Cursor access token manually via /connect -> Cursor -> API key.".to_string());
            Err(e)
        }
    }
}

pub fn parse_cursor_token(combined: &str) -> (String, String) {
    if let Some((token, machine)) = combined.split_once(":::") {
        (token.to_string(), machine.to_string())
    } else {
        (combined.to_string(), String::new())
    }
}
