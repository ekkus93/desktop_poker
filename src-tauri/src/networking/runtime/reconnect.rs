use std::sync::{Arc, Mutex};

use base64::Engine as _;
use rand_core::{OsRng, RngCore};

use crate::domain::{
    ConnectionState, ParticipantRegistryEntry, ParticipantState, TournamentPhase, TournamentState,
};

use super::*;

pub(crate) fn mark_participant_reconnect_eligible(
    authoritative_state: &Arc<Mutex<TournamentState>>,
    player_id: &str,
) -> Result<(), NetworkingError> {
    let mut state = authoritative_state
        .lock()
        .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))?;
    let tournament_phase = state.phase;
    let hand_is_active = state.current_hand.is_some();
    if let Some(participant) = state.participants.get_mut(player_id) {
        if participant.state == ParticipantState::Removed {
            return Ok(());
        }

        participant.connection_state = ConnectionState::Reconnecting;
        if participant.state != ParticipantState::EliminatedObserver {
            participant.state = ParticipantState::Reconnecting;
        }
        let reconnect_state = participant.state;
        participant.reconnect_expiry_ms = Some(
            now_epoch_ms() + reconnect_window_ms(tournament_phase, reconnect_state, hand_is_active),
        );
    }

    Ok(())
}

pub(crate) fn reconnect_window_ms(
    tournament_phase: TournamentPhase,
    participant_state: ParticipantState,
    hand_is_active: bool,
) -> u64 {
    if participant_state == ParticipantState::EliminatedObserver {
        300_000
    } else if tournament_phase != TournamentPhase::Running {
        120_000
    } else if hand_is_active {
        30_000
    } else {
        120_000
    }
}

pub(crate) fn restore_participant_after_reconnect(
    participant: &mut ParticipantRegistryEntry,
    tournament_phase: TournamentPhase,
) {
    participant.connection_state = ConnectionState::Connected;
    participant.reconnect_expiry_ms = None;
    if participant.state == ParticipantState::EliminatedObserver {
        return;
    }

    participant.state = if participant.seat_index.is_some() {
        if tournament_phase == TournamentPhase::Running {
            ParticipantState::Active
        } else {
            ParticipantState::Seated
        }
    } else {
        ParticipantState::Admitted
    };
}

pub(crate) fn is_reconnectable_participant(participant: &ParticipantRegistryEntry) -> bool {
    matches!(
        participant.state,
        ParticipantState::Seated
            | ParticipantState::Active
            | ParticipantState::EliminatedObserver
            | ParticipantState::Reconnecting
            | ParticipantState::Admitted
    ) && participant.state != ParticipantState::Removed
}

/// Merge networking-only fields (reconnect tokens, connection state, admitted
/// timestamps) from the old authoritative state into the new controller-derived
/// state.  The tournament controller never carries these fields — it writes
/// `reconnect_token: None` and `connection_state: Connected` for every
/// participant.  Without this merge, every tick-loop state replacement would
/// silently wipe all reconnect tokens.
pub(crate) fn merge_networking_state(
    authoritative_source: &TournamentState,
    controller_state: &mut TournamentState,
) {
    for (player_id, source_participant) in &authoritative_source.participants {
        if let Some(target_participant) = controller_state.participants.get_mut(player_id) {
            target_participant.reconnect_token = source_participant.reconnect_token.clone();
            target_participant.reconnect_expiry_ms = source_participant.reconnect_expiry_ms;
            target_participant.connection_state = source_participant.connection_state;
            target_participant.admitted_at_ms = source_participant.admitted_at_ms;
        }
    }
}

pub(crate) fn issue_reconnect_token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn missing_reconnect_identity_message() -> String {
    "original reconnect identity is unavailable; v1 requires the original ephemeral signing/encryption keypair"
        .to_string()
}

