use log::{error, info};
use rusqlite::Connection;
use serde_json::Value;
use std::path::Path;

pub fn get_recent_workspace_paths(db_path: &str) -> Vec<String> {
    if !Path::new(db_path).exists() {
        info!("VSCode database file does not exist at {db_path}");
        return vec![];
    }

    let conn =
        match Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
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
            if let Some(uri) = entry.get("folderUri").and_then(|v| v.as_str()) {
                Some(uri.to_owned())
            } else {
                entry
                    .get("workspace")
                    .and_then(|w| w.get("configPath"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_owned())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recent_paths_with_entries() {
        let json = r#"{
            "entries": [
                {"folderUri": "file:///home/user/project1"},
                {"folderUri": "file:///home/user/project2"}
            ]
        }"#;
        let result = parse_recent_paths(json);
        assert_eq!(
            result,
            vec!["file:///home/user/project1", "file:///home/user/project2",]
        );
    }

    #[test]
    fn test_parse_recent_paths_empty_entries() {
        let json = r#"{"entries": []}"#;
        let result = parse_recent_paths(json);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_recent_paths_no_entries_key() {
        let json = r#"{"other": "value"}"#;
        let result = parse_recent_paths(json);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_recent_paths_invalid_json() {
        let result = parse_recent_paths("not valid json");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_recent_paths_skips_entries_without_folder_uri_or_workspace() {
        let json = r#"{
            "entries": [
                {"folderUri": "file:///home/user/project1"},
                {"other": "value"},
                {"folderUri": "file:///home/user/project3"}
            ]
        }"#;
        let result = parse_recent_paths(json);
        assert_eq!(
            result,
            vec!["file:///home/user/project1", "file:///home/user/project3",]
        );
    }

    #[test]
    fn test_parse_recent_paths_includes_workspace_config_path() {
        let json = r#"{
            "entries": [
                {"folderUri": "file:///home/user/project1"},
                {"workspace": {"id": "abc123", "configPath": "file:///home/user/my-project/my-project.code-workspace"}},
                {"folderUri": "file:///home/user/project3"}
            ]
        }"#;
        let result = parse_recent_paths(json);
        assert_eq!(
            result,
            vec![
                "file:///home/user/project1",
                "file:///home/user/my-project/my-project.code-workspace",
                "file:///home/user/project3",
            ]
        );
    }

    #[test]
    fn test_get_recent_workspace_paths_nonexistent_db() {
        let result = get_recent_workspace_paths("/nonexistent/path/state.vscdb");
        assert!(result.is_empty());
    }
}
