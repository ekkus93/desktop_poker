use crate::domain::ActionType;
use crate::npc::NpcStyle;

use super::llm_action::{parse_llm_response, validate_llm_action};
use super::llm_client::{LlmClient, LlmError};
use super::profile::NpcProfile;
use super::prompt::{
    build_system_prompt, build_user_message, render_game_state, GameStateSnapshot,
};
#[cfg(test)]
use super::runner::first_check_or_call;

/// Convert a freeform profile style string to an `NpcStyle` for rule-based fallback.
///
/// The profile style is user/persona data, so parsing is keyword-based and
/// case-insensitive. Unknown strings fall back to `NpcStyle::Conservative`.
fn profile_style_to_npc_style(style: &str) -> NpcStyle {
    let norm = style.trim().to_ascii_lowercase().replace('_', "-");
    match norm.as_str() {
        "aggressive" | "loose-aggressive" | "lag" | "maniac" | "bully" | "pressure" | "bluffer"
        | "bluff-heavy" | "loose" => NpcStyle::Aggressive,
        _ => NpcStyle::Conservative,
    }
}

/// Resolve the rule-based fallback style for an NPC.
///
/// - Profile present → derive from `NpcProfile.style` so profiled fallback is consistent.
/// - Profile absent → use `NpcConfig.style`.
pub fn resolve_fallback_style(profile: Option<&NpcProfile>, config_style: NpcStyle) -> NpcStyle {
    match profile {
        Some(p) => profile_style_to_npc_style(&p.style),
        None => config_style,
    }
}

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
/// Returns `Ok((action, amount))` when the LLM produced a valid legal action.
/// Returns `Err(reason)` on any LLM failure (timeout, parse error, invalid action,
/// etc.) so the caller can decide whether to apply rule-based fallback.
/// Rule-based fallback is NOT applied here.
pub fn choose_llm_action(
    client: &LlmClient,
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> Result<(ActionType, Option<u32>), LlmFallbackReason> {
    let system = build_system_prompt();
    let user = build_user_message(profile, snapshot);

    match client.complete(&system, &user) {
        Ok(text) => match parse_llm_response(&text) {
            Ok(response) => {
                let (at, amt) = validate_llm_action(&response, snapshot);
                // Check whether the LLM's action was already in legal_actions.
                let action_was_legal = {
                    let normalized = response.action.to_lowercase().replace(['_', '-'], "");
                    snapshot.legal_actions.iter().any(|la| {
                        let la_str = format!("{la:?}").to_lowercase();
                        normalized == la_str
                            || (normalized == "bet" && la_str == "raise")
                            || (normalized == "raise" && la_str == "bet")
                    })
                };
                if action_was_legal {
                    Ok((at, amt))
                } else {
                    eprintln!(
                        "[llm_strategy] LLM returned illegal action {:?}",
                        response.action
                    );
                    Err(LlmFallbackReason::InvalidAction)
                }
            }
            Err(e) => {
                eprintln!("[llm_strategy] parse error: {e}");
                Err(LlmFallbackReason::ResponseParseFailed)
            }
        },
        Err(e) => {
            let reason = match &e {
                LlmError::Timeout => LlmFallbackReason::Timeout,
                LlmError::ApiKeyMissing(_) => LlmFallbackReason::ApiKeyMissing,
                _ => LlmFallbackReason::RequestFailed,
            };
            eprintln!("[llm_strategy] LLM error: {e}");
            Err(reason)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedActionCandidate {
    pub action_type: ActionType,
    pub amount: Option<u32>,
    pub label: String,
}

pub fn build_embedded_action_candidates(
    snapshot: &GameStateSnapshot,
) -> Vec<EmbeddedActionCandidate> {
    let mut candidates = Vec::new();

    for action in &snapshot.legal_actions {
        match action {
            ActionType::Fold => candidates.push(EmbeddedActionCandidate {
                action_type: ActionType::Fold,
                amount: None,
                label: "Fold".to_string(),
            }),
            ActionType::Check => candidates.push(EmbeddedActionCandidate {
                action_type: ActionType::Check,
                amount: None,
                label: "Check".to_string(),
            }),
            ActionType::Call => candidates.push(EmbeddedActionCandidate {
                action_type: ActionType::Call,
                amount: None,
                label: format!("Call {}", snapshot.call_amount),
            }),
            ActionType::Bet | ActionType::Raise => {
                if let (Some(minimum), Some(maximum)) =
                    (snapshot.min_raise_to, snapshot.max_raise_to)
                {
                    let mut amounts = vec![minimum];
                    if maximum > minimum {
                        let midpoint = minimum + (maximum - minimum) / 2;
                        if midpoint > minimum && midpoint < maximum {
                            amounts.push(midpoint);
                        }
                        if maximum != snapshot.stack {
                            amounts.push(maximum);
                        }
                    }
                    amounts.sort_unstable();
                    amounts.dedup();
                    for amount in amounts {
                        candidates.push(EmbeddedActionCandidate {
                            action_type: *action,
                            amount: Some(amount),
                            label: format!(
                                "{} to {amount}",
                                if *action == ActionType::Bet {
                                    "Bet"
                                } else {
                                    "Raise"
                                }
                            ),
                        });
                    }
                }
            }
            ActionType::AllIn => candidates.push(EmbeddedActionCandidate {
                action_type: ActionType::AllIn,
                amount: None,
                label: format!("All-in for {}", snapshot.stack),
            }),
        }
    }

    candidates
}

pub fn choose_embedded_action(
    client: &LlmClient,
    profile: &NpcProfile,
    snapshot: &GameStateSnapshot,
) -> Result<(ActionType, Option<u32>), LlmFallbackReason> {
    let candidates = build_embedded_action_candidates(snapshot);
    if candidates.is_empty() || candidates.len() > 10 {
        return Err(LlmFallbackReason::InvalidAction);
    }

    let options = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| format!("{index}: {}", candidate.label))
        .collect::<Vec<_>>()
        .join("\n");
    let profile_description = profile.description.chars().take(600).collect::<String>();
    let system = "You are a tiny local poker NPC decision selector. Rust has already calculated every legal action and raise amount. Choose exactly one listed option that best matches the NPC profile and game state. Return one digit only. Do not explain your answer.";
    let user = format!(
        "NPC: {}\nStyle: {}\nSkill: {}\nProfile: {}\n\n{}\nCandidate actions:\n{}\n\nReturn exactly one digit from 0 through {}. /no_think",
        profile.name,
        profile.style,
        profile.skill,
        profile_description,
        render_game_state(snapshot),
        options,
        candidates.len() - 1
    );

    let index = client
        .choose_embedded_index(system, &user, candidates.len())
        .map_err(|error| match error {
            LlmError::Timeout => LlmFallbackReason::Timeout,
            LlmError::ApiKeyMissing(_) => LlmFallbackReason::ApiKeyMissing,
            LlmError::Parse(_) => LlmFallbackReason::ResponseParseFailed,
            LlmError::Api(_, _) | LlmError::Network(_) | LlmError::Embedded(_) => {
                LlmFallbackReason::RequestFailed
            }
        })?;
    let candidate = candidates
        .get(index)
        .ok_or(LlmFallbackReason::InvalidAction)?;
    Ok((candidate.action_type, candidate.amount))
}

/// Style-aware rule-based fallback respecting the profile's configured style.
///
/// - aggressive/loose: prefer raising when sensible
/// - balanced: check or call on modest bets, fold on large bets
/// - tight/passive/conservative (default): fold on any meaningful bet, check otherwise
#[cfg(test)]
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
            settings: crate::npc::provider::LlmProviderSettings {
                provider: crate::npc::provider::LlmProviderType::Anthropic,
                endpoint_url: None,
                model: None,
            },
            api_key: Some(key.to_string()),
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
        LlmClient::new(anthropic_cfg("test-key"))
            .expect("test client builds")
            .with_endpoint_override(url)
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
        LlmClient::with_timeout_secs(anthropic_cfg("test-key"), 1, url).expect("test client builds")
    }

    #[test]
    fn valid_raise_from_llm_is_submitted_within_legal_bounds() {
        let client = valid_raise_client();
        let profile = make_profile("loose-aggressive");
        let snap = snap_with_raise();

        let (at, amt) =
            choose_llm_action(&client, &profile, &snap).expect("LLM returned a valid action");
        assert_eq!(at, ActionType::Raise);
        let amount = amt.unwrap();
        assert!(
            (100..=1000).contains(&amount),
            "raise {amount} out of bounds"
        );
    }

    #[test]
    fn timeout_returns_err_timeout_reason() {
        let client = timeout_client();
        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let result = choose_llm_action(&client, &profile, &snap);
        assert_eq!(
            result,
            Err(crate::npc::LlmFallbackReason::Timeout),
            "timeout must return Err(Timeout), not {result:?}"
        );
    }

    #[test]
    fn api_error_returns_err_request_failed_reason() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/messages");
            then.status(401).body(r#"{"error":"invalid key"}"#);
        });
        let url = server.base_url();
        std::mem::forget(server);
        let client = LlmClient::new(anthropic_cfg("bad-key"))
            .expect("test client builds")
            .with_endpoint_override(url);

        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let result = choose_llm_action(&client, &profile, &snap);
        assert_eq!(result, Err(LlmFallbackReason::RequestFailed));
    }

    #[test]
    fn malformed_response_returns_err_parse_failed_reason() {
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
        let client = LlmClient::new(anthropic_cfg("test-key"))
            .expect("test client builds")
            .with_endpoint_override(url);

        let profile = make_profile("conservative");
        let snap = snap_with_raise();

        let result = choose_llm_action(&client, &profile, &snap);
        assert_eq!(result, Err(LlmFallbackReason::ResponseParseFailed));
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
    fn illegal_action_from_llm_returns_err_invalid_action() {
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
        let client = LlmClient::new(anthropic_cfg("test-key"))
            .expect("test client builds")
            .with_endpoint_override(url);

        let profile = make_profile("balanced");
        let mut snap = snap_with_raise();
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];

        let result = choose_llm_action(&client, &profile, &snap);
        assert_eq!(
            result,
            Err(LlmFallbackReason::InvalidAction),
            "LLM returning an illegal action must return Err(InvalidAction)"
        );
    }

    fn style_profile(style: &str) -> crate::npc::profile::NpcProfile {
        crate::npc::profile::NpcProfile {
            id: "test-profile".to_string(),
            name: "Test".to_string(),
            style: style.to_string(),
            skill: "beginner".to_string(),
            description: String::new(),
            opponent_tendencies: None,
            tilt_behaviour: None,
        }
    }

    #[test]
    fn unprofiled_fallback_uses_config_style() {
        assert_eq!(
            resolve_fallback_style(None, crate::npc::NpcStyle::Aggressive),
            crate::npc::NpcStyle::Aggressive
        );
        assert_eq!(
            resolve_fallback_style(None, crate::npc::NpcStyle::Conservative),
            crate::npc::NpcStyle::Conservative
        );
    }

    #[test]
    fn profiled_aggressive_strings_map_to_aggressive() {
        for s in &[
            "aggressive",
            "loose-aggressive",
            "lag",
            "maniac",
            "bully",
            "pressure",
            "bluffer",
            "bluff-heavy",
            "loose",
            "  Aggressive ", // whitespace + case
            "LOOSE-AGGRESSIVE",
        ] {
            let p = style_profile(s);
            assert_eq!(
                resolve_fallback_style(Some(&p), crate::npc::NpcStyle::Conservative),
                crate::npc::NpcStyle::Aggressive,
                "expected Aggressive for style {:?}",
                s
            );
        }
    }

    #[test]
    fn profiled_conservative_strings_map_to_conservative() {
        for s in &[
            "conservative",
            "tight",
            "passive",
            "nit",
            "cautious",
            "rock",
            "defensive",
        ] {
            let p = style_profile(s);
            assert_eq!(
                resolve_fallback_style(Some(&p), crate::npc::NpcStyle::Aggressive),
                crate::npc::NpcStyle::Conservative,
                "expected Conservative for style {:?}",
                s
            );
        }
    }

    #[test]
    fn profiled_unknown_style_falls_back_to_conservative() {
        let p = style_profile("tricky table captain");
        assert_eq!(
            resolve_fallback_style(Some(&p), crate::npc::NpcStyle::Aggressive),
            crate::npc::NpcStyle::Conservative,
            "unrecognized profile style should fall back to Conservative"
        );
    }

    #[test]
    fn profiled_fallback_overrides_config_style() {
        // Profile says aggressive, config says conservative — profile wins.
        let p = style_profile("loose-aggressive");
        assert_eq!(
            resolve_fallback_style(Some(&p), crate::npc::NpcStyle::Conservative),
            crate::npc::NpcStyle::Aggressive
        );
        // Profile says conservative, config says aggressive — profile wins.
        let p2 = style_profile("tight");
        assert_eq!(
            resolve_fallback_style(Some(&p2), crate::npc::NpcStyle::Aggressive),
            crate::npc::NpcStyle::Conservative
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
    use crate::npc::provider::{LlmProviderConfig, LlmProviderSettings, LlmProviderType};
    use crate::npc::strategy::Position;

    fn ollama_cfg(model: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            settings: LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: Some(model.to_string()),
            },
            api_key: None,
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
        let client = LlmClient::new(ollama_cfg("llama3.2:3b")).expect("test client builds");
        let profile = balanced_profile();
        let snap = preflop_snap();

        match choose_llm_action(&client, &profile, &snap) {
            Ok((action_type, raise_to)) => {
                assert_legal_live_action(&snap, action_type, raise_to);
                eprintln!("llama3.2 preflop chose: {action_type:?} {raise_to:?}");
            }
            Err(reason) => panic!("LLM action failed: {reason:?}"),
        }
    }

    #[test]
    #[ignore]
    fn ollama_llama32_postflop_returns_legal_poker_action() {
        let _guard = LIVE_OLLAMA_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let client = LlmClient::new(ollama_cfg("llama3.2:3b")).expect("test client builds");
        let profile = balanced_profile();
        let snap = postflop_snap();

        match choose_llm_action(&client, &profile, &snap) {
            Ok((action_type, raise_to)) => {
                assert_legal_live_action(&snap, action_type, raise_to);
                eprintln!("llama3.2 postflop chose: {action_type:?} {raise_to:?}");
            }
            Err(reason) => panic!("LLM action failed: {reason:?}"),
        }
    }
}
