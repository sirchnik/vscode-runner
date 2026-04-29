#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VSCodeVersion {
    Stable,
    Insiders,
    Codium,
}

impl VSCodeVersion {
    pub fn executable(&self) -> &'static str {
        match self {
            Self::Stable => "code",
            Self::Insiders => "code-insiders",
            Self::Codium => "codium",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Stable => "vscode",
            Self::Insiders => "vscode-insiders",
            Self::Codium => "vscodium",
        }
    }

    pub fn id_prefix(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Insiders => "insiders",
            Self::Codium => "codium",
        }
    }

    pub fn from_id_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "stable" => Some(Self::Stable),
            "insiders" => Some(Self::Insiders),
            "codium" => Some(Self::Codium),
            _ => None,
        }
    }

    pub fn db_path(&self) -> Option<String> {
        let config_dir = dirs::config_dir()?;
        let subpath = match self {
            Self::Stable => "Code/User/globalStorage/state.vscdb",
            Self::Insiders => "Code - Insiders/User/globalStorage/state.vscdb",
            Self::Codium => "VSCodium/User/globalStorage/state.vscdb",
        };
        Some(config_dir.join(subpath).to_string_lossy().to_string())
    }
}

pub struct VSCode {
    pub version: VSCodeVersion,
    db_path: Option<String>,
}

impl VSCode {
    pub fn new(version: VSCodeVersion) -> Self {
        Self {
            version,
            db_path: version.db_path(),
        }
    }

    pub fn recent_workspace_paths(&self) -> Vec<String> {
        match &self.db_path {
            Some(path) => crate::database::get_recent_workspace_paths(path),
            None => vec![],
        }
    }
}
