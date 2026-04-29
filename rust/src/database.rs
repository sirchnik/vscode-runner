use log::{error, info};
use rusqlite::Connection;
use serde_json::Value;
use std::path::Path;

pub fn get_recent_workspace_paths(db_path: &str) -> Vec<String> {
    if !Path::new(db_path).exists() {
        info!("VSCode database file does not exist at {db_path}");
        return vec![];
    }

    let conn = match Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(e) => {
            error!("Unable to open VSCode database file at {db_path}: {e}");
            return vec![];
        }
    };

    info!("Opened VSCode database file at {db_path}");

    let json_string: String = match conn.query_row(
        "SELECT value FROM ItemTable WHERE key = 'history.recentlyOpenedPathsList'",
        [],
        |row| row.get(0),
    ) {
        Ok(v) => v,
        Err(e) => {
            info!("No recent workspaces found in VSCode database: {e}");
            return vec![];
        }
    };

    parse_recent_paths(&json_string)
}

fn parse_recent_paths(json_str: &str) -> Vec<String> {
    let data: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse recent paths JSON: {e}");
            return vec![];
        }
    };

    let entries = match data.get("entries").and_then(|e| e.as_array()) {
        Some(arr) => arr,
        None => return vec![],
    };

    entries
        .iter()
        .filter_map(|entry| {
            entry
                .get("folderUri")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned())
        })
        .collect()
}
