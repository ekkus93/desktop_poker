use super::profile::NpcProfile;
use super::strategy::Position;
use crate::domain::{ActionType, BlindLevel, Card, Rank, StreetPhase, Suit};

/// A distilled snapshot of game state for building an LLM prompt.
pub struct GameStateSnapshot {
    pub hand_number: u32,
    pub street: StreetPhase,
    pub board_cards: Vec<Card>,
    pub hole_cards: Vec<Card>,
    pub pot_total: u32,
    pub call_amount: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub stack: u32,
    pub position: Position,
    pub active_player_count: u8,
    pub legal_actions: Vec<ActionType>,
    pub blind_level: BlindLevel,
    pub street_history: Vec<StreetAction>,
    /// Rendered output of `NpcSessionHistory::render_context()`.
    pub session_context: Option<String>,
    /// Rendered output of `OpponentStatsTable::render_context()`.
    pub opponent_context: Option<String>,
    /// Short tilt description from `TiltState::description()`.
    pub tilt_description: Option<String>,
}

/// A single action taken during the current betting street.
pub struct StreetAction {
    pub seat_index: u8,
    pub action_type: ActionType,
    pub amount: Option<u32>,
}

fn rank_name(rank: Rank) -> &'static str {
    match rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
    }
}

fn suit_symbol(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "♣",
        Suit::Diamonds => "♦",
        Suit::Hearts => "♥",
        Suit::Spades => "♠",
    }
}

fn fmt_card(card: &Card) -> String {
    format!("{}{}", rank_name(card.rank), suit_symbol(card.suit))
}

fn fmt_chips(amount: u32) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn position_label(pos: Position) -> &'static str {
    match pos {
        Position::Early => "Early",
        Position::Middle => "Middle",
        Position::Late => "Late (button/cutoff)",
    }
}

fn street_label(street: StreetPhase) -> &'static str {
    match street {
        StreetPhase::Preflop => "Pre-flop",
        StreetPhase::Flop => "Flop",
        StreetPhase::Turn => "Turn",
        StreetPhase::River => "River",
        StreetPhase::Showdown => "Showdown",
    }
}

fn action_label(action: ActionType, amount: Option<u32>) -> String {
    match action {
        ActionType::Fold => "folded".to_string(),
        ActionType::Check => "checked".to_string(),
        ActionType::Call => match amount {
            Some(a) => format!("called {}", fmt_chips(a)),
            None => "called".to_string(),
        },
        ActionType::Bet => match amount {
            Some(a) => format!("bet {}", fmt_chips(a)),
            None => "bet".to_string(),
        },
        ActionType::Raise => match amount {
            Some(a) => format!("raised to {}", fmt_chips(a)),
            None => "raised".to_string(),
        },
        ActionType::AllIn => match amount {
            Some(a) => format!("went all-in for {}", fmt_chips(a)),
            None => "went all-in".to_string(),
        },
    }
}

/// Render a game state snapshot as human-readable text for the LLM user message.
pub fn render_game_state(snapshot: &GameStateSnapshot) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "Hand #{} — {}\n",
        snapshot.hand_number,
        street_label(snapshot.street)
    ));

    if !snapshot.board_cards.is_empty() {
        let board: Vec<String> = snapshot.board_cards.iter().map(fmt_card).collect();
        out.push_str(&format!("Board: {}\n", board.join(" ")));
    }

    if !snapshot.hole_cards.is_empty() {
        let hole: Vec<String> = snapshot.hole_cards.iter().map(fmt_card).collect();
        out.push_str(&format!("Your hole cards: {}\n", hole.join(" ")));
    }

    out.push_str(&format!("Pot: {} chips\n", fmt_chips(snapshot.pot_total)));
    out.push_str(&format!(
        "Your stack: {} chips\n",
        fmt_chips(snapshot.stack)
    ));
    out.push_str(&format!(
        "Position: {}\n",
        position_label(snapshot.position)
    ));
    out.push_str(&format!(
        "Players still in hand: {}\n",
        snapshot.active_player_count
    ));
    out.push_str(&format!(
        "Blinds: {} / {}\n",
        fmt_chips(snapshot.blind_level.small_blind),
        fmt_chips(snapshot.blind_level.big_blind)
    ));

    if !snapshot.street_history.is_empty() {
        out.push_str("\nAction this street:\n");
        for act in &snapshot.street_history {
            out.push_str(&format!(
                "- Seat {} {}\n",
                act.seat_index + 1,
                action_label(act.action_type, act.amount)
            ));
        }
    }

    out.push_str("\nYour options:\n");
    let can_raise = snapshot
        .legal_actions
        .iter()
        .any(|a| matches!(a, ActionType::Raise | ActionType::Bet));

    for action in &snapshot.legal_actions {
        match action {
            ActionType::Fold => out.push_str("- fold\n"),
            ActionType::Check => out.push_str("- check\n"),
            ActionType::Call => {
                out.push_str(&format!("- call {}\n", fmt_chips(snapshot.call_amount)))
            }
            ActionType::Bet | ActionType::Raise => {
                if let (Some(min), Some(max)) = (snapshot.min_raise_to, snapshot.max_raise_to) {
                    out.push_str(&format!(
                        "- {} to {}–{}\n",
                        if *action == ActionType::Bet {
                            "bet"
                        } else {
                            "raise"
                        },
                        fmt_chips(min),
                        fmt_chips(max)
                    ));
                }
            }
            ActionType::AllIn => {
                out.push_str(&format!("- all-in for {}\n", fmt_chips(snapshot.stack)))
            }
        }
    }

    // Omit raise bounds lines if raising is not a legal action.
    let _ = can_raise;

    out
}

