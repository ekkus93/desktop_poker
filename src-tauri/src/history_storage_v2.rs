//! Tournament-aware, crash-recoverable version 2 hand-history storage.
//!
//! The original hand-history.json is read-only legacy evidence. New hands use
//! the stable (tableId, sessionEpoch, handNumber) identity and are written to
//! hand-history.v2.json in the same per-profile directory.

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::State;
use tempfile::NamedTempFile;

use crate::{
    app_state::DesktopAppState,
    history_storage::{PersistedHandHistory, PersistedHistoryEntry},
};

const V2_FILE_NAME: &str = "hand-history.v2.json";
const V1_FILE_NAME: &str = "hand-history.json";
const V2_SCHEMA_VERSION: u8 = 2;
const LEGACY_DURABLE_ORIGIN: &str = "durable-v1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum AtomicWriteStage {
    Create,
    Write,
    FileSync,
    Replace,
    #[cfg(unix)]
    DirectorySync,
}

fn fail_at(
    requested: Option<AtomicWriteStage>,
    stage: AtomicWriteStage,
    name: &str,
) -> Result<(), String> {
    if requested == Some(stage) {
        Err(format!("injected V2 history {name} failure"))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum HistoryProvenance {
    Live,
    ImportedDurableV1,
    ImportedCacheV1,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHistoryEntryV2 {
    #[serde(flatten)]
    pub hand: PersistedHistoryEntry,
    pub provenance: HistoryProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_epoch: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_origin_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tournament_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_started_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHandHistoryV2 {
    pub schema_version: u8,
    pub updated_at_ms: u64,
    pub entries: Vec<PersistedHistoryEntryV2>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum PersistedHistoryIdentity {
    Live {
        table_id: String,
        session_epoch: u64,
        hand_number: u32,
    },
    Legacy {
        provenance: HistoryProvenance,
        origin_id: String,
        hand_number: u32,
    },
}

fn validate_entry(entry: &PersistedHistoryEntryV2) -> Result<PersistedHistoryIdentity, String> {
    if entry.hand.hand_number == 0 {
        return Err("V2 hand history has a non-positive hand number".to_string());
    }
    if entry.hand.summary.len() > 16 * 1024 || entry.hand.board_cards.len() > 7 {
        return Err("V2 hand history contains oversized fields".to_string());
    }
    match entry.provenance {
        HistoryProvenance::Live => {
            let table = entry
                .table_id
                .as_deref()
                .filter(|value| !value.trim().is_empty() && value.len() <= 256)
                .ok_or_else(|| "live V2 hand is missing a valid tableId".to_string())?;
            let epoch = entry
                .session_epoch
                .filter(|value| *value > 0)
                .ok_or_else(|| "live V2 hand is missing a valid sessionEpoch".to_string())?;
            if entry.legacy_origin_id.is_some() {
                return Err("live V2 hand must not carry a legacy origin".to_string());
            }
            Ok(PersistedHistoryIdentity::Live {
                table_id: table.to_string(),
                session_epoch: epoch,
                hand_number: entry.hand.hand_number,
            })
        }
        HistoryProvenance::ImportedDurableV1 | HistoryProvenance::ImportedCacheV1 => {
            if entry.table_id.is_some() || entry.session_epoch.is_some() {
                return Err("legacy V2 hand must not invent table identity".to_string());
            }
            let origin = entry
                .legacy_origin_id
                .as_deref()
                .filter(|value| !value.trim().is_empty() && value.len() <= 256)
                .ok_or_else(|| "legacy V2 hand must have a stable origin".to_string())?;
            Ok(PersistedHistoryIdentity::Legacy {
                provenance: entry.provenance,
                origin_id: origin.to_string(),
                hand_number: entry.hand.hand_number,
            })
        }
    }
}

fn normalize_entries(
    entries: Vec<PersistedHistoryEntryV2>,
) -> Result<Vec<PersistedHistoryEntryV2>, String> {
    let mut by_identity = BTreeMap::new();
    for entry in entries {
        let key = validate_entry(&entry)?;
        by_identity.insert(key, entry);
    }
    let mut entries: Vec<_> = by_identity.into_values().collect();
    entries.sort_by(|left, right| {
        right
            .session_epoch
            .unwrap_or_default()
            .cmp(&left.session_epoch.unwrap_or_default())
            .then_with(|| right.hand.hand_number.cmp(&left.hand.hand_number))
            .then_with(|| left.table_id.cmp(&right.table_id))
            .then_with(|| left.legacy_origin_id.cmp(&right.legacy_origin_id))
    });
    Ok(entries)
}

fn read_v2(path: &Path) -> Result<Option<PersistedHandHistoryV2>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("V2 hand history could not be read: {error}")),
    };
    let history: PersistedHandHistoryV2 = serde_json::from_str(&contents)
        .map_err(|error| format!("V2 hand history is invalid: {error}"))?;
    if history.schema_version != V2_SCHEMA_VERSION {
        return Err("unsupported V2 hand history schemaVersion".to_string());
    }
    Ok(Some(PersistedHandHistoryV2 {
        entries: normalize_entries(history.entries)?,
        ..history
    }))
}

fn import_legacy_v1(path: &Path) -> Result<Option<PersistedHandHistoryV2>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("legacy V1 hand history could not be read: {error}")),
    };
    let history: PersistedHandHistory = serde_json::from_str(&contents)
        .map_err(|error| format!("legacy V1 history is invalid (original preserved): {error}"))?;
    let entries = history
        .entries
        .into_iter()
        .map(|hand| PersistedHistoryEntryV2 {
            hand,
            provenance: HistoryProvenance::ImportedDurableV1,
            table_id: None,
            session_epoch: None,
            legacy_origin_id: Some(LEGACY_DURABLE_ORIGIN.to_string()),
            tournament_name: None,
            session_started_at_ms: None,
        })
        .collect();
    Ok(Some(PersistedHandHistoryV2 {
        schema_version: V2_SCHEMA_VERSION,
        updated_at_ms: history.updated_at_ms,
        entries: normalize_entries(entries)?,
    }))
}

