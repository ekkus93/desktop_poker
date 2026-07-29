use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use base64::Engine as _;

use crate::{
    crypto::{key_fingerprint, DefaultCryptoProvider, ProtocolCryptoProvider, SigningKeyMaterial},
    domain::{ActionType, PlayerIdentity, TournamentState},
    protocol::{
        JsonSignedEnvelope, PlayerActionSubmission, ProtocolMessageType, SignedEnvelope,
        PROTOCOL_VERSION,
    },
    tournament::{RegisteredPlayer, TournamentController},
};

use super::{super::*, support::sample_tournament_state};

struct StartedRuntimeFixture {
    provider: DefaultCryptoProvider,
    authoritative_state: Arc<Mutex<TournamentState>>,
    tournament_runtime: Arc<Mutex<Option<TournamentController>>>,
    signing_keys: HashMap<String, SigningKeyMaterial>,
}

fn fingerprint(public_key_base64: &str) -> String {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(public_key_base64.as_bytes())
        .expect("test signing public key decodes");
    key_fingerprint(&bytes)
}

fn started_runtime(start_ms: u64) -> StartedRuntimeFixture {
    let provider = DefaultCryptoProvider;
    let mut signing_keys = HashMap::new();
    let registered_players = [("player-a", "Alice", 0_u8), ("player-b", "Bob", 1_u8)]
        .into_iter()
        .map(|(player_id, display_name, seat_index)| {
            let signing = provider.generate_signing_keypair();
            let signing_public_key = signing.public_key_base64();
            let encryption = provider.generate_encryption_keypair();
            let identity = PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: display_name.to_string(),
                signing_public_key: signing_public_key.clone(),
                encryption_public_key: encryption.public_key_base64(),
                signing_key_fingerprint: fingerprint(&signing_public_key),
            };
            signing_keys.insert(player_id.to_string(), signing);
            RegisteredPlayer {
                identity,
                seat_index,
                is_host: seat_index == 0,
                is_ready: true,
            }
        })
        .collect::<Vec<_>>();

    let mut config = sample_tournament_state("table-action-outcomes", 401).config;
    config.max_players = 2;
    config.turn_timer_seconds = 20;
    let mut controller =
        TournamentController::new("table-action-outcomes", 401, config, registered_players)
            .expect("controller builds");
    controller
        .start_tournament(start_ms)
        .expect("tournament starts");
    let authoritative_state = Arc::new(Mutex::new(controller.state().clone()));
    let tournament_runtime = Arc::new(Mutex::new(Some(controller)));

    StartedRuntimeFixture {
        provider,
        authoritative_state,
        tournament_runtime,
        signing_keys,
    }
}

fn current_window(fixture: &StartedRuntimeFixture) -> crate::domain::ActionWindow {
    fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.clone())
        .expect("open action window")
}

fn signed_action(
    fixture: &StartedRuntimeFixture,
    sender_id: &str,
    action_window_id: String,
    seat_index: u8,
    action_type: ActionType,
    raise_to_amount: Option<u32>,
) -> JsonSignedEnvelope {
    let mut envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::ActionSubmissionRequest,
        table_id: "table-action-outcomes".to_string(),
        session_epoch: 401,
        sender_id: sender_id.to_string(),
        counter: 2,
        message_id: format!("action-{sender_id}"),
        server_sequence: None,
        payload: serde_json::to_value(PlayerActionSubmission {
            action_window_id,
            seat_index,
            action_type,
            raise_to_amount,
        })
        .expect("action payload serializes"),
        signature: None,
    };
    envelope
        .sign(
            &fixture.provider,
            fixture
                .signing_keys
                .get(sender_id)
                .expect("sender signing keys"),
        )
        .expect("action envelope signs");
    envelope
}

#[test]
fn remote_timeout_rejection_commits_advanced_authoritative_state() {
    let fixture = started_runtime(0);
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        window.action_window_id,
        window.seat_index,
        ActionType::Fold,
        None,
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("timeout advancement remains a typed outcome");

    let RemoteActionSubmissionOutcome::TimeoutAdvancedThenRejected {
        previous_state,
        after_state,
        error,
    } = outcome
    else {
        panic!("expected timeout-advanced rejection");
    };
    assert!(error.to_string().contains("stale action window"));
    assert_eq!(previous_state, before);
    assert_ne!(after_state, before);
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after timeout"),
        after_state
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &after_state
    );
}

#[test]
fn remote_wrong_player_rejection_does_not_mutate_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let wrong_player = if window.player_id == "player-a" {
        "player-b"
    } else {
        "player-a"
    };
    let envelope = signed_action(
        &fixture,
        wrong_player,
        window.action_window_id,
        window.seat_index,
        ActionType::Fold,
        None,
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("wrong-player rejection is typed");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after rejection"),
        before
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &before
    );
}

#[test]
fn remote_committed_action_updates_runtime_and_authoritative_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        window.action_window_id,
        window.seat_index,
        ActionType::Fold,
        None,
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("legal remote action commits");

    let RemoteActionSubmissionOutcome::Committed {
        previous_state,
        after_state,
    } = outcome
    else {
        panic!("expected committed outcome");
    };
    assert_eq!(previous_state, before);
    assert_ne!(after_state, before);
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after commit"),
        after_state
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &after_state
    );
}


#[test]
fn remote_stale_window_rejection_does_not_mutate_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        "stale-action-window".to_string(),
        window.seat_index,
        ActionType::Fold,
        None,
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("stale-window rejection is typed");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after rejection"),
        before
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &before
    );
}

#[test]
fn remote_invalid_raise_rejection_does_not_mutate_state() {
    let fixture = started_runtime(now_epoch_ms());
    let before = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state")
        .clone();
    let window = current_window(&fixture);
    let envelope = signed_action(
        &fixture,
        &window.player_id,
        window.action_window_id,
        window.seat_index,
        ActionType::Raise,
        Some(0),
    );

    let outcome = handle_action_submission_request(
        &fixture.provider,
        envelope,
        &fixture.authoritative_state,
        &fixture.tournament_runtime,
    )
    .expect("invalid-raise rejection is typed");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::RejectedNoStateChange { .. }
    ));
    assert_eq!(
        *fixture
            .authoritative_state
            .lock()
            .expect("authoritative state after rejection"),
        before
    );
    assert_eq!(
        fixture
            .tournament_runtime
            .lock()
            .expect("runtime")
            .as_ref()
            .expect("controller")
            .state(),
        &before
    );
}
