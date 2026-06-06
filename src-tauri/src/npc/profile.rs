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
    /// Free-form Markdown body (minus any named sections extracted below).
    pub description: String,
    /// Content of the optional `## Opponent tendencies` section.
    pub opponent_tendencies: Option<String>,
    /// Content of the optional `## Tilt behaviour` section.
    pub tilt_behaviour: Option<String>,
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
/// The body may contain optional H2 sections:
/// - `## Opponent tendencies`
/// - `## Tilt behaviour`
///
/// These are extracted into their own fields; all other body content stays in
/// `description`.
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

    let (description, opponent_tendencies, tilt_behaviour) = extract_sections(&body);

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
        description,
        opponent_tendencies,
        tilt_behaviour,
    })
}

/// Extract the `## Opponent tendencies` and `## Tilt behaviour` sections from `body`.
///
/// Returns `(remaining_body, opponent_tendencies, tilt_behaviour)`.
fn extract_sections(body: &str) -> (String, Option<String>, Option<String>) {
    let mut opponent_tendencies: Option<String> = None;
    let mut tilt_behaviour: Option<String> = None;

    // Split body at every `## ` heading boundary.
    // We walk through the body splitting on "\n##" to find named sections.
    let sections = split_h2_sections(body);

    let mut remaining_parts: Vec<String> = Vec::new();

    for (heading, section_body) in sections {
        match heading.as_deref() {
            None => {
                // Content before the first ## heading.
                remaining_parts.push(section_body.trim().to_string());
            }
            Some(h) if h.trim().eq_ignore_ascii_case("Opponent tendencies") => {
                let trimmed = section_body.trim().to_string();
                if !trimmed.is_empty() {
                    opponent_tendencies = Some(trimmed);
                }
            }
            Some(h) if h.trim().eq_ignore_ascii_case("Tilt behaviour") => {
                let trimmed = section_body.trim().to_string();
                if !trimmed.is_empty() {
                    tilt_behaviour = Some(trimmed);
                }
            }
            Some(h) => {
                // Any other ## section stays in the description.
                let part = format!("## {}\n{}", h, section_body);
                remaining_parts.push(part.trim_end().to_string());
            }
        }
    }

    let description = remaining_parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    (description, opponent_tendencies, tilt_behaviour)
}

/// Split a Markdown body into `(Option<heading_text>, body_text)` pairs.
///
/// The first pair has `None` for its heading (content before the first `##`).
fn split_h2_sections(body: &str) -> Vec<(Option<String>, String)> {
    let mut result: Vec<(Option<String>, String)> = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // Save the previous section.
            result.push((current_heading.take(), current_lines.join("\n")));
            current_heading = Some(rest.to_string());
            current_lines = Vec::new();
        } else {
            current_lines.push(line);
        }
    }
    // Save the final section.
    result.push((current_heading, current_lines.join("\n")));
    result
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
        assert!(profile.opponent_tendencies.is_none());
        assert!(profile.tilt_behaviour.is_none());
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

    // --- Phase 3 section extraction tests ---

    #[test]
    fn profile_without_sections_has_none_fields_and_unchanged_description() {
        let content = "---\nname: Plain\n---\nJust a plain body with no sections.";
        let profile = parse_profile("plain", content).unwrap();
        assert!(profile.opponent_tendencies.is_none());
        assert!(profile.tilt_behaviour.is_none());
        assert!(profile.description.contains("plain body"));
    }

    #[test]
    fn opponent_tendencies_section_captured_and_removed_from_description() {
        let content = "\
---
name: Alice
---
Main strategy body.

## Opponent tendencies
Bluff tight players on the river.

Isolate limpers.";
        let profile = parse_profile("alice", content).unwrap();
        let ot = profile.opponent_tendencies.unwrap();
        assert!(ot.contains("Bluff tight players"), "got: {ot}");
        assert!(ot.contains("Isolate limpers"), "got: {ot}");
        assert!(
            !profile.description.contains("Opponent tendencies"),
            "section heading leaked into description: {}",
            profile.description
        );
    }

    #[test]
    fn tilt_behaviour_section_captured_and_removed_from_description() {
        let content = "\
---
name: Alice
---
Base strategy.

## Tilt behaviour
Widen range after losses.";
        let profile = parse_profile("alice", content).unwrap();
        assert!(profile.tilt_behaviour.is_some());
        assert!(profile.tilt_behaviour.unwrap().contains("Widen range"));
        assert!(!profile.description.contains("Tilt behaviour"));
    }

    #[test]
    fn both_sections_can_coexist() {
        let content = "\
---
name: Sam
---
General strategy.

## Opponent tendencies
Observe carefully.

## Tilt behaviour
Stay calm.";
        let profile = parse_profile("sam", content).unwrap();
        assert!(profile.opponent_tendencies.is_some());
        assert!(profile.tilt_behaviour.is_some());
        assert!(profile.description.contains("General strategy"));
        assert!(!profile.description.contains("Observe carefully"));
        assert!(!profile.description.contains("Stay calm"));
    }

    #[test]
    fn section_extraction_is_whitespace_tolerant() {
        let content = "\
---
name: Alice
---

Main body.

## Opponent tendencies

  Bluff on missed draws.

";
        let profile = parse_profile("alice", content).unwrap();
        let ot = profile.opponent_tendencies.unwrap();
        assert!(ot.contains("Bluff on missed draws"), "got: {ot}");
    }

    #[test]
    fn unrelated_h2_headings_preserved_in_description() {
        let content = "\
---
name: Sam
---
Intro.

## Pre-flop notes
Open wide in position.

## Opponent tendencies
Bluff tight players.";
        let profile = parse_profile("sam", content).unwrap();
        assert!(
            profile.description.contains("Pre-flop notes"),
            "got: {}",
            profile.description
        );
        assert!(
            profile.description.contains("Open wide in position"),
            "got: {}",
            profile.description
        );
        assert!(profile.opponent_tendencies.is_some());
    }
}
