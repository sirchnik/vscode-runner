use std::process::Command;

use krunner::{Match, RunnerExt};
use log::{error, info};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

mod database;
mod matching;
mod vscode;

use vscode::{VSCode, VSCodeVersion};

const SERVICE_NAME: &str = "sirchnik.vscode_krunner";
const OBJECT_PATH: &str = "/vscode_krunner";

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
    let output = Command::new("pidof").arg("vscode_krunner").output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                return;
            }
            let stdout = String::from_utf8_lossy(&result.stdout);
            let pids: Vec<&str> = stdout.split_whitespace().collect();
            if pids.len() > 1 {
                eprintln!(
                    "An instance of vscode_krunner appears to already be running. \
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
        if executable_exists("antigravity-ide") {
            info!("Found Antigravity");
            instances.push(VSCode::new(VSCodeVersion::AntigravityIde));
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
            for db_path in instance.version.db_paths() {
                let path = std::path::PathBuf::from(&db_path);
                if path.exists() {
                    let version = instance.version;
                    info!("Watching database file at {db_path}");

                    let watcher = notify::recommended_watcher(
                        move |res: Result<notify::Event, notify::Error>| {
                            if let Ok(event) = res {
                                if event.kind.is_modify() {
                                    info!("Database file changed for {:?}", version);
                                }
                            }
                        },
                    );

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

        let recent_paths = instance.recent_workspace_paths();
        matching::build_krunner_matches(
            query,
            &recent_paths,
            version.icon_name(),
            version.id_prefix(),
        )
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

fn open_workspace(uri: &str, version: VSCodeVersion) {
    let executable = version.executable();
    info!("Opening workspace at {uri} with {executable}");

    let arg = if uri.ends_with(".code-workspace") {
        format!("--file-uri={uri}")
    } else {
        format!("--folder-uri={uri}")
    };

    let result = Command::new(executable).arg(arg).spawn();

    match result {
        Ok(_) => info!("Successfully opened workspace"),
        Err(e) => error!("Issue opening workspace: {e}"),
    }
}

fn open_containing_folder(path: &str) {
    let clean_path = matching::path_to_uri(path);
    info!("Opening containing folder at {clean_path}");

    let result = Command::new("xdg-open").arg(&clean_path).spawn();

    match result {
        Ok(_) => info!("Successfully opened containing folder"),
        Err(e) => error!("Issue opening containing folder: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executable_exists_with_known_binary() {
        // `ls` should exist on any Linux system
        assert!(executable_exists("ls"));
    }

    #[test]
    fn test_executable_exists_with_unknown_binary() {
        assert!(!executable_exists("definitely_not_a_real_binary_xyz"));
    }
}
