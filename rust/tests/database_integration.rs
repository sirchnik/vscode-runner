use vscode_runner::database::get_recent_workspace_paths;

const TEST_JSON: &str = r#"{"entries":[{"folderUri":"file:///home/user/Projects/alpha"},{"folderUri":"file:///home/user/Projects/beta"},{"folderUri":"file:///home/user/Projects/gamma"},{"workspace":{"configPath":"/some/workspace.code-workspace"}},{"folderUri":"vscode-remote://ssh-remote%2Bmyserver/home/user/remote-project"}]}"#;

fn create_test_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("state.vscdb");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ItemTable (key, value) VALUES (?1, ?2)",
        rusqlite::params!["history.recentlyOpenedPathsList", TEST_JSON],
    )
    .unwrap();

    let path_str = db_path.to_string_lossy().to_string();
    (dir, path_str)
}

#[test]
fn reads_recent_workspace_paths_from_vscdb() {
    let (_dir, db_path) = create_test_db();
    let paths = get_recent_workspace_paths(&db_path);

    assert_eq!(paths.len(), 4);
    assert_eq!(paths[0], "file:///home/user/Projects/alpha");
    assert_eq!(paths[1], "file:///home/user/Projects/beta");
    assert_eq!(paths[2], "file:///home/user/Projects/gamma");
    assert_eq!(
        paths[3],
        "vscode-remote://ssh-remote%2Bmyserver/home/user/remote-project"
    );
}

#[test]
fn skips_entries_without_folder_uri() {
    let (_dir, db_path) = create_test_db();
    let paths = get_recent_workspace_paths(&db_path);

    // The test database has 5 entries total but one is a workspace entry
    // without folderUri — it should be skipped.
    assert_eq!(paths.len(), 4);
    assert!(!paths.iter().any(|p| p.contains("workspace")));
}

#[test]
fn returns_empty_for_nonexistent_db() {
    let paths = get_recent_workspace_paths("/nonexistent/path/to/state.vscdb");
    assert!(paths.is_empty());
}

#[test]
fn returns_empty_for_db_without_recent_paths_key() {
    // Create a temporary database with no relevant key.
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("empty.vscdb");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);")
        .unwrap();

    let paths = get_recent_workspace_paths(db_path.to_str().unwrap());
    assert!(paths.is_empty());
}
