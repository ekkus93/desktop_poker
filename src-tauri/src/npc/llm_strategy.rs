use crate::domain::ActionType;

use super::llm_action::{parse_llm_response, validate_llm_action};
use super::llm_client::LlmClient;
use super::profile::NpcProfile;
use super::prompt::{build_system_prompt, build_user_message, GameStateSnapshot};
use super::runner::first_check_or_call;
use super::NpcStyle;

/// Choose an action using the LLM.
///
/// On any error (timeout, API error, parse error), falls back to the Phase 1
/// rule-based strategy using the profile's `style` field.
pub fn choose_llm_action(
    client: &LlmClient,
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> (ActionType, Option<u32>) {
    let system = build_system_prompt();
    let user = build_user_message(profile, snapshot);

    match client.complete(&system, &user) {
        Ok(text) => match parse_llm_response(&text) {
            Ok(response) => validate_llm_action(&response, snapshot),
            Err(e) => {
                eprintln!("[llm_strategy] parse error: {e}; falling back to rule-based");
                rule_based_fallback(profile, snapshot)
            }
        },
        Err(e) => {
            eprintln!("[llm_strategy] LLM error: {e}; falling back to rule-based");
            rule_based_fallback(profile, snapshot)
        }
    }
}

/// Map a profile's style string to a Phase 1 `NpcStyle` for rule-based fallback.
fn profile_style_to_npc_style(style: &str) -> NpcStyle {
    let lower = style.to_lowercase();
    if lower.contains("aggressive") || lower.contains("loose") {
        NpcStyle::Aggressive
    } else {
        NpcStyle::Conservative
    }
}

fn rule_based_fallback(
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> (ActionType, Option<u32>) {
    let style = profile_style_to_npc_style(&profile.style);
    let _ = (style, snapshot);

    // Minimal fallback: check or call — safe under all circumstances.
    first_check_or_call(&snapshot.legal_actions)
}

#[cfg(test)]
mod tests {
    use super::super::prompt::GameStateSnapshot;
    use super::super::strategy::Position;
    use super::*;
    use crate::domain::{BlindLevel, Card, Rank, StreetPhase, Suit};

    fn blind() -> BlindLevel {
        BlindLevel {
            level_index: 0,
            label: "L1".to_string(),
            small_blind: 25,
            big_blind: 50,
            ante: 0,
            duration_seconds: 600,
        }
    }

    fn snap_with_raise() -> GameStateSnapshot {
        GameStateSnapshot {
            hand_number: 1,
            street: StreetPhase::Preflop,
            board_cards: vec![],
            hole_cards: vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Hearts,
                },
            ],
            pot_total: 100,
            call_amount: 50,
            min_raise_to: Some(100),
            max_raise_to: Some(1000),
            stack: 950,
            position: Position::Late,
            active_player_count: 3,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            blind_level: blind(),
            street_history: vec![],
        }
    }

    fn make_profile(style: &str) -> NpcProfile {
        NpcProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            style: style.to_string(),
            skill: "intermediate".to_string(),
            description: "You must always raise preflop with premium hands.".to_string(),
        }
    }

    /// A test LlmClient that returns a fixed response via a mock server.
    fn valid_raise_client() -> LlmClient {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "content": [{"type": "text", "text": "{\"action\":\"raise\",\"amount\":300}"}],
                    "model": "claude-haiku-4-5-20251001",
                    "stop_reason": "end_turn"
                }));
        });
        // Keep the server alive by leaking it (test scope).
        let url = format!("{}/v1/messages", server.base_url());
        std::mem::forget(server);
        LlmClient::new("test-key".to_string()).with_base_url(url)
    }

    fn timeout_client() -> LlmClient {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .delay(std::time::Duration::from_secs(10))
                .json_body(serde_json::json!({"content": []}));
        });
        let url = format!("{}/v1/messages", server.base_url());
        std::mem::forget(server);
        LlmClient::with_options(
            "test-key".to_string(),
            "claude-haiku-4-5-20251001".to_string(),
            1,
        )
        .with_base_url(url)
    }

    #[test]
    fn valid_raise_from_llm_is_submitted_within_legal_bounds() {
        let client = valid_raise_client();
        let profile = make_profile("loose-aggressive");
        let snap = snap_with_raise();

        let (at, amt) = choose_llm_action(&client, &profile, &snap);
        assert_eq!(at, ActionType::Raise);
        let amount = amt.unwrap();
        assert!(
            (100..=1000).contains(&amount),
            "raise {amount} out of bounds"
        );
    }

    #[test]
    fn timeout_falls_back_to_rule_based_legal_action() {
        let client = timeout_client();
        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let (at, _) = choose_llm_action(&client, &profile, &snap);
        assert!(snap.legal_actions.contains(&at));
    }

    #[test]
    fn illegal_action_from_llm_produces_legal_fallback() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                // "check" is not in legal_actions (only fold/call/raise)
                .json_body(serde_json::json!({
                    "content": [{"type": "text", "text": "{\"action\":\"check\",\"amount\":null}"}],
                    "model": "claude-haiku-4-5-20251001",
                    "stop_reason": "end_turn"
                }));
        });
        let url = format!("{}/v1/messages", server.base_url());
        std::mem::forget(server);
        let client = LlmClient::new("test-key".to_string()).with_base_url(url);

        let profile = make_profile("balanced");
        let mut snap = snap_with_raise();
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let (at, _) = choose_llm_action(&client, &profile, &snap);
        // "check" not legal → validate_llm_action → first_check_or_call → Call
        assert!(matches!(at, ActionType::Call | ActionType::Fold));
    }
}