/// Fixed system prompt that instructs the LLM to respond with a JSON action only.
pub fn build_system_prompt() -> String {
    r#"You are playing No-Limit Texas Hold'em poker. You must decide on a single action.

Respond with ONLY a JSON object — no explanation, no commentary, no additional text.

The JSON must match exactly this schema:
{"action": "<action>", "amount": <integer or null>}

Valid actions: "fold", "check", "call", "raise", "allIn"

Rules:
- "amount" is required (integer) when action is "raise" and must be within the stated bounds.
- "amount" must be null for all other actions.
- Only choose actions that are listed under "Your options".
- If you want to raise but no raise bounds are shown, choose "call" instead."#
        .to_string()
}

/// Approximate token count: characters / 4.
pub fn count_approx_tokens(s: &str) -> usize {
    s.len() / 4
}

const TOKEN_LIMIT: usize = 6_000;

/// Build the user message injecting session history, opponent stats, and tilt state.
///
/// Sections are injected between the profile body and the game state. If the assembled
/// message would exceed 6 000 tokens the context is progressively trimmed.
pub fn build_user_message(profile: &NpcProfile, snapshot: &GameStateSnapshot) -> String {
    let game_state = render_game_state(snapshot);
    let profile_body = &profile.description;

    // Build full context block (may be trimmed below).
    let full_msg = assemble_message(
        profile,
        profile_body,
        snapshot.session_context.as_deref(),
        snapshot.opponent_context.as_deref(),
        snapshot.tilt_description.as_deref(),
        &game_state,
    );

    if count_approx_tokens(&full_msg) <= TOKEN_LIMIT {
        return full_msg;
    }

    // First trim: drop opponent context, keep trimmed session context (3 lines max).
    let trimmed_session = snapshot
        .session_context
        .as_deref()
        .map(|s| truncate_session_context(s, 3));

    let medium_msg = assemble_message(
        profile,
        profile_body,
        trimmed_session.as_deref(),
        None,
        snapshot.tilt_description.as_deref(),
        &game_state,
    );

    if count_approx_tokens(&medium_msg) <= TOKEN_LIMIT {
        return medium_msg;
    }

    // Final trim: drop all context except tilt.
    assemble_message(
        profile,
        profile_body,
        None,
        None,
        snapshot.tilt_description.as_deref(),
        &game_state,
    )
}

/// Assemble the full user message from its parts.
fn assemble_message(
    profile: &NpcProfile,
    profile_body: &str,
    session_context: Option<&str>,
    opponent_context: Option<&str>,
    tilt_description: Option<&str>,
    game_state: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(profile_body.to_string());

    if let Some(opp) = &profile.opponent_tendencies {
        parts.push(format!("## Your opponent tendencies\n{opp}"));
    }

    if let Some(ctx) = session_context {
        if !ctx.is_empty() {
            parts.push(format!("## Session history\n{ctx}"));
        }
    }

    if let Some(ctx) = opponent_context {
        if !ctx.is_empty() {
            parts.push(format!("## Opponent tendencies observed\n{ctx}"));
        }
    }

    if let Some(desc) = tilt_description {
        parts.push(format!("## Current tilt state\nYou are {desc}."));
        if let Some(tb) = &profile.tilt_behaviour {
            parts.push(format!("## Your tilt behaviour\n{tb}"));
        }
    }

    parts.push("---".to_string());
    parts.push(game_state.to_string());

    parts.join("\n\n")
}

