#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VSCodeVersion {
    Stable,
    Insiders,
    Codium,
    AntigravityIde,
}

impl VSCodeVersion {
    pub fn executable(&self) -> &'static str {
        match self {
            Self::Stable => "code",
            Self::Insiders => "code-insiders",
            Self::Codium => "codium",
            Self::AntigravityIde => "antigravity-ide",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Stable => "visual-studio-code", // com.visualstudio.code.oss for oss
            Self::Insiders => "visual-studio-code-insiders",
            Self::Codium => "vscodium",
            Self::AntigravityIde => "antigravity-ide", // usually antigravity or antigravity-ide
        }
    }

    pub fn id_prefix(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Insiders => "insiders",
            Self::Codium => "codium",
            Self::AntigravityIde => "antigravity",
        }
    }

    pub fn from_id_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "stable" => Some(Self::Stable),
            "insiders" => Some(Self::Insiders),
            "codium" => Some(Self::Codium),
            "antigravity" => Some(Self::AntigravityIde),
            _ => None,
        }
    }

    pub fn db_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            let subpaths_old = match self {
                Self::Stable => vec!["Code/User/globalStorage/state.vscdb"],
                Self::Insiders => vec!["Code - Insiders/User/globalStorage/state.vscdb"],
                Self::Codium => vec!["VSCodium/User/globalStorage/state.vscdb"],
                Self::AntigravityIde => vec![
                    "Antigravity/User/globalStorage/state.vscdb",
                    "Antigravity IDE/User/globalStorage/state.vscdb",
                ],
            };
            for subpath in subpaths_old {
                paths.push(config_dir.join(subpath).to_string_lossy().to_string());
            }
        }

        if let Some(home_dir) = dirs::home_dir() {
            let subpaths_new = match self {
                Self::Stable => vec![".vscode-shared/sharedStorage/state.vscdb"],
                Self::Insiders => vec![".vscode-insiders-shared/sharedStorage/state.vscdb"],
                Self::Codium => vec![".vscodium-shared/sharedStorage/state.vscdb"],
                Self::AntigravityIde => vec![
                    ".antigravity-ide-shared/sharedStorage/state.vscdb",
                    ".antigravity-shared/sharedStorage/state.vscdb",
                ],
            };
            for subpath in subpaths_new {
                paths.push(home_dir.join(subpath).to_string_lossy().to_string());
            }
        }

        paths
    }
}

pub struct VSCode {
    pub version: VSCodeVersion,
    db_paths: Vec<String>,
}

impl VSCode {
    pub fn new(version: VSCodeVersion) -> Self {
        Self {
            version,
            db_paths: version.db_paths(),
        }
    }

    pub fn recent_workspace_paths(&self) -> Vec<String> {
        let mut all_paths = Vec::new();
        for path in &self.db_paths {
            all_paths.extend(crate::database::get_recent_workspace_paths(path));
        }

        let mut unique_paths = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for path in all_paths {
            if seen.insert(path.clone()) {
                unique_paths.push(path);
            }
        }
        unique_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executable_names() {
        assert_eq!(VSCodeVersion::Stable.executable(), "code");
        assert_eq!(VSCodeVersion::Insiders.executable(), "code-insiders");
        assert_eq!(VSCodeVersion::Codium.executable(), "codium");
    }

    #[test]
    fn test_icon_names() {
        assert_eq!(VSCodeVersion::Stable.icon_name(), "visual-studio-code");
        assert_eq!(
            VSCodeVersion::Insiders.icon_name(),
            "visual-studio-code-insiders"
        );
        assert_eq!(VSCodeVersion::Codium.icon_name(), "vscodium");
    }

    #[test]
    fn test_id_prefix() {
        assert_eq!(VSCodeVersion::Stable.id_prefix(), "stable");
        assert_eq!(VSCodeVersion::Insiders.id_prefix(), "insiders");
        assert_eq!(VSCodeVersion::Codium.id_prefix(), "codium");
    }

    #[test]
    fn test_from_id_prefix_valid() {
        assert_eq!(
            VSCodeVersion::from_id_prefix("stable"),
            Some(VSCodeVersion::Stable)
        );
        assert_eq!(
            VSCodeVersion::from_id_prefix("insiders"),
            Some(VSCodeVersion::Insiders)
        );
        assert_eq!(
            VSCodeVersion::from_id_prefix("codium"),
            Some(VSCodeVersion::Codium)
        );
    }

    #[test]
    fn test_from_id_prefix_invalid() {
        assert_eq!(VSCodeVersion::from_id_prefix("unknown"), None);
        assert_eq!(VSCodeVersion::from_id_prefix(""), None);
    }

    #[test]
    fn test_db_path_contains_expected_subpath() {
        let stable_paths = VSCodeVersion::Stable.db_paths();
        assert!(stable_paths
            .iter()
            .any(|p| p.ends_with(".vscode-shared/sharedStorage/state.vscdb")));
        assert!(stable_paths
            .iter()
            .any(|p| p.ends_with("Code/User/globalStorage/state.vscdb")));

        let insiders_paths = VSCodeVersion::Insiders.db_paths();
        assert!(insiders_paths
            .iter()
            .any(|p| p.ends_with(".vscode-insiders-shared/sharedStorage/state.vscdb")));
        assert!(insiders_paths
            .iter()
            .any(|p| p.ends_with("Code - Insiders/User/globalStorage/state.vscdb")));

        let codium_paths = VSCodeVersion::Codium.db_paths();
        assert!(codium_paths
            .iter()
            .any(|p| p.ends_with(".vscodium-shared/sharedStorage/state.vscdb")));
        assert!(codium_paths
            .iter()
            .any(|p| p.ends_with("VSCodium/User/globalStorage/state.vscdb")));

        let antigravity_paths = VSCodeVersion::AntigravityIde.db_paths();
        assert!(antigravity_paths
            .iter()
            .any(|p| p.ends_with(".antigravity-ide-shared/sharedStorage/state.vscdb")));
        assert!(antigravity_paths
            .iter()
            .any(|p| p.ends_with("Antigravity IDE/User/globalStorage/state.vscdb")));
    }
}
