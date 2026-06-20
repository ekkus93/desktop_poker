use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::profile::{parse_profile, NpcProfile, ProfileError};

const STARTER_PROFILES: &[(&str, &str)] = &[
    (
        "aggressive-alice",
        include_str!("profiles/aggressive-alice.md"),
    ),
    (
        "conservative-carlos",
        include_str!("profiles/conservative-carlos.md"),
    ),
    ("balanced-sam", include_str!("profiles/balanced-sam.md")),
];

/// Set of IDs that are built-in and cannot be deleted.
pub const BUILTIN_PROFILE_IDS: &[&str] =
    &["aggressive-alice", "conservative-carlos", "balanced-sam"];

/// Returns the profiles directory path: `{app_data_dir}/npc-profiles/`.
pub fn profiles_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("npc-profiles")
}

/// Ensure the profiles directory exists, seeding starter profiles on first run.
pub fn ensure_profiles_dir(dir: &Path) -> Result<(), ProfileError> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
        seed_starter_profiles(dir)?;
    } else if is_empty_dir(dir) {
        seed_starter_profiles(dir)?;
    }
    Ok(())
}

fn is_empty_dir(dir: &Path) -> bool {
    dir.read_dir()
        .map(|mut rd| rd.next().is_none())
        .unwrap_or(false)
}

fn seed_starter_profiles(dir: &Path) -> Result<(), ProfileError> {
    for (id, content) in STARTER_PROFILES {
        let path = dir.join(format!("{id}.md"));
        if !path.exists() {
            fs::write(&path, content)?;
        }
    }
    Ok(())
}

/// A profile file that could not be parsed, included in `NpcProfileListResult`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NpcProfileError {
    /// Filename (not full path) of the failing profile file.
    pub filename: String,
    /// Human-readable parse or I/O error.
    pub error: String,
}

/// Result of listing profiles: valid profiles plus any files that failed to parse.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NpcProfileListResult {
    pub profiles: Vec<NpcProfile>,
    pub errors: Vec<NpcProfileError>,
}

/// List all profiles in `dir`, sorted alphabetically by name.
///
/// Files that fail to parse are included in `errors` rather than silently skipped.
pub fn list_profiles(dir: &Path) -> Result<NpcProfileListResult, ProfileError> {
    ensure_profiles_dir(dir)?;

    let mut profiles = Vec::new();
    let mut errors = Vec::new();

    let entries = fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&stem)
            .to_string();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(NpcProfileError {
                    filename,
                    error: e.to_string(),
                });
                continue;
            }
        };
        match parse_profile(&stem, &content) {
            Ok(p) => profiles.push(p),
            Err(e) => {
                errors.push(NpcProfileError {
                    filename,
                    error: e.to_string(),
                });
            }
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(NpcProfileListResult { profiles, errors })
}

/// Load a single profile by its ID (filename stem).
pub fn load_profile(dir: &Path, id: &str) -> Result<NpcProfile, ProfileError> {
    let path = dir.join(format!("{id}.md"));
    let content = fs::read_to_string(&path)?;
    parse_profile(id, &content)
}

/// Write `content` to `{dir}/{id}.md`, validating it parses before writing.
///
/// Returns the parsed profile on success.
pub fn save_profile(dir: &Path, id: &str, content: &str) -> Result<NpcProfile, ProfileError> {
    ensure_profiles_dir(dir)?;
    // Validate before writing.
    let profile = parse_profile(id, content)?;
    let path = dir.join(format!("{id}.md"));
    fs::write(&path, content)?;
    Ok(profile)
}

/// Delete `{dir}/{id}.md`.
pub fn delete_profile(dir: &Path, id: &str) -> Result<(), ProfileError> {
    let path = dir.join(format!("{id}.md"));
    fs::remove_file(&path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    const SAMPLE: &str = "\
---
name: Sample Player
style: balanced
skill: intermediate
---
A balanced test player.";

    const BAD: &str = "no frontmatter here";

    #[test]
    fn list_profiles_empty_dir_seeds_starters() {
        let dir = temp_dir();
        let result = list_profiles(dir.path()).unwrap();
        // Starter profiles should have been seeded.
        assert_eq!(result.profiles.len(), 3);
        assert!(result.errors.is_empty());
        let names: Vec<&str> = result.profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Aggressive Alice"));
        assert!(names.contains(&"Conservative Carlos"));
        assert!(names.contains(&"Balanced Sam"));
    }

    #[test]
    fn list_profiles_surfaces_unparseable_file_as_error() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();
        // Write one valid and one invalid profile.
        fs::write(pdir.join("good.md"), SAMPLE).unwrap();
        fs::write(pdir.join("bad.md"), BAD).unwrap();

        let result = list_profiles(&pdir).unwrap();
        assert_eq!(result.profiles.len(), 1, "valid profiles");
        assert_eq!(result.profiles[0].name, "Sample Player");
        assert_eq!(result.errors.len(), 1, "parse errors");
        assert_eq!(result.errors[0].filename, "bad.md");
        assert!(!result.errors[0].error.is_empty());
    }

    #[test]
    fn load_profile_returns_correct_profile() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("sample.md"), SAMPLE).unwrap();

        let profile = load_profile(&pdir, "sample").unwrap();
        assert_eq!(profile.id, "sample");
        assert_eq!(profile.name, "Sample Player");
    }

    #[test]
    fn load_profile_unknown_id_returns_io_error() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();

        let result = load_profile(&pdir, "nonexistent");
        assert!(matches!(result, Err(ProfileError::Io(_))));
    }

    #[test]
    fn save_profile_writes_and_can_be_read_back() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());

        let saved = save_profile(&pdir, "sample", SAMPLE).unwrap();
        assert_eq!(saved.name, "Sample Player");

        let loaded = load_profile(&pdir, "sample").unwrap();
        assert_eq!(loaded.name, "Sample Player");
    }

    #[test]
    fn save_profile_rejects_invalid_content() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();

        let result = save_profile(&pdir, "bad", BAD);
        assert!(result.is_err());
        // File should not have been written.
        assert!(!pdir.join("bad.md").exists());
    }

    #[test]
    fn delete_profile_removes_file() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();
        fs::write(pdir.join("sample.md"), SAMPLE).unwrap();

        delete_profile(&pdir, "sample").unwrap();
        assert!(load_profile(&pdir, "sample").is_err());
    }
}
