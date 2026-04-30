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
            Self::Stable => "visual-studio-code", // com.visualstudio.code.oss for oss
            Self::Insiders => "visual-studio-code-insiders",
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
        let config_dir = dirs::home_dir()?;
        let subpath = match self {
            Self::Stable => ".vscode-shared/sharedStorage/state.vscdb",
            Self::Insiders => ".vscode-insiders-shared/sharedStorage/state.vscdb",
            Self::Codium => ".vscodium-shared/sharedStorage/state.vscdb",
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
        // db_path depends on the system config dir, but we can verify the suffix
        if let Some(path) = VSCodeVersion::Stable.db_path() {
            assert!(path.ends_with(".vscode-shared/sharedStorage/state.vscdb"));
        }
        if let Some(path) = VSCodeVersion::Insiders.db_path() {
            assert!(path.ends_with(".vscode-insiders-shared/sharedStorage/state.vscdb"));
        }
        if let Some(path) = VSCodeVersion::Codium.db_path() {
            assert!(path.ends_with(".vscodium-shared/sharedStorage/state.vscdb"));
        }
    }
}