fn atomic_write_v2(path: &Path, history: &PersistedHandHistoryV2) -> Result<(), String> {
    atomic_write_v2_with_fault(path, history, None)
}

fn atomic_write_v2_with_fault(
    path: &Path,
    history: &PersistedHandHistoryV2,
    injected_failure: Option<AtomicWriteStage>,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "V2 history path has no parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create V2 history directory: {error}"))?;
    let mut bytes = serde_json::to_vec_pretty(history)
        .map_err(|error| format!("failed to encode V2 history: {error}"))?;
    bytes.push(b'\n');
    fail_at(injected_failure, AtomicWriteStage::Create, "create")?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| format!("failed to create temporary V2 history: {error}"))?;
    let halfway = bytes.len() / 2;
    temporary
        .write_all(&bytes[..halfway])
        .map_err(|error| format!("failed to write V2 history: {error}"))?;
    fail_at(injected_failure, AtomicWriteStage::Write, "write")?;
    temporary
        .write_all(&bytes[halfway..])
        .map_err(|error| format!("failed to write V2 history: {error}"))?;
    fail_at(injected_failure, AtomicWriteStage::FileSync, "file sync")?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("failed to sync V2 history: {error}"))?;
    // persist() uses a unique create-new temporary and replaces an existing
    // destination on Windows as well as Unix. The old file remains intact if
    // the move fails; dropping the PersistError's file cleans the temp.
    fail_at(injected_failure, AtomicWriteStage::Replace, "replace")?;
    temporary
        .persist(path)
        .map_err(|error| format!("failed to replace V2 history: {}", error.error))?;
    #[cfg(unix)]
    fail_at(
        injected_failure,
        AtomicWriteStage::DirectorySync,
        "directory sync",
    )?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync V2 history directory: {error}"))?;
    Ok(())
}

fn with_profile_lock<T>(
    profile_directory: &Path,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    fs::create_dir_all(profile_directory)
        .map_err(|error| format!("failed to create history directory: {error}"))?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(profile_directory.join(".hand-history.lock"))
        .map_err(|error| format!("failed to open history lock: {error}"))?;
    lock.lock()
        .map_err(|error| format!("failed to lock history profile: {error}"))?;
    operation()
}

