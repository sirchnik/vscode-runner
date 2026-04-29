use std::process::Command;

use krunner::{Match, MatchIcon, MatchType, RunnerExt};
use log::{error, info};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

mod database;
mod notifications;
mod vscode;

use vscode::{VSCode, VSCodeVersion};

const SERVICE_NAME: &str = "sirchnik.vscode_runner";
const OBJECT_PATH: &str = "/vscode_runner";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    check_if_already_running();

    info!("Starting VSCode runner.");

    let runner = VscodeRunner::new();
    runner.start(SERVICE_NAME, OBJECT_PATH)?;

    Ok(())
}

/// Check if an instance of this plugin is already running.
/// If we don't check KRunner will just launch a new instance every time.
fn check_if_already_running() {
    let output = Command::new("pidof")
        .arg("vscode_runner")
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                return;
            }
            let stdout = String::from_utf8_lossy(&result.stdout);
            let pids: Vec<&str> = stdout.trim().split_whitespace().collect();
            if pids.len() > 1 {
                eprintln!(
                    "An instance of vscode_runner appears to already be running. \
                     Aborting run of new instance."
                );
                std::process::exit(0);
            }
        }
        Err(e) => {
            eprintln!("Issue checking for existing process: {e}");
        }
    }
}

#[derive(krunner::Action)]
enum Action {
    #[action(
        id = "openContainingFolder",
        title = "Open Containing Folder",
        icon = "document-open-folder"
    )]
    OpenContainingFolder,
}

struct VscodeRunner {
    instances: Vec<VSCode>,
    _watchers: Vec<RecommendedWatcher>,
}

impl VscodeRunner {
    fn new() -> Self {
        let mut instances = Vec::new();

        if executable_exists("code") {
            info!("Found VSCode (stable)");
            instances.push(VSCode::new(VSCodeVersion::Stable));
        }
        if executable_exists("code-insiders") {
            info!("Found VSCode (insiders)");
            instances.push(VSCode::new(VSCodeVersion::Insiders));
        }
        if executable_exists("codium") {
            info!("Found VSCodium");
            instances.push(VSCode::new(VSCodeVersion::Codium));
        }

        if instances.is_empty() {
            error!(
                "Unable to find any instances of VSCode. \
                 Please make sure at least one is installed and on your PATH."
            );
            std::process::exit(1);
        }

        // Set up file watchers for each database file so cached paths refresh.
        let mut watchers = Vec::new();
        for instance in &instances {
            if let Some(db_path) = instance.version.db_path() {
                let path = std::path::PathBuf::from(&db_path);
                if path.exists() {
                    let version = instance.version;
                    info!("Watching database file at {db_path}");

                    let watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = res {
                            if event.kind.is_modify() {
                                info!("Database file changed for {:?}", version);
                            }
                        }
                    });

                    if let Ok(mut w) = watcher {
                        let _ = w.watch(&path, RecursiveMode::NonRecursive);
                        watchers.push(w);
                    }
                }
            }
        }

        Self {
            instances,
            _watchers: watchers,
        }
    }

    fn get_matches_for(&self, query: &str, version: VSCodeVersion) -> Vec<Match<Action>> {
        let instance = match self.instances.iter().find(|i| i.version == version) {
            Some(i) => i,
            None => return vec![],
        };

        let regex = match regex::RegexBuilder::new(query)
            .case_insensitive(true)
            .build()
        {
            Ok(r) => r,
            Err(_) => return vec![],
        };

        let recent_paths = instance.recent_workspace_paths();
        let matching: Vec<&str> = recent_paths
            .iter()
            .map(|s| s.as_str())
            .filter(|path| regex.is_match(path))
            .collect();

        if matching.is_empty() {
            return vec![];
        }

        let icon_name = version.icon_name();
        let home_dir = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        matching
            .into_iter()
            .map(|path| {
                let uri = path_to_uri(path);
                let project_name = path.rsplit('/').next().unwrap_or(path);
                let subtitle = parse_relative_path(&uri, &home_dir).unwrap_or(uri);
                let id_prefix = version.id_prefix();
                let id = format!("{id_prefix}-{path}");

                Match {
                    id,
                    title: project_name.to_owned(),
                    icon: MatchIcon::ByName(icon_name.to_owned()),
                    subtitle: Some(subtitle),
                    ty: MatchType::ExactMatch,
                    relevance: 1.0,
                    ..Match::default()
                }
            })
            .collect()
    }
}

impl krunner::Runner for VscodeRunner {
    type Action = Action;
    type Err = String;

    fn matches(&mut self, query: String) -> Result<Vec<Match<Self::Action>>, Self::Err> {
        info!("Running query for: {query}");

        let mut matches = Vec::new();

        for instance in &self.instances {
            let instance_matches = self.get_matches_for(&query, instance.version);
            info!(
                "Found {} matches for {:?}",
                instance_matches.len(),
                instance.version
            );
            matches.extend(instance_matches);
        }

        Ok(matches)
    }

    fn run(&mut self, match_id: String, action: Option<Self::Action>) -> Result<(), Self::Err> {
        info!("Running action for match_id: {match_id}");

        let (prefix, path) = match match_id.split_once('-') {
            Some(pair) => pair,
            None => return Err(format!("Invalid match_id: {match_id}")),
        };

        let version = VSCodeVersion::from_id_prefix(prefix)
            .ok_or_else(|| format!("Unknown version prefix: {prefix}"))?;

        match action {
            Some(Action::OpenContainingFolder) => {
                open_containing_folder(path);
            }
            None => {
                open_workspace(path, version);
            }
        }

        Ok(())
    }
}

fn executable_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn path_to_uri(path: &str) -> String {
    path.replace("file://", "").replace("vscode-remote://", "")
}

fn parse_relative_path(uri: &str, home_dir: &str) -> Option<String> {
    if !home_dir.is_empty() && uri.contains(home_dir) {
        let without_home = &uri[home_dir.len()..];
        Some(format!("~{without_home}"))
    } else {
        None
    }
}

fn open_workspace(uri: &str, version: VSCodeVersion) {
    let executable = version.executable();
    info!("Opening workspace at {uri} with {executable}");

    let result = Command::new(executable)
        .arg(format!("--folder-uri={uri}"))
        .spawn();

    match result {
        Ok(_) => info!("Successfully opened workspace"),
        Err(e) => error!("Issue opening workspace: {e}"),
    }
}

fn open_containing_folder(path: &str) {
    let clean_path = path_to_uri(path);
    info!("Opening containing folder at {clean_path}");

    let result = Command::new("xdg-open").arg(&clean_path).spawn();

    match result {
        Ok(_) => info!("Successfully opened containing folder"),
        Err(e) => error!("Issue opening containing folder: {e}"),
    }
}