/// Keep only the first `n` lines of a session context block.
fn truncate_session_context(ctx: &str, n: usize) -> String {
    ctx.lines().take(n).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BlindLevel, Card, Rank, StreetPhase, Suit};

    fn blind() -> BlindLevel {
        BlindLevel {
            level_index: 0,
            label: "L1".to_string(),
            small_blind: 10,
            big_blind: 20,
            ante: 0,
            duration_seconds: 300,
        }
    }

    fn preflop_snapshot() -> GameStateSnapshot {
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
            pot_total: 30,
            call_amount: 20,
            min_raise_to: Some(40),
            max_raise_to: Some(1500),
            stack: 1500,
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

    fn make_profile() -> NpcProfile {
        NpcProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            style: "aggressive".to_string(),
            skill: "intermediate".to_string(),
            description: "Play aggressively at all times.".to_string(),
            opponent_tendencies: None,
            tilt_behaviour: None,
        }
    }

    #[test]
    fn render_game_state_contains_hand_number_and_street() {
        let snap = preflop_snapshot();
        let text = render_game_state(&snap);
        assert!(text.contains("Hand #1"), "got: {text}");
        assert!(text.contains("Pre-flop"), "got: {text}");
    }

    #[test]
    fn render_game_state_contains_hole_cards() {
        let snap = preflop_snapshot();
        let text = render_game_state(&snap);
        assert!(text.contains("A♠"), "got: {text}");
        assert!(text.contains("K♥"), "got: {text}");
    }

    #[test]
    fn render_game_state_shows_street_history() {
        let mut snap = preflop_snapshot();
        snap.street_history = vec![
            StreetAction {
                seat_index: 0,
                action_type: ActionType::Check,
                amount: None,
            },
            StreetAction {
                seat_index: 2,
                action_type: ActionType::Bet,
                amount: Some(200),
            },
        ];
        let text = render_game_state(&snap);
        assert!(text.contains("Seat 1 checked"), "got: {text}");
        assert!(text.contains("Seat 3 bet 200"), "got: {text}");
    }

    #[test]
    fn build_user_message_with_no_context_matches_phase2_behaviour() {
        let profile = make_profile();
        let snap = preflop_snapshot();
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Play aggressively at all times."));
        assert!(msg.contains("Hand #1"));
        // No context sections should appear.
        assert!(!msg.contains("Session history"));
        assert!(!msg.contains("Opponent tendencies observed"));
        assert!(!msg.contains("Current tilt state"));
    }

    #[test]
    fn build_user_message_includes_profile_description_and_game_state() {
        let profile = NpcProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            style: "aggressive".to_string(),
            skill: "intermediate".to_string(),
            description: "Play aggressively at all times.".to_string(),
            opponent_tendencies: None,
            tilt_behaviour: None,
        };
        let snap = preflop_snapshot();
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Play aggressively at all times."));
        assert!(msg.contains("Hand #1"));
    }

    #[test]
    fn session_context_block_appears_when_set() {
        let profile = make_profile();
        let mut snap = preflop_snapshot();
        snap.session_context = Some("Session: 5 hands played, up 300 chips.".to_string());
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Session history"), "got: {msg}");
        assert!(msg.contains("5 hands played"), "got: {msg}");
    }

    #[test]
    fn opponent_context_block_appears_when_set() {
        let profile = make_profile();
        let mut snap = preflop_snapshot();
        snap.opponent_context = Some("Alice: VPIP 60%, PFR 20%, AF 1.5 (5 hands)".to_string());
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Opponent tendencies observed"), "got: {msg}");
        assert!(msg.contains("Alice"), "got: {msg}");
    }

    #[test]
    fn tilt_description_appears_when_set() {
        let profile = make_profile();
        let mut snap = preflop_snapshot();
        snap.tilt_description = Some("on a 3-hand losing streak (full tilt)".to_string());
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Current tilt state"), "got: {msg}");
        assert!(msg.contains("3-hand losing streak"), "got: {msg}");
    }

    #[test]
    fn profile_opponent_tendencies_and_tilt_behaviour_injected() {
        let profile = NpcProfile {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            style: "aggressive".to_string(),
            skill: "intermediate".to_string(),
            description: "Base strategy.".to_string(),
            opponent_tendencies: Some("Bluff tight players.".to_string()),
            tilt_behaviour: Some("Widen range after losses.".to_string()),
        };
        let mut snap = preflop_snapshot();
        snap.tilt_description = Some("on a 2-hand losing streak (mild tilt)".to_string());
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Your opponent tendencies"), "got: {msg}");
        assert!(msg.contains("Bluff tight players"), "got: {msg}");
        assert!(msg.contains("Your tilt behaviour"), "got: {msg}");
        assert!(msg.contains("Widen range after losses"), "got: {msg}");
    }

    #[test]
    fn message_exceeding_token_limit_is_truncated() {
        let profile = make_profile();
        let mut snap = preflop_snapshot();
        // Generate a session context that is very large.
        let large_ctx = "x".repeat(100_000);
        snap.session_context = Some(large_ctx);
        snap.opponent_context = Some("y".repeat(50_000));
        let msg = build_user_message(&profile, &snap);
        assert!(
            count_approx_tokens(&msg) <= TOKEN_LIMIT,
            "message exceeds token limit: {} tokens",
            count_approx_tokens(&msg)
        );
    }

    #[test]
    fn truncation_preserves_tilt_state() {
        let profile = make_profile();
        let mut snap = preflop_snapshot();
        snap.session_context = Some("x".repeat(100_000));
        snap.opponent_context = Some("y".repeat(50_000));
        snap.tilt_description = Some("on a 4-hand losing streak (full tilt)".to_string());
        let msg = build_user_message(&profile, &snap);
        assert!(
            count_approx_tokens(&msg) <= TOKEN_LIMIT,
            "message exceeds token limit"
        );
        assert!(msg.contains("4-hand losing streak"), "tilt dropped: {msg}");
    }
}
