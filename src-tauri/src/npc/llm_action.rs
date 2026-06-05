use serde::Deserialize;

use crate::domain::ActionType;

use super::llm_client::LlmError;
use super::prompt::GameStateSnapshot;
use super::runner::first_check_or_call;

#[derive(Debug, Deserialize)]
pub struct LlmActionResponse {
    pub action: String,
    pub amount: Option<u32>,
}

/// Parse the raw text from the LLM into an `LlmActionResponse`.
///
/// Attempts a direct JSON parse first; falls back to searching for the first
/// `{…}` substring to handle responses with leading preamble text.
pub fn parse_llm_response(text: &str) -> Result<LlmActionResponse, LlmError> {
    // Direct parse.
    if let Ok(r) = serde_json::from_str::<LlmActionResponse>(text) {
        return Ok(r);
    }

    // Find the first `{` … `}` span and retry.
    if let Some(start) = text.find('{') {
        let rest = &text[start..];
        let mut depth = 0_i32;
        let mut end = None;
        for (i, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            if let Ok(r) = serde_json::from_str::<LlmActionResponse>(&rest[..end]) {
                return Ok(r);
            }
        }
    }

    Err(LlmError::Parse(
        "no valid JSON action object found".to_string(),
    ))
}

/// Validate and sanitize an `LlmActionResponse` against the legal action window.
///
/// Returns `(ActionType, Option<u32>)` guaranteed to be legal.
pub fn validate_llm_action(
    response: &LlmActionResponse,
    snapshot: &GameStateSnapshot,
) -> (ActionType, Option<u32>) {
    let resolved = map_action_string(&response.action);

    // Verify the resolved action is legal.
    let is_legal = resolved.is_some_and(|at| {
        snapshot.legal_actions.contains(&at)
            || (at == ActionType::Raise && snapshot.legal_actions.contains(&ActionType::Bet))
    });

    if !is_legal {
        return first_check_or_call(&snapshot.legal_actions);
    }

    let action_type = resolved.unwrap();

    match action_type {
        ActionType::Raise | ActionType::Bet => {
            let Some(amount) = response.amount else {
                // amount required for raise/bet; fall back.
                return first_check_or_call(&snapshot.legal_actions);
            };
            let Some(max) = snapshot.max_raise_to else {
                // No raise allowed; fall back.
                return first_check_or_call(&snapshot.legal_actions);
            };
            let min = snapshot.min_raise_to.unwrap_or(amount);
            let clamped = amount.clamp(min, max).min(snapshot.stack);

            let at = if snapshot.legal_actions.contains(&ActionType::Raise) {
                ActionType::Raise
            } else {
                ActionType::Bet
            };
            (at, Some(clamped))
        }
        other => (other, None),
    }
}

fn map_action_string(s: &str) -> Option<ActionType> {
    match s.to_lowercase().replace(['_', '-'], "").as_str() {
        "fold" => Some(ActionType::Fold),
        "check" => Some(ActionType::Check),
        "call" => Some(ActionType::Call),
        "raise" => Some(ActionType::Raise),
        "bet" => Some(ActionType::Bet),
        "allin" => Some(ActionType::AllIn),
        _ => None,
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

    fn full_snapshot(legal: Vec<ActionType>) -> GameStateSnapshot {
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
                    rank: Rank::King,
                    suit: Suit::Hearts,
                },
            ],
            pot_total: 100,
            call_amount: 50,
            min_raise_to: Some(100),
            max_raise_to: Some(1000),
            stack: 2000,
            position: Position::Late,
            active_player_count: 3,
            legal_actions: legal,
            blind_level: blind(),
            street_history: vec![],
        }
    }

    #[test]
    fn clean_json_raise_parses_and_validates_correctly() {
        let text = r#"{"action":"raise","amount":480}"#;
        let resp = parse_llm_response(text).unwrap();
        assert_eq!(resp.action, "raise");
        assert_eq!(resp.amount, Some(480));

        let snap = full_snapshot(vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        let (at, amt) = validate_llm_action(&resp, &snap);
        assert_eq!(at, ActionType::Raise);
        assert_eq!(amt, Some(480));
    }

    #[test]
    fn json_embedded_in_prose_extracts_action() {
        let text = "Sure, I'll raise. Here's my answer: {\"action\":\"raise\",\"amount\":200}";
        let resp = parse_llm_response(text).unwrap();
        assert_eq!(resp.action, "raise");
        assert_eq!(resp.amount, Some(200));
    }

    #[test]
    fn fold_when_not_legal_falls_back_to_check_or_call() {
        let resp = LlmActionResponse {
            action: "fold".to_string(),
            amount: None,
        };
        let snap = full_snapshot(vec![ActionType::Check, ActionType::Raise]);
        let (at, _) = validate_llm_action(&resp, &snap);
        assert_eq!(at, ActionType::Check);
    }

    #[test]
    fn raise_amount_above_max_is_clamped_to_max() {
        let resp = LlmActionResponse {
            action: "raise".to_string(),
            amount: Some(9999),
        };
        let snap = full_snapshot(vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        let (at, amt) = validate_llm_action(&resp, &snap);
        assert_eq!(at, ActionType::Raise);
        assert_eq!(amt, Some(1000)); // max_raise_to = 1000
    }

    #[test]
    fn raise_with_no_max_raise_to_falls_back_to_check_or_call() {
        let resp = LlmActionResponse {
            action: "raise".to_string(),
            amount: Some(300),
        };
        let mut snap = full_snapshot(vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        snap.max_raise_to = None;
        let (at, _) = validate_llm_action(&resp, &snap);
        assert_eq!(at, ActionType::Call);
    }

    #[test]
    fn completely_invalid_json_returns_parse_error() {
        let result = parse_llm_response("this is not json at all");
        assert!(matches!(result, Err(LlmError::Parse(_))));
    }

    #[test]
    fn unknown_action_string_falls_back_to_check_or_call() {
        let resp = LlmActionResponse {
            action: "bluff".to_string(),
            amount: None,
        };
        let snap = full_snapshot(vec![ActionType::Check, ActionType::Call]);
        let (at, _) = validate_llm_action(&resp, &snap);
        // "bluff" maps to None → not legal → fallback
        assert!(matches!(at, ActionType::Check | ActionType::Call));
    }
}
