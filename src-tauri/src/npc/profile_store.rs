use std::{
    fs,
    path::{Path, PathBuf},
};

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

/// List all profiles in `dir`, sorted alphabetically by name.
///
/// Files that fail to parse are skipped with a logged warning.
pub fn list_profiles(dir: &Path) -> Result<Vec<NpcProfile>, ProfileError> {
    ensure_profiles_dir(dir)?;

    let mut profiles = Vec::new();

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
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[profile_store] skipping {:?}: {e}", path);
                continue;
            }
        };
        match parse_profile(&stem, &content) {
            Ok(p) => profiles.push(p),
            Err(e) => {
                eprintln!("[profile_store] skipping {:?}: {e}", path);
            }
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
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
        let profiles = list_profiles(dir.path()).unwrap();
        // Starter profiles should have been seeded.
        assert_eq!(profiles.len(), 3);
        let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"Aggressive Alice"));
        assert!(names.contains(&"Conservative Carlos"));
        assert!(names.contains(&"Balanced Sam"));
    }

    #[test]
    fn list_profiles_skips_unparseable_file() {
        let dir = temp_dir();
        let pdir = profiles_dir(dir.path());
        fs::create_dir_all(&pdir).unwrap();
        // Write one valid and one invalid profile.
        fs::write(pdir.join("good.md"), SAMPLE).unwrap();
        fs::write(pdir.join("bad.md"), BAD).unwrap();

        let profiles = list_profiles(&pdir).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Sample Player");
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
