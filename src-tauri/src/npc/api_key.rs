use std::{
    fs,
    path::{Path, PathBuf},
};

const KEY_FILE_NAME: &str = "claude-api-key.txt";

fn key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(KEY_FILE_NAME)
}

/// Read the Claude API key from `{app_data_dir}/claude-api-key.txt`.
///
/// Returns `None` if the file does not exist or is empty.
pub fn load_api_key(app_data_dir: &Path) -> Option<String> {
    let path = key_path(app_data_dir);
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Write the trimmed key to `{app_data_dir}/claude-api-key.txt`.
///
/// If `key` is empty after trimming, the file is deleted (clearing the key).
pub fn save_api_key(app_data_dir: &Path, key: &str) -> Result<(), std::io::Error> {
    let trimmed = key.trim();
    let path = key_path(app_data_dir);

    if trimmed.is_empty() {
        if path.exists() {
            fs::remove_file(&path)?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, trimmed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_returns_same_key() {
        let dir = tempfile::tempdir().unwrap();
        save_api_key(dir.path(), "sk-test-key-abc123").unwrap();
        let loaded = load_api_key(dir.path());
        assert_eq!(loaded, Some("sk-test-key-abc123".to_string()));
    }

    #[test]
    fn save_empty_deletes_key_file() {
        let dir = tempfile::tempdir().unwrap();
        save_api_key(dir.path(), "sk-test-key").unwrap();
        save_api_key(dir.path(), "").unwrap();
        assert!(load_api_key(dir.path()).is_none());
        assert!(!key_path(dir.path()).exists());
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_api_key(dir.path()).is_none());
    }

    #[test]
    fn key_is_trimmed_on_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        save_api_key(dir.path(), "  sk-key  \n").unwrap();
        assert_eq!(load_api_key(dir.path()), Some("sk-key".to_string()));
    }
}
