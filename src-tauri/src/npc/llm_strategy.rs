use crate::domain::ActionType;

use super::llm_action::{parse_llm_response, validate_llm_action};
use super::llm_client::{LlmClient, LlmError};
use super::profile::NpcProfile;
use super::prompt::{build_system_prompt, build_user_message, GameStateSnapshot};
use super::runner::first_check_or_call;

/// Structured reason for falling back from LLM to rule-based decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmFallbackReason {
    ProviderNotConfigured,
    ApiKeyMissing,
    RequestFailed,
    ResponseParseFailed,
    InvalidAction,
    Timeout,
}

impl std::fmt::Display for LlmFallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderNotConfigured => write!(f, "provider not configured"),
            Self::ApiKeyMissing => write!(f, "API key missing"),
            Self::RequestFailed => write!(f, "request failed"),
            Self::ResponseParseFailed => write!(f, "response parse failed"),
            Self::InvalidAction => write!(f, "invalid action in response"),
            Self::Timeout => write!(f, "request timed out"),
        }
    }
}

/// Choose an action using the LLM.
///
/// Returns the chosen action and, if the LLM path failed and fell back to
/// rule-based, the reason for the fallback.
pub fn choose_llm_action(
    client: &LlmClient,
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> (ActionType, Option<u32>, Option<LlmFallbackReason>) {
    let system = build_system_prompt();
    let user = build_user_message(profile, snapshot);

    match client.complete(&system, &user) {
        Ok(text) => match parse_llm_response(&text) {
            Ok(response) => {
                let (at, amt) = validate_llm_action(&response, snapshot);
                // Detect whether the LLM response contained an action that wasn't in
                // legal_actions. validate_llm_action already corrected it; we surface
                // InvalidAction so the caller can log or display the fallback.
                let action_was_legal = {
                    let normalized = response.action.to_lowercase().replace(['_', '-'], "");
                    snapshot.legal_actions.iter().any(|la| {
                        let la_str = format!("{la:?}").to_lowercase();
                        normalized == la_str
                            || (normalized == "bet" && la_str == "raise")
                            || (normalized == "raise" && la_str == "bet")
                    })
                };
                let fallback = if action_was_legal {
                    None
                } else {
                    Some(LlmFallbackReason::InvalidAction)
                };
                (at, amt, fallback)
            }
            Err(e) => {
                eprintln!("[llm_strategy] parse error: {e}; falling back to rule-based");
                let (at, amt) = rule_based_fallback(profile, snapshot);
                (at, amt, Some(LlmFallbackReason::ResponseParseFailed))
            }
        },
        Err(e) => {
            let reason = match &e {
                LlmError::Timeout => LlmFallbackReason::Timeout,
                _ => LlmFallbackReason::RequestFailed,
            };
            eprintln!("[llm_strategy] LLM error: {e}; falling back to rule-based");
            let (at, amt) = rule_based_fallback(profile, snapshot);
            (at, amt, Some(reason))
        }
    }
}

/// Style-aware rule-based fallback respecting the profile's configured style.
///
/// - aggressive/loose: prefer raising when sensible
/// - balanced: check or call on modest bets, fold on large bets
/// - tight/passive/conservative (default): fold on any meaningful bet, check otherwise
fn rule_based_fallback(
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> (ActionType, Option<u32>) {
    let style = profile.style.to_lowercase();
    let legal = &snapshot.legal_actions;

    if style.contains("aggressive") || style.contains("loose") {
        // Aggressive: raise when the bet is not too large relative to stack.
        if (legal.contains(&ActionType::Raise) || legal.contains(&ActionType::Bet))
            && snapshot.stack > 0
            && snapshot.call_amount <= snapshot.stack / 3
        {
            let raise_to = snapshot.min_raise_to.unwrap_or(snapshot.call_amount * 2);
            let at = if legal.contains(&ActionType::Raise) {
                ActionType::Raise
            } else {
                ActionType::Bet
            };
            return (at, Some(raise_to));
        }
        first_check_or_call(legal)
    } else if style.contains("balanced") {
        // Balanced: call on modest bets (≤ 1/3 pot), fold on large bets.
        if legal.contains(&ActionType::Check) {
            (ActionType::Check, None)
        } else if snapshot.call_amount <= snapshot.pot_total / 3
            && legal.contains(&ActionType::Call)
        {
            (ActionType::Call, None)
        } else if legal.contains(&ActionType::Fold) {
            (ActionType::Fold, None)
        } else {
            first_check_or_call(legal)
        }
    } else {
        // Tight/passive: fold on any meaningful bet (> 5% of stack), check otherwise.
        if legal.contains(&ActionType::Check) {
            (ActionType::Check, None)
        } else if snapshot.stack > 0
            && snapshot.call_amount <= snapshot.stack / 20
            && legal.contains(&ActionType::Call)
        {
            (ActionType::Call, None)
        } else if legal.contains(&ActionType::Fold) {
            (ActionType::Fold, None)
        } else {
            first_check_or_call(legal)
        }
    }
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
            session_context: None,
            opponent_context: None,
            tilt_description: None,
        }
    }

    fn make_profile(style: &str) -> NpcProfile {
        NpcProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            style: style.to_string(),
            skill: "intermediate".to_string(),
            description: "You must always raise preflop with premium hands.".to_string(),
            opponent_tendencies: None,
            tilt_behaviour: None,
        }
    }

    fn anthropic_cfg(key: &str) -> crate::npc::provider::LlmProviderConfig {
        crate::npc::provider::LlmProviderConfig {
            provider: crate::npc::provider::LlmProviderType::Anthropic,
            api_key: Some(key.to_string()),
            endpoint_url: None,
            model: None,
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
        let url = server.base_url();
        std::mem::forget(server);
        LlmClient::new(anthropic_cfg("test-key")).with_endpoint_override(url)
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
        let url = server.base_url();
        std::mem::forget(server);
        LlmClient::with_timeout_secs(anthropic_cfg("test-key"), 1, url)
    }

    #[test]
    fn valid_raise_from_llm_is_submitted_within_legal_bounds() {
        let client = valid_raise_client();
        let profile = make_profile("loose-aggressive");
        let snap = snap_with_raise();

        let (at, amt, _fallback) = choose_llm_action(&client, &profile, &snap);
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

        let (at, _, fallback) = choose_llm_action(&client, &profile, &snap);
        assert!(snap.legal_actions.contains(&at));
        assert!(fallback.is_some(), "timeout must set a fallback reason");
    }

    #[test]
    fn api_error_sets_request_failed_fallback_reason() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(401).body(r#"{"error":"invalid key"}"#);
        });
        let url = server.base_url();
        std::mem::forget(server);
        let client = LlmClient::new(anthropic_cfg("bad-key")).with_endpoint_override(url);

        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let (at, _, fallback) = choose_llm_action(&client, &profile, &snap);
        assert!(snap.legal_actions.contains(&at));
        assert_eq!(fallback, Some(LlmFallbackReason::RequestFailed));
    }

    #[test]
    fn malformed_response_sets_parse_failed_fallback_reason() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "content": [{"type": "text", "text": "not json at all"}],
                    "model": "claude-haiku-4-5-20251001",
                    "stop_reason": "end_turn"
                }));
        });
        let url = server.base_url();
        std::mem::forget(server);
        let client = LlmClient::new(anthropic_cfg("test-key")).with_endpoint_override(url);

        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let (at, _, fallback) = choose_llm_action(&client, &profile, &snap);
        assert!(snap.legal_actions.contains(&at));
        assert_eq!(fallback, Some(LlmFallbackReason::ResponseParseFailed));
    }

    #[test]
    fn aggressive_style_prefers_raise_when_call_amount_is_small() {
        let profile = make_profile("aggressive");
        let snap = snap_with_raise(); // call_amount=50, stack=950 → 50 <= 950/3 → raise

        let (at, raise_to) = super::rule_based_fallback(&profile, &snap);
        assert_eq!(at, ActionType::Raise);
        assert!(
            raise_to.is_some(),
            "aggressive should include a raise amount"
        );
    }

    #[test]
    fn conservative_style_folds_on_large_bet() {
        let profile = make_profile("conservative");
        let mut snap = snap_with_raise();
        // call_amount (200) > stack/20 (950/20 = 47) → fold
        snap.call_amount = 200;
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let (at, _) = super::rule_based_fallback(&profile, &snap);
        assert_eq!(at, ActionType::Fold);
    }

    #[test]
    fn conservative_style_calls_on_tiny_bet() {
        let profile = make_profile("conservative");
        let mut snap = snap_with_raise();
        // call_amount (10) <= stack/20 (950/20 = 47) → call
        snap.call_amount = 10;
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let (at, _) = super::rule_based_fallback(&profile, &snap);
        assert_eq!(at, ActionType::Call);
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
        let url = server.base_url();
        std::mem::forget(server);
        let client = LlmClient::new(anthropic_cfg("test-key")).with_endpoint_override(url);

        let profile = make_profile("balanced");
        let mut snap = snap_with_raise();
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let (at, _, _fallback) = choose_llm_action(&client, &profile, &snap);
        // "check" not legal → validate_llm_action → first_check_or_call → Call
        assert!(matches!(at, ActionType::Call | ActionType::Fold));
    }

    #[test]
    fn illegal_action_from_llm_sets_invalid_action_fallback_reason() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(200)
                .header("content-type", "application/json")
                // "check" is not in legal_actions (fold/call/raise only)
                .json_body(serde_json::json!({
                    "content": [{"type": "text", "text": "{\"action\":\"check\",\"amount\":null}"}],
                    "model": "claude-haiku-4-5-20251001",
                    "stop_reason": "end_turn"
                }));
        });
        let url = server.base_url();
        std::mem::forget(server);
        let client = LlmClient::new(anthropic_cfg("test-key")).with_endpoint_override(url);

        let profile = make_profile("balanced");
        let mut snap = snap_with_raise();
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let (_, _, fallback_reason) = choose_llm_action(&client, &profile, &snap);
        assert_eq!(
            fallback_reason,
            Some(LlmFallbackReason::InvalidAction),
            "LLM returning an illegal action should set InvalidAction fallback reason"
        );
    }
}

