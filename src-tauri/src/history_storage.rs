use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::DesktopAppState;

const HISTORY_FILE_NAME: &str = "hand-history.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHistoryCard {
    pub label: String,
    pub compact_label: String,
    pub suit_symbol: String,
    pub tone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHistoryEntry {
    pub hand_number: u32,
    pub summary: String,
    pub pot_total: u32,
    pub winning_players: Vec<String>,
    pub eliminated_players: Vec<String>,
    pub board_cards: Vec<PersistedHistoryCard>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHandHistory {
    pub updated_at_ms: u64,
    pub entries: Vec<PersistedHistoryEntry>,
}

#[tauri::command]
pub fn read_persisted_hand_history(
    state: State<'_, DesktopAppState>,
) -> Result<Option<PersistedHandHistory>, String> {
    read_history_file(&history_path(&state))
}

#[tauri::command]
pub fn merge_persisted_hand_history(
    state: State<'_, DesktopAppState>,
    entries: Vec<PersistedHistoryEntry>,
) -> Result<PersistedHandHistory, String> {
    merge_history_file(&history_path(&state), entries)
}

fn history_path(state: &DesktopAppState) -> PathBuf {
    PathBuf::from(state.bootstrap().profile_directory).join(HISTORY_FILE_NAME)
}

fn read_history_file(path: &Path) -> Result<Option<PersistedHandHistory>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "saved hand history exists but cannot be read ({}): {error}",
                path.display()
            ))
        }
    };

    let parsed: PersistedHandHistory = serde_json::from_str(&text).map_err(|error| {
        format!(
            "saved hand history is invalid JSON or has an unsupported schema ({}): {error}",
            path.display()
        )
    })?;

    Ok(Some(PersistedHandHistory {
        updated_at_ms: parsed.updated_at_ms,
        entries: normalize_entries(parsed.entries),
    }))
}

fn merge_history_file(
    path: &Path,
    incoming_entries: Vec<PersistedHistoryEntry>,
) -> Result<PersistedHandHistory, String> {
    let mut all_entries = read_history_file(path)?
        .map(|history| history.entries)
        .unwrap_or_default();
    all_entries.extend(incoming_entries);

    let history = PersistedHandHistory {
        updated_at_ms: now_epoch_ms()?,
        entries: normalize_entries(all_entries),
    };
    write_history_file(path, &history)?;
    Ok(history)
}

fn normalize_entries(entries: Vec<PersistedHistoryEntry>) -> Vec<PersistedHistoryEntry> {
    let mut entries_by_hand_number = BTreeMap::new();
    for entry in entries {
        entries_by_hand_number.insert(entry.hand_number, entry);
    }
    entries_by_hand_number
        .into_iter()
        .rev()
        .map(|(_, entry)| entry)
        .collect()
}

fn write_history_file(path: &Path, history: &PersistedHandHistory) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("hand history path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create hand history directory ({}): {error}",
            parent.display()
        )
    })?;

    let mut payload = serde_json::to_vec_pretty(history)
        .map_err(|error| format!("failed to serialize hand history: {error}"))?;
    payload.push(b'\n');

    let temporary_path = parent.join(format!(
        ".{HISTORY_FILE_NAME}.{}.{}.tmp",
        std::process::id(),
        history.updated_at_ms
    ));

    let write_result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|error| {
                format!(
                    "failed to create temporary hand history file ({}): {error}",
                    temporary_path.display()
                )
            })?;
        file.write_all(&payload).map_err(|error| {
            format!(
                "failed to write temporary hand history file ({}): {error}",
                temporary_path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "failed to flush temporary hand history file ({}): {error}",
                temporary_path.display()
            )
        })?;
        drop(file);

        replace_history_file(&temporary_path, path)?;
        sync_directory(parent)?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

#[cfg(not(windows))]
fn replace_history_file(temporary_path: &Path, final_path: &Path) -> Result<(), String> {
    fs::rename(temporary_path, final_path).map_err(|error| {
        format!(
            "failed to atomically replace hand history file ({}): {error}",
            final_path.display()
        )
    })
}

#[cfg(windows)]
fn replace_history_file(temporary_path: &Path, final_path: &Path) -> Result<(), String> {
    let backup_path = final_path.with_extension("json.backup");
    let had_existing_file = final_path.exists();

    if backup_path.exists() {
        fs::remove_file(&backup_path).map_err(|error| {
            format!(
                "failed to remove stale hand history backup ({}): {error}",
                backup_path.display()
            )
        })?;
    }

    if had_existing_file {
        fs::rename(final_path, &backup_path).map_err(|error| {
            format!(
                "failed to stage existing hand history for replacement ({}): {error}",
                final_path.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(temporary_path, final_path) {
        if had_existing_file {
            let _ = fs::rename(&backup_path, final_path);
        }
        return Err(format!(
            "failed to replace hand history file ({}): {error}",
            final_path.display()
        ));
    }

    if had_existing_file {
        fs::remove_file(&backup_path).map_err(|error| {
            format!(
                "hand history was replaced but its backup could not be removed ({}): {error}",
                backup_path.display()
            )
        })?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!(
                "failed to flush hand history directory metadata ({}): {error}",
                path.display()
            )
        })
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn now_epoch_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn entry(hand_number: u32, summary: &str) -> PersistedHistoryEntry {
        PersistedHistoryEntry {
            hand_number,
            summary: summary.to_string(),
            pot_total: hand_number * 100,
            winning_players: vec!["Maya".to_string()],
            eliminated_players: Vec::new(),
            board_cards: Vec::new(),
        }
    }

    #[test]
    fn missing_history_is_not_an_error() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join(HISTORY_FILE_NAME);
        assert_eq!(read_history_file(&path).expect("read should succeed"), None);
    }

    #[test]
    fn merge_is_durable_sorted_and_duplicate_free() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join(HISTORY_FILE_NAME);

        merge_history_file(&path, vec![entry(1, "old one"), entry(2, "two")])
            .expect("first merge");
        let merged = merge_history_file(&path, vec![entry(1, "new one"), entry(3, "three")])
            .expect("second merge");

        assert_eq!(
            merged
                .entries
                .iter()
                .map(|entry| (entry.hand_number, entry.summary.as_str()))
                .collect::<Vec<_>>(),
            vec![(3, "three"), (2, "two"), (1, "new one")]
        );
        assert_eq!(
            read_history_file(&path)
                .expect("persisted read")
                .expect("history should exist")
                .entries,
            merged.entries
        );
    }

    #[test]
    fn corrupt_history_is_reported_and_not_overwritten() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join(HISTORY_FILE_NAME);
        fs::write(&path, "not json").expect("seed corrupt history");

        let error = merge_history_file(&path, vec![entry(1, "one")])
            .expect_err("corrupt history must block merge");
        assert!(error.contains("invalid JSON"));
        assert_eq!(fs::read_to_string(path).expect("corrupt file remains"), "not json");
    }
}
