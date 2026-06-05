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

/// Build the user message: profile description + separator + rendered game state.
pub fn build_user_message(profile: &NpcProfile, snapshot: &GameStateSnapshot) -> String {
    format!(
        "{}\n\n---\n\n{}",
        profile.description,
        render_game_state(snapshot)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BlindLevel, Rank, Suit};

    fn blind_level() -> BlindLevel {
        BlindLevel {
            level_index: 0,
            label: "Level 1".to_string(),
            small_blind: 25,
            big_blind: 50,
            ante: 0,
            duration_seconds: 600,
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
            pot_total: 75,
            call_amount: 50,
            min_raise_to: Some(100),
            max_raise_to: Some(1500),
            stack: 1450,
            position: Position::Late,
            active_player_count: 4,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            blind_level: blind_level(),
            street_history: vec![],
        }
    }

    fn flop_snapshot() -> GameStateSnapshot {
        GameStateSnapshot {
            hand_number: 3,
            street: StreetPhase::Flop,
            board_cards: vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Diamonds,
                },
                Card {
                    rank: Rank::Seven,
                    suit: Suit::Clubs,
                },
            ],
            hole_cards: vec![
                Card {
                    rank: Rank::Queen,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::Jack,
                    suit: Suit::Spades,
                },
            ],
            pot_total: 480,
            call_amount: 200,
            min_raise_to: Some(400),
            max_raise_to: Some(1240),
            stack: 1240,
            position: Position::Late,
            active_player_count: 3,
            legal_actions: vec![ActionType::Fold, ActionType::Call, ActionType::Raise],
            blind_level: blind_level(),
            street_history: vec![
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
            ],
        }
    }

    #[test]
    fn render_preflop_includes_hole_cards_pot_and_options() {
        let snap = preflop_snapshot();
        let text = render_game_state(&snap);
        assert!(text.contains("Hand #1"));
        assert!(text.contains("Pre-flop"));
        assert!(text.contains("A♠"));
        assert!(text.contains("K♥"));
        assert!(text.contains("Pot: 75 chips"));
        assert!(text.contains("call 50"));
        assert!(text.contains("raise to 100–1,500"));
    }

    #[test]
    fn render_flop_includes_board_and_street_history() {
        let snap = flop_snapshot();
        let text = render_game_state(&snap);
        assert!(text.contains("Hand #3"));
        assert!(text.contains("Flop"));
        assert!(text.contains("A♠"));
        assert!(text.contains("K♦"));
        assert!(text.contains("7♣"));
        assert!(text.contains("Action this street:"));
        assert!(text.contains("Seat 1 checked"));
        assert!(text.contains("Seat 3 bet 200"));
    }

    #[test]
    fn build_user_message_includes_profile_description_and_game_state() {
        use super::super::profile::NpcProfile;
        let profile = NpcProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            style: "aggressive".to_string(),
            skill: "intermediate".to_string(),
            description: "Play aggressively at all times.".to_string(),
        };
        let snap = preflop_snapshot();
        let msg = build_user_message(&profile, &snap);
        assert!(msg.contains("Play aggressively at all times."));
        assert!(msg.contains("Hand #1"));
    }

    #[test]
    fn render_omits_raise_line_when_raise_not_legal() {
        let mut snap = preflop_snapshot();
        snap.legal_actions = vec![ActionType::Fold, ActionType::Call];
        snap.min_raise_to = None;
        snap.max_raise_to = None;
        let text = render_game_state(&snap);
        assert!(!text.contains("raise to"));
    }

    #[test]
    fn chip_amounts_above_1000_have_thousands_separator() {
        let snap = preflop_snapshot();
        let text = render_game_state(&snap);
        assert!(text.contains("1,500"));
    }
}
