use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = "forkme.toml";
const LOCK_FILE: &str = "forkme.lock";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub upstream: Upstream,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Upstream {
    pub url: String,
    pub branch: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        Self::load_from(CONFIG_FILE)
    }

    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read {}", path.as_ref().display()))?;
        toml::from_str(&content).with_context(|| "Failed to parse forkme.toml")
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(CONFIG_FILE)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write {}", path.as_ref().display()))?;
        Ok(())
    }

    pub fn exists() -> bool {
        Path::new(CONFIG_FILE).exists()
    }
}

pub fn load_lock() -> Result<Option<String>> {
    let path = Path::new(LOCK_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", LOCK_FILE))?;
    let sha = content.trim().to_string();
    if sha.is_empty() {
        return Ok(None);
    }
    Ok(Some(sha))
}

pub fn save_lock(sha: &str) -> Result<()> {
    fs::write(LOCK_FILE, format!("{}\n", sha))
        .with_context(|| format!("Failed to write {}", LOCK_FILE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let config = Config {
            upstream: Upstream {
                url: "https://github.com/test/repo.git".to_string(),
                branch: "main".to_string(),
            },
        };

        config.save_to(path).unwrap();
        let loaded = Config::load_from(path).unwrap();

        assert_eq!(loaded.upstream.url, "https://github.com/test/repo.git");
        assert_eq!(loaded.upstream.branch, "main");
    }

    #[test]
    fn test_config_toml_format() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let config = Config {
            upstream: Upstream {
                url: "https://github.com/test/repo.git".to_string(),
                branch: "develop".to_string(),
            },
        };

        config.save_to(path).unwrap();
        let content = fs::read_to_string(path).unwrap();

        assert!(content.contains("[upstream]"));
        assert!(content.contains("url = \"https://github.com/test/repo.git\""));
        assert!(content.contains("branch = \"develop\""));
    }

    #[test]
    fn test_load_invalid_toml() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        fs::write(path, "invalid toml content {{{").unwrap();
        let result = Config::load_from(path);

        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_field() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        fs::write(path, "[upstream]\nurl = \"test\"").unwrap();
        let result = Config::load_from(path);

        assert!(result.is_err());
    }
}