pub(crate) fn is_stale_server_sequence(
    last_seen_server_sequence: Option<u64>,
    next_server_sequence: Option<u64>,
) -> bool {
    matches!(
        (last_seen_server_sequence, next_server_sequence),
        (Some(last_seen), Some(next_sequence)) if next_sequence <= last_seen
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ConnectionState, ParticipantRegistryEntry, ParticipantState, PlayerIdentity,
        TournamentPhase, TournamentState,
    };

    fn test_participant(
        player_id: &str,
        state: ParticipantState,
        seat_index: Option<u8>,
    ) -> ParticipantRegistryEntry {
        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: player_id.to_string(),
                signing_public_key: format!("sign-{player_id}"),
                encryption_public_key: format!("enc-{player_id}"),
                signing_key_fingerprint: format!("fp-{player_id}"),
            },
            state,
            connection_state: ConnectionState::Connected,
            seat_index,
            admitted_at_ms: 0,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: false,
        }
    }

    // T4.1 — is_stale_server_sequence boundary cases
    #[test]
    fn is_stale_server_sequence_false_when_both_none() {
        assert!(!is_stale_server_sequence(None, None));
    }

    #[test]
    fn is_stale_server_sequence_false_when_last_seen_none() {
        assert!(!is_stale_server_sequence(None, Some(1)));
    }

    #[test]
    fn is_stale_server_sequence_false_when_next_none() {
        assert!(!is_stale_server_sequence(Some(5), None));
    }

    #[test]
    fn is_stale_server_sequence_false_when_next_strictly_ahead() {
        assert!(!is_stale_server_sequence(Some(5), Some(6)));
    }

    #[test]
    fn is_stale_server_sequence_true_when_next_equals_last_seen() {
        assert!(is_stale_server_sequence(Some(5), Some(5)));
    }

    #[test]
    fn is_stale_server_sequence_true_when_next_is_behind() {
        assert!(is_stale_server_sequence(Some(5), Some(4)));
    }

    // T4.2 — reconnect_window_ms covers all four branches
    #[test]
    fn reconnect_window_ms_eliminated_observer_returns_300000() {
        assert_eq!(
            reconnect_window_ms(
                TournamentPhase::Running,
                ParticipantState::EliminatedObserver,
                true
            ),
            300_000
        );
    }

    #[test]
    fn reconnect_window_ms_non_running_phase_returns_120000() {
        assert_eq!(
            reconnect_window_ms(
                TournamentPhase::WaitingForPlayers,
                ParticipantState::Active,
                true
            ),
            120_000
        );
    }

    #[test]
    fn reconnect_window_ms_running_with_active_hand_returns_30000() {
        assert_eq!(
            reconnect_window_ms(TournamentPhase::Running, ParticipantState::Active, true),
            30_000
        );
    }

    #[test]
    fn reconnect_window_ms_running_without_hand_returns_120000() {
        assert_eq!(
            reconnect_window_ms(TournamentPhase::Running, ParticipantState::Active, false),
            120_000
        );
    }

    // T4.3 — is_reconnectable_participant
    #[test]
    fn is_reconnectable_participant_true_for_eligible_states() {
        for state in [
            ParticipantState::Seated,
            ParticipantState::Active,
            ParticipantState::EliminatedObserver,
            ParticipantState::Reconnecting,
            ParticipantState::Admitted,
        ] {
            let participant = test_participant("px", state, Some(0));
            assert!(
                is_reconnectable_participant(&participant),
                "expected reconnectable for {state:?}"
            );
        }
    }

    #[test]
    fn is_reconnectable_participant_false_for_removed() {
        let participant = test_participant("px", ParticipantState::Removed, Some(0));
        assert!(!is_reconnectable_participant(&participant));
    }

    // T4.4 — restore_participant_after_reconnect state transitions
    #[test]
    fn restore_participant_keeps_eliminated_observer_state() {
        let mut p = test_participant("px", ParticipantState::EliminatedObserver, Some(0));
        p.reconnect_expiry_ms = Some(9999);
        p.connection_state = ConnectionState::Reconnecting;
        restore_participant_after_reconnect(&mut p, TournamentPhase::Running);
        assert_eq!(p.state, ParticipantState::EliminatedObserver);
        assert_eq!(p.connection_state, ConnectionState::Connected);
        assert_eq!(p.reconnect_expiry_ms, None);
    }

    #[test]
    fn restore_participant_seated_in_running_becomes_active() {
        let mut p = test_participant("px", ParticipantState::Seated, Some(1));
        restore_participant_after_reconnect(&mut p, TournamentPhase::Running);
        assert_eq!(p.state, ParticipantState::Active);
    }

    #[test]
    fn restore_participant_seated_in_waiting_stays_seated() {
        let mut p = test_participant("px", ParticipantState::Seated, Some(1));
        restore_participant_after_reconnect(&mut p, TournamentPhase::WaitingForPlayers);
        assert_eq!(p.state, ParticipantState::Seated);
    }

    #[test]
    fn restore_participant_no_seat_index_in_running_becomes_admitted() {
        let mut p = test_participant("px", ParticipantState::Reconnecting, None);
        restore_participant_after_reconnect(&mut p, TournamentPhase::Running);
        assert_eq!(p.state, ParticipantState::Admitted);
    }

    // T4.5 — merge_networking_state preserves networking fields only
    #[test]
    fn merge_networking_state_copies_networking_fields_but_not_participant_state() {
        use crate::domain::{BlindSchedule, TournamentConfig};
        use std::collections::BTreeMap;

        let make_state = |reconnect_token: Option<String>,
                          expiry: Option<u64>,
                          conn: ConnectionState,
                          admitted: u64,
                          state: ParticipantState|
         -> TournamentState {
            let mut participants = BTreeMap::new();
            let mut p = test_participant("p1", state, Some(0));
            p.reconnect_token = reconnect_token;
            p.reconnect_expiry_ms = expiry;
            p.connection_state = conn;
            p.admitted_at_ms = admitted;
            participants.insert("p1".to_string(), p);
            TournamentState {
                table_id: "t".to_string(),
                session_epoch: 1,
                phase: TournamentPhase::Running,
                config: TournamentConfig {
                    tournament_name: "T".to_string(),
                    table_name: None,
                    max_players: 2,
                    starting_stack: 1000,
                    turn_timer_seconds: 10,
                    blind_schedule: BlindSchedule { levels: vec![] },
                },
                blind_schedule: BlindSchedule { levels: vec![] },
                blind_level_index: 0,
                participants,
                seats: vec![],
                hand_results: vec![],
                placements: vec![],
                current_hand: None,
            }
        };

        let source = make_state(
            Some("token-from-source".to_string()),
            Some(5000),
            ConnectionState::Reconnecting,
            42,
            ParticipantState::Reconnecting,
        );
        let mut target = make_state(
            None,
            None,
            ConnectionState::Connected,
            0,
            ParticipantState::Active,
        );

        merge_networking_state(&source, &mut target);

        let p = target.participants.get("p1").unwrap();
        // Networking fields: taken from source
        assert_eq!(p.reconnect_token, Some("token-from-source".to_string()));
        assert_eq!(p.reconnect_expiry_ms, Some(5000));
        assert_eq!(p.connection_state, ConnectionState::Reconnecting);
        assert_eq!(p.admitted_at_ms, 42);
        // Controller-derived fields: NOT overwritten
        assert_eq!(p.state, ParticipantState::Active);
    }

    #[test]
    fn merge_networking_state_skips_participant_not_in_target() {
        use crate::domain::{BlindSchedule, TournamentConfig};
        use std::collections::BTreeMap;

        let blank_state = |participants| TournamentState {
            table_id: "t".to_string(),
            session_epoch: 1,
            phase: TournamentPhase::Running,
            config: TournamentConfig {
                tournament_name: "T".to_string(),
                table_name: None,
                max_players: 2,
                starting_stack: 1000,
                turn_timer_seconds: 10,
                blind_schedule: BlindSchedule { levels: vec![] },
            },
            blind_schedule: BlindSchedule { levels: vec![] },
            blind_level_index: 0,
            participants,
            seats: vec![],
            hand_results: vec![],
            placements: vec![],
            current_hand: None,
        };

        let mut source_participants = BTreeMap::new();
        source_participants.insert(
            "ghost".to_string(),
            test_participant("ghost", ParticipantState::Active, Some(0)),
        );
        let source = blank_state(source_participants);
        let mut target = blank_state(BTreeMap::new());

        // Must not panic when source has participant not in target
        merge_networking_state(&source, &mut target);
        assert!(target.participants.is_empty());
    }

    // T4.6 — issue_reconnect_token format and uniqueness
    #[test]
    fn issue_reconnect_token_is_base64url_safe_and_correct_length() {
        let token = issue_reconnect_token();
        assert!(!token.is_empty());
        // base64url no-pad: 24 bytes → 32 chars
        assert_eq!(token.len(), 32, "expected 32-char base64url-no-pad token");
        assert!(
            token
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
            "token contains non-base64url characters: {token}"
        );
    }

    #[test]
    fn issue_reconnect_token_generates_unique_tokens() {
        let a = issue_reconnect_token();
        let b = issue_reconnect_token();
        assert_ne!(a, b, "consecutive tokens should differ (probabilistic)");
    }
}
