use serde::{Deserialize, Serialize};

/// A player profile driving LLM-based NPC decisions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcProfile {
    /// Filename stem, e.g. "aggressive-alice".
    pub id: String,
    /// Human-readable name from the frontmatter `name:` field.
    pub name: String,
    /// Playing style tag, e.g. "loose-aggressive".  Defaults to "custom" if absent.
    pub style: String,
    /// Skill level tag, e.g. "intermediate".  Defaults to "unknown" if absent.
    pub skill: String,
    /// Free-form Markdown body used as the LLM system prompt context.
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct NpcProfileFrontmatter {
    name: Option<String>,
    style: Option<String>,
    skill: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("missing frontmatter delimiter")]
    MissingDelimiter,
    #[error("invalid YAML frontmatter: {0}")]
    YamlParse(String),
    #[error("profile name is required")]
    MissingName,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a Markdown profile file with YAML frontmatter into an `NpcProfile`.
///
/// Format expected:
/// ```text
/// ---
/// name: Aggressive Alice
/// style: loose-aggressive
/// skill: intermediate
/// ---
/// Free-form body text.
/// ```
pub fn parse_profile(file_stem: &str, content: &str) -> Result<NpcProfile, ProfileError> {
    let rest = content
        .strip_prefix("---")
        .ok_or(ProfileError::MissingDelimiter)?;

    let end = rest.find("\n---").ok_or(ProfileError::MissingDelimiter)?;

    let yaml_block = &rest[..end];
    let after_delim = &rest[end + 4..]; // skip the `\n---` bytes
    let body = after_delim.trim().to_string();

    let frontmatter: NpcProfileFrontmatter =
        serde_yaml::from_str(yaml_block).map_err(|e| ProfileError::YamlParse(e.to_string()))?;

    let name = frontmatter
        .name
        .filter(|s| !s.trim().is_empty())
        .ok_or(ProfileError::MissingName)?;

    Ok(NpcProfile {
        id: file_stem.to_string(),
        name,
        style: frontmatter
            .style
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "custom".to_string()),
        skill: frontmatter
            .skill
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "unknown".to_string()),
        description: body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_PROFILE: &str = "\
---
name: Test Player
style: loose-aggressive
skill: intermediate
---
This player bets aggressively on every street.
Has a tendency to bluff on missed draws.";

    #[test]
    fn valid_profile_parses_correctly() {
        let profile = parse_profile("test-player", VALID_PROFILE).unwrap();
        assert_eq!(profile.id, "test-player");
        assert_eq!(profile.name, "Test Player");
        assert_eq!(profile.style, "loose-aggressive");
        assert_eq!(profile.skill, "intermediate");
        assert!(profile.description.contains("bets aggressively"));
    }

    #[test]
    fn missing_second_delimiter_returns_error() {
        let content = "---\nname: Test Player\nstyle: aggressive\nno closing delimiter";
        let result = parse_profile("bad", content);
        assert!(matches!(result, Err(ProfileError::MissingDelimiter)));
    }

    #[test]
    fn missing_first_delimiter_returns_error() {
        let content = "name: Test Player\n---\nbody";
        let result = parse_profile("bad", content);
        assert!(matches!(result, Err(ProfileError::MissingDelimiter)));
    }

    #[test]
    fn invalid_yaml_returns_yaml_parse_error() {
        let content = "---\nname: [unclosed bracket\n---\nbody";
        let result = parse_profile("bad", content);
        assert!(matches!(result, Err(ProfileError::YamlParse(_))));
    }

    #[test]
    fn missing_name_field_returns_error() {
        let content = "---\nstyle: aggressive\nskill: beginner\n---\nbody";
        let result = parse_profile("no-name", content);
        assert!(matches!(result, Err(ProfileError::MissingName)));
    }

    #[test]
    fn empty_name_field_returns_error() {
        let content = "---\nname: \"\"\nstyle: aggressive\n---\nbody";
        let result = parse_profile("empty-name", content);
        assert!(matches!(result, Err(ProfileError::MissingName)));
    }

    #[test]
    fn absent_style_and_skill_use_defaults() {
        let content = "---\nname: Minimal Player\n---\nbody";
        let profile = parse_profile("minimal", content).unwrap();
        assert_eq!(profile.style, "custom");
        assert_eq!(profile.skill, "unknown");
    }

    #[test]
    fn multi_paragraph_body_is_preserved() {
        let content =
            "---\nname: Verbose Player\n---\nParagraph one.\n\nParagraph two.\n\nParagraph three.";
        let profile = parse_profile("verbose", content).unwrap();
        assert!(profile.description.contains("Paragraph one."));
        assert!(profile.description.contains("Paragraph two."));
        assert!(profile.description.contains("Paragraph three."));
    }

    #[test]
    fn body_is_trimmed_of_leading_and_trailing_whitespace() {
        let content = "---\nname: Padded\n---\n\n\n  Body text here.  \n\n";
        let profile = parse_profile("padded", content).unwrap();
        assert_eq!(profile.description, "Body text here.");
    }
}
