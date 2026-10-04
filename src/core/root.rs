use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::error::{LedgerError, Result};

pub const CONFIG_FILE: &str = "ledger.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub templates_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            templates_dir: "_templates".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ledger {
    pub root: PathBuf,
    pub config: Config,
}

/// Find the ledger root: `env_root` when given (must contain `ledger.toml`),
/// otherwise walk up from `cwd`. NotFound when absent.
pub fn find_root(cwd: &Path, env_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(root) = env_root {
        let config_path = root.join(CONFIG_FILE);
        if config_path.exists() {
            Ok(root.to_path_buf())
        } else {
            Err(LedgerError::NotFound(format!(
                "LEDGER_ROOT={} does not contain {}",
                root.display(),
                CONFIG_FILE
            )))
        }
    } else {
        for ancestor in cwd.ancestors() {
            if ancestor.join(CONFIG_FILE).exists() {
                return Ok(ancestor.to_path_buf());
            }
        }
        Err(LedgerError::NotFound(format!(
            "no {} found in {} or any parent directory",
            CONFIG_FILE,
            cwd.display()
        )))
    }
}

/// Parse `<root>/ledger.toml`. Empty file = defaults; unknown keys ignored;
/// invalid TOML = Invalid naming the file.
pub fn load_config(root: &Path) -> Result<Config> {
    let config_path = root.join(CONFIG_FILE);
    let contents =
        std::fs::read_to_string(&config_path).map_err(|e| LedgerError::io(&config_path, e))?;

    if contents.trim().is_empty() {
        return Ok(Config::default());
    }

    toml::from_str(&contents)
        .map_err(|e| LedgerError::Invalid(format!("{}: {}", config_path.display(), e)))
}

pub fn discover(cwd: &Path, env_root: Option<&Path>) -> Result<Ledger> {
    let root = find_root(cwd, env_root)?;
    let config = load_config(&root)?;
    Ok(Ledger { root, config })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn root_found_from_nested_directory() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join(CONFIG_FILE), b"").unwrap();

        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();

        let result = find_root(&nested, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root);
    }

    #[test]
    fn env_override_used_regardless_of_cwd() {
        let temp_root = TempDir::new().unwrap();
        let temp_other = TempDir::new().unwrap();

        let root = temp_root.path();
        std::fs::write(root.join(CONFIG_FILE), b"").unwrap();

        let other_cwd = temp_other.path();

        let result = find_root(other_cwd, Some(root));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), root);
    }

    #[test]
    fn env_override_without_ledger_toml_not_found() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let result = find_root(root, Some(root));
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 3);
        assert!(err.to_string().contains("does not contain"));
    }

    #[test]
    fn no_root_anywhere_not_found() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();

        let result = find_root(cwd, None);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 3);
        assert!(err.to_string().contains("no"));
        assert!(err.to_string().contains("found"));
    }

    #[test]
    fn empty_config_returns_defaults() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join(CONFIG_FILE), b"").unwrap();

        let result = load_config(root);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.templates_dir, "_templates");
    }

    #[test]
    fn templates_dir_setting_respected() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join(CONFIG_FILE), b"templates_dir = \"tpl\"\n").unwrap();

        let result = load_config(root);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.templates_dir, "tpl");
    }

    #[test]
    fn unknown_key_ignored() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join(CONFIG_FILE),
            b"templates_dir = \"tpl\"\nunknown_field = 42\n",
        )
        .unwrap();

        let result = load_config(root);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.templates_dir, "tpl");
    }

    #[test]
    fn invalid_toml_returns_invalid_with_exit_code_4() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join(CONFIG_FILE), b"this is not valid toml {{{").unwrap();

        let result = load_config(root);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.exit_code(), 4);
        let msg = err.to_string();
        assert!(msg.contains("ledger.toml"));
    }
}