#[cfg(test)]
mod ollama_live_tests {
    use super::*;
    use crate::domain::{ActionType, BlindLevel, StreetPhase};
    use crate::npc::llm_client::LlmClient;
    use crate::npc::profile::NpcProfile;
    use crate::npc::prompt::GameStateSnapshot;
    use crate::npc::provider::{LlmProviderConfig, LlmProviderType};
    use crate::npc::strategy::Position;

    fn ollama_cfg(model: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            provider: LlmProviderType::Ollama,
            api_key: None,
            endpoint_url: None,
            model: Some(model.to_string()),
        }
    }

    fn balanced_profile() -> NpcProfile {
        NpcProfile {
            id: "balanced-sam".to_string(),
            name: "Balanced Sam".to_string(),
            style: "balanced".to_string(),
            skill: "advanced".to_string(),
            description: "You are Balanced Sam, a balanced Texas Hold'em player. Always respond with valid JSON only.".to_string(),
            opponent_tendencies: None,
            tilt_behaviour: None,
        }
    }

    fn preflop_snap() -> GameStateSnapshot {
        GameStateSnapshot {
            hand_number: 1,
            street: StreetPhase::Preflop,
            board_cards: vec![],
            hole_cards: vec![
                crate::domain::Card {
                    rank: crate::domain::Rank::Ace,
                    suit: crate::domain::Suit::Spades,
                },
                crate::domain::Card {
                    rank: crate::domain::Rank::King,
                    suit: crate::domain::Suit::Hearts,
                },
            ],
            pot_total: 30,
            call_amount: 20,
            min_raise_to: Some(40),
            max_raise_to: Some(1000),
            stack: 1000,
            position: Position::Late,
            active_player_count: 3,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            blind_level: BlindLevel {
                level_index: 0,
                label: "L1".into(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 300,
            },
            street_history: vec![],
            session_context: None,
            opponent_context: None,
            tilt_description: None,
        }
    }

    // Serializes the live Ollama integration tests so they never run concurrently
    // against a single (CPU-only) Ollama server. Two simultaneous requests force
    // Ollama to load/run both at once and both miss the timeout; run one at a time
    // each completes in seconds.
    static LIVE_OLLAMA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn postflop_snap() -> GameStateSnapshot {
        GameStateSnapshot {
            hand_number: 1,
            street: StreetPhase::Flop,
            board_cards: vec![
                crate::domain::Card {
                    rank: crate::domain::Rank::Queen,
                    suit: crate::domain::Suit::Hearts,
                },
                crate::domain::Card {
                    rank: crate::domain::Rank::Jack,
                    suit: crate::domain::Suit::Spades,
                },
                crate::domain::Card {
                    rank: crate::domain::Rank::Two,
                    suit: crate::domain::Suit::Clubs,
                },
            ],
            hole_cards: vec![
                crate::domain::Card {
                    rank: crate::domain::Rank::Ace,
                    suit: crate::domain::Suit::Spades,
                },
                crate::domain::Card {
                    rank: crate::domain::Rank::King,
                    suit: crate::domain::Suit::Hearts,
                },
            ],
            pot_total: 120,
            call_amount: 40,
            min_raise_to: Some(80),
            max_raise_to: Some(1000),
            stack: 980,
            position: Position::Late,
            active_player_count: 2,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            blind_level: BlindLevel {
                level_index: 0,
                label: "L1".into(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 300,
            },
            street_history: vec![],
            session_context: None,
            opponent_context: None,
            tilt_description: None,
        }
    }

    fn assert_legal_live_action(
        snap: &GameStateSnapshot,
        action_type: ActionType,
        raise_to: Option<u32>,
    ) {
        assert!(
            snap.legal_actions.contains(&action_type),
            "ollama returned illegal action: {action_type:?}"
        );
        if matches!(action_type, ActionType::Raise | ActionType::Bet) {
            let amt = raise_to.unwrap_or(0);
            let lo = snap.min_raise_to.unwrap_or(0);
            let hi = snap.max_raise_to.unwrap_or(u32::MAX);
            assert!(
                (lo..=hi).contains(&amt),
                "raise amount {amt} out of bounds [{lo}, {hi}]"
            );
        }
    }

    /// Live integration test against a running Ollama server using the default
    /// recommended model. Skipped automatically when Ollama is unreachable.
    /// Serialized via LIVE_OLLAMA_LOCK so `-- --ignored` runs them one at a time.
    #[test]
    #[ignore]
    fn ollama_llama32_preflop_returns_legal_poker_action() {
        let _guard = LIVE_OLLAMA_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let client = LlmClient::new(ollama_cfg("llama3.2:3b"));
        let profile = balanced_profile();
        let snap = preflop_snap();

        let (action_type, raise_to, _fallback) = choose_llm_action(&client, &profile, &snap);
        assert_legal_live_action(&snap, action_type, raise_to);
        eprintln!("llama3.2 preflop chose: {action_type:?} {raise_to:?}");
    }

    #[test]
    #[ignore]
    fn ollama_llama32_postflop_returns_legal_poker_action() {
        let _guard = LIVE_OLLAMA_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let client = LlmClient::new(ollama_cfg("llama3.2:3b"));
        let profile = balanced_profile();
        let snap = postflop_snap();

        let (action_type, raise_to, _fallback) = choose_llm_action(&client, &profile, &snap);
        assert_legal_live_action(&snap, action_type, raise_to);
        eprintln!("llama3.2 postflop chose: {action_type:?} {raise_to:?}");
    }
}