fn read_or_migrate(directory: &Path) -> Result<Option<PersistedHandHistoryV2>, String> {
    let path = directory.join(V2_FILE_NAME);
    if let Some(history) = read_v2(&path)? {
        return Ok(Some(history));
    }
    if let Some(history) = import_legacy_v1(&directory.join(V1_FILE_NAME))? {
        atomic_write_v2(&path, &history)?;
        return Ok(Some(history));
    }
    Ok(None)
}

fn now_epoch_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn merge_v2(
    directory: &Path,
    incoming: Vec<PersistedHistoryEntryV2>,
) -> Result<PersistedHandHistoryV2, String> {
    // Validate external data before acquiring any filesystem locks.
    for entry in &incoming {
        validate_entry(entry)?;
    }
    with_profile_lock(directory, || {
        let existing = read_or_migrate(directory)?;
        let mut entries = existing
            .as_ref()
            .map(|history| history.entries.clone())
            .unwrap_or_default();
        #[cfg(test)]
        if let Ok(delay_ms) = std::env::var("DESKTOP_POKER_TEST_HISTORY_LOCK_DELAY_MS") {
            if let Ok(delay_ms) = delay_ms.parse::<u64>() {
                // Child-process regression hook: make an unlocked lost update
                // deterministic while keeping production code unchanged.
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
        entries.extend(incoming);
        let entries = normalize_entries(entries)?;
        if let Some(ref previous) = existing {
            if previous.entries == entries {
                return Ok(previous.clone());
            }
        }
        let history = PersistedHandHistoryV2 {
            schema_version: V2_SCHEMA_VERSION,
            updated_at_ms: now_epoch_ms()?,
            entries,
        };
        atomic_write_v2(&directory.join(V2_FILE_NAME), &history)?;
        Ok(history)
    })
}

#[tauri::command]
pub fn read_persisted_hand_history_v2(
    state: State<'_, DesktopAppState>,
) -> Result<Option<PersistedHandHistoryV2>, String> {
    let directory = PathBuf::from(state.bootstrap().profile_directory);
    with_profile_lock(&directory, || read_or_migrate(&directory))
}

#[tauri::command]
pub fn merge_persisted_hand_history_v2(
    state: State<'_, DesktopAppState>,
    entries: Vec<PersistedHistoryEntryV2>,
) -> Result<PersistedHandHistoryV2, String> {
    merge_v2(&PathBuf::from(state.bootstrap().profile_directory), entries)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use tempfile::tempdir;

    use super::*;

    fn sample_hand(number: u32, summary: &str) -> PersistedHistoryEntry {
        PersistedHistoryEntry {
            hand_number: number,
            summary: summary.to_string(),
            pot_total: 40,
            winning_players: vec!["Alice".to_string()],
            eliminated_players: vec![],
            board_cards: vec![],
        }
    }

    fn live(table: &str, epoch: u64, hand: u32) -> PersistedHistoryEntryV2 {
        PersistedHistoryEntryV2 {
            hand: sample_hand(hand, "Won"),
            provenance: HistoryProvenance::Live,
            table_id: Some(table.to_string()),
            session_epoch: Some(epoch),
            legacy_origin_id: None,
            tournament_name: Some("Tournament".to_string()),
            session_started_at_ms: None,
        }
    }

    #[test]
    fn two_tournaments_can_keep_the_same_hand_number() {
        let directory = tempdir().unwrap();
        merge_v2(directory.path(), vec![live("A", 1, 1)]).unwrap();
        merge_v2(directory.path(), vec![live("B", 2, 1)]).unwrap();
        let persisted = read_v2(&directory.path().join(V2_FILE_NAME))
            .unwrap()
            .unwrap();
        assert_eq!(persisted.entries.len(), 2);
        assert_eq!(persisted.entries[0].table_id.as_deref(), Some("B"));
    }

    #[test]
    fn migrates_v1_without_modifying_original_and_is_idempotent() {
        let directory = tempdir().unwrap();
        let previous = PersistedHandHistory {
            updated_at_ms: 100,
            entries: vec![sample_hand(1, "Legacy")],
        };
        let bytes = serde_json::to_vec(&previous).unwrap();
        fs::write(directory.path().join(V1_FILE_NAME), &bytes).unwrap();
        let first = with_profile_lock(directory.path(), || read_or_migrate(directory.path()))
            .unwrap()
            .unwrap();
        let again = with_profile_lock(directory.path(), || read_or_migrate(directory.path()))
            .unwrap()
            .unwrap();
        assert_eq!(first, again);
        assert_eq!(
            fs::read(directory.path().join(V1_FILE_NAME)).unwrap(),
            bytes
        );
        assert_eq!(
            again.entries[0].provenance,
            HistoryProvenance::ImportedDurableV1
        );
        assert_eq!(again.entries[0].table_id, None);
    }

    #[test]
    fn unsupported_v2_is_preserved_and_fails_closed() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(V2_FILE_NAME);
        let original = br#"{"schemaVersion":3,"updatedAtMs":10,"entries":[]}"#;
        fs::write(&path, original).unwrap();

        assert!(read_v2(&path)
            .unwrap_err()
            .contains("unsupported V2 hand history schemaVersion"));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn update_within_one_tournament_replaces_only_that_hand() {
        let directory = tempdir().unwrap();
        let mut original = live("A", 1, 1);
        original.hand.summary = "Earlier summary".to_string();
        merge_v2(directory.path(), vec![original]).unwrap();
        let mut update = live("A", 1, 1);
        update.hand.summary = "Authoritative update".to_string();
        merge_v2(directory.path(), vec![update]).unwrap();

        let history = read_v2(&directory.path().join(V2_FILE_NAME))
            .unwrap()
            .unwrap();
        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].hand.summary, "Authoritative update");
    }

    #[test]
    fn v1_reader_can_roll_back_without_deleting_v2_then_reupgrade() {
        let directory = tempdir().unwrap();
        let legacy = PersistedHandHistory {
            updated_at_ms: 100,
            entries: vec![sample_hand(1, "Legacy")],
        };
        let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
        let legacy_path = directory.path().join(V1_FILE_NAME);
        fs::write(&legacy_path, &legacy_bytes).unwrap();
        read_or_migrate(directory.path()).unwrap();
        merge_v2(directory.path(), vec![live("new-tournament", 2, 1)]).unwrap();

        // An older binary still reads the unchanged V1 file; its view omits
        // V2-only records, while re-upgrading retains all records in V2.
        let old_binary_view: PersistedHandHistory =
            serde_json::from_slice(&fs::read(&legacy_path).unwrap()).unwrap();
        assert_eq!(old_binary_view.entries, legacy.entries);
        assert_eq!(fs::read(&legacy_path).unwrap(), legacy_bytes);
        let upgraded = read_or_migrate(directory.path()).unwrap().unwrap();
        assert_eq!(upgraded.entries.len(), 2);
    }

    #[test]
    fn rejects_ambiguous_or_incomplete_tournament_identity() {
        let directory = tempdir().unwrap();
        let invalid = live("A", 0, 1);
        assert!(merge_v2(directory.path(), vec![invalid]).is_err());
        assert!(!directory.path().join(V2_FILE_NAME).exists());
    }

    #[test]
    fn serialized_schema_and_identity_validation_are_stable() {
        let entry = live("table-A", 42, 3);
        let value = serde_json::to_value(PersistedHandHistoryV2 {
            schema_version: V2_SCHEMA_VERSION,
            updated_at_ms: 100,
            entries: vec![entry.clone()],
        })
        .unwrap();
        assert_eq!(value["schemaVersion"], 2);
        assert_eq!(value["entries"][0]["tableId"], "table-A");
        assert_eq!(value["entries"][0]["sessionEpoch"], 42);
        assert_eq!(value["entries"][0]["provenance"], "live");
        assert!(value["entries"][0].get("table_id").is_none());
        assert!(validate_entry(&entry).is_ok());

        let mut invalid = entry.clone();
        invalid.table_id = Some("  ".to_string());
        assert!(validate_entry(&invalid).is_err());
        invalid = entry.clone();
        invalid.table_id = None;
        assert!(validate_entry(&invalid).is_err());
        invalid = entry.clone();
        invalid.session_epoch = None;
        assert!(validate_entry(&invalid).is_err());
        invalid = entry.clone();
        invalid.legacy_origin_id = Some("cache-v1:x".to_string());
        assert!(validate_entry(&invalid).is_err());

        invalid.provenance = HistoryProvenance::ImportedDurableV1;
        invalid.table_id = Some("invented-table".to_string());
        invalid.session_epoch = None;
        invalid.legacy_origin_id = Some("durable-v1".to_string());
        assert!(validate_entry(&invalid).is_err());
        invalid.table_id = None;
        invalid.legacy_origin_id = None;
        assert!(validate_entry(&invalid).is_err());
    }

    #[test]
    fn concurrent_merges_retain_disjoint_sessions() {
        let directory = tempdir().unwrap();
        let path = directory.path().to_path_buf();
        let handles: Vec<_> = (1..=8)
            .map(|epoch| {
                let directory = path.clone();
                std::thread::spawn(move || {
                    merge_v2(&directory, vec![live("A", epoch, 1)]).unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let final_store = read_v2(&path.join(V2_FILE_NAME)).unwrap().unwrap();
        assert_eq!(final_store.entries.len(), 8);
    }

    #[test]
    fn profile_lock_serializes_disjoint_process_writers() {
        let directory = tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let mut children: Vec<_> = (1..=4)
            .map(|epoch| {
                Command::new(&executable)
                    .args([
                        "--exact",
                        "history_storage_v2::tests::interprocess_writer_child",
                    ])
                    .env("DESKTOP_POKER_HISTORY_TEST_PROFILE", directory.path())
                    .env("DESKTOP_POKER_HISTORY_TEST_EPOCH", epoch.to_string())
                    .env("DESKTOP_POKER_TEST_HISTORY_LOCK_DELAY_MS", "100")
                    .spawn()
                    .unwrap()
            })
            .collect();

        for child in &mut children {
            assert!(child.wait().unwrap().success());
        }

        let history = with_profile_lock(directory.path(), || read_or_migrate(directory.path()))
            .unwrap()
            .unwrap();
        assert_eq!(history.entries.len(), 4);
    }

    #[test]
    fn interprocess_writer_child() {
        let Ok(profile) = std::env::var("DESKTOP_POKER_HISTORY_TEST_PROFILE") else {
            return;
        };
        let epoch = std::env::var("DESKTOP_POKER_HISTORY_TEST_EPOCH")
            .unwrap()
            .parse()
            .unwrap();
        merge_v2(Path::new(&profile), vec![live("shared", epoch, 1)]).unwrap();
    }

    #[test]
    fn corrupt_v2_is_not_overwritten_by_a_new_merge() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(V2_FILE_NAME);
        let original = b"{invalid-v2";
        fs::write(&path, original).unwrap();
        assert!(merge_v2(directory.path(), vec![live("A", 1, 1)]).is_err());
        assert_eq!(fs::read(&path).unwrap(), original.to_vec());
    }

    #[test]
    fn corrupt_legacy_v1_is_preserved_without_creating_v2() {
        let directory = tempdir().unwrap();
        let legacy_path = directory.path().join(V1_FILE_NAME);
        let original = b"{invalid-v1";
        fs::write(&legacy_path, original).unwrap();
        assert!(with_profile_lock(directory.path(), || read_or_migrate(directory.path())).is_err());
        assert_eq!(fs::read(&legacy_path).unwrap(), original.to_vec());
        assert!(!directory.path().join(V2_FILE_NAME).exists());
    }

    #[test]
    fn repeating_an_identical_merge_does_not_rewrite_the_store() {
        let directory = tempdir().unwrap();
        let hand = live("A", 1, 1);
        let first = merge_v2(directory.path(), vec![hand.clone()]).unwrap();
        let path = directory.path().join(V2_FILE_NAME);
        let original_bytes = fs::read(&path).unwrap();
        let second = merge_v2(directory.path(), vec![hand]).unwrap();
        assert_eq!(first, second);
        assert_eq!(fs::read(&path).unwrap(), original_bytes);
    }

    #[test]
    fn replacement_is_atomic_and_leaves_no_temporary_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(V2_FILE_NAME);
        merge_v2(directory.path(), vec![live("A", 1, 1)]).unwrap();
        let old_bytes = fs::read(&path).unwrap();

        merge_v2(directory.path(), vec![live("B", 2, 1)]).unwrap();

        assert_ne!(fs::read(&path).unwrap(), old_bytes);
        assert_eq!(read_v2(&path).unwrap().unwrap().entries.len(), 2);
        let files: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert!(files
            .iter()
            .all(|name| { name == V2_FILE_NAME || name == ".hand-history.lock" }));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn atomic_write_failures_preserve_the_previous_store_until_replace() {
        let directory = tempdir().unwrap();
        let path = directory.path().join(V2_FILE_NAME);
        let first = PersistedHandHistoryV2 {
            schema_version: V2_SCHEMA_VERSION,
            updated_at_ms: 10,
            entries: vec![live("A", 1, 1)],
        };
        let replacement = PersistedHandHistoryV2 {
            schema_version: V2_SCHEMA_VERSION,
            updated_at_ms: 20,
            entries: vec![live("B", 2, 1)],
        };
        atomic_write_v2(&path, &first).unwrap();
        let previous_bytes = fs::read(&path).unwrap();

        for stage in [
            AtomicWriteStage::Create,
            AtomicWriteStage::Write,
            AtomicWriteStage::FileSync,
            AtomicWriteStage::Replace,
        ] {
            assert!(atomic_write_v2_with_fault(&path, &replacement, Some(stage)).is_err());
            assert_eq!(fs::read(&path).unwrap(), previous_bytes);
            assert!(fs::read_dir(directory.path()).unwrap().all(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name == V2_FILE_NAME)
            }));
        }

        #[cfg(unix)]
        {
            assert!(atomic_write_v2_with_fault(
                &path,
                &replacement,
                Some(AtomicWriteStage::DirectorySync)
            )
            .is_err());
            let persisted = read_v2(&path).unwrap().unwrap();
            assert_eq!(persisted.entries, replacement.entries);
        }
    }

    #[test]
    fn profile_directories_remain_isolated_across_restarts() {
        let root = tempdir().unwrap();
        let alice = root.path().join("alice");
        let bob = root.path().join("bob");
        merge_v2(&alice, vec![live("shared-table", 100, 1)]).unwrap();
        merge_v2(&bob, vec![live("shared-table", 200, 1)]).unwrap();
        let alice_history = with_profile_lock(&alice, || read_or_migrate(&alice))
            .unwrap()
            .unwrap();
        let bob_history = with_profile_lock(&bob, || read_or_migrate(&bob))
            .unwrap()
            .unwrap();
        assert_eq!(alice_history.entries.len(), 1);
        assert_eq!(bob_history.entries.len(), 1);
        assert_eq!(alice_history.entries[0].session_epoch, Some(100));
        assert_eq!(bob_history.entries[0].session_epoch, Some(200));
    }

    #[test]
    fn history_profile_lock_preserves_existing_lock_file_contents() {
        let directory = tempdir().unwrap();
        let lock_path = directory.path().join(".hand-history.lock");
        let existing = b"synthetic owner metadata";
        fs::write(&lock_path, existing).unwrap();

        merge_v2(directory.path(), vec![live("A", 1, 1)]).unwrap();
        let store = with_profile_lock(directory.path(), || read_or_migrate(directory.path()))
            .unwrap()
            .unwrap();

        assert_eq!(store.entries.len(), 1);
        assert_eq!(fs::read(&lock_path).unwrap(), existing);
    }

    #[test]
    fn failed_history_lock_acquisition_preserves_existing_v2_bytes() {
        let directory = tempdir().unwrap();
        merge_v2(directory.path(), vec![live("A", 1, 1)]).unwrap();
        let history_path = directory.path().join(V2_FILE_NAME);
        let original = fs::read(&history_path).unwrap();

        let lock_path = directory.path().join(".hand-history.lock");
        fs::remove_file(&lock_path).unwrap();
        fs::create_dir(&lock_path).unwrap();

        assert!(merge_v2(directory.path(), vec![live("B", 2, 1)]).is_err());
        assert_eq!(fs::read(&history_path).unwrap(), original);
    }
}
