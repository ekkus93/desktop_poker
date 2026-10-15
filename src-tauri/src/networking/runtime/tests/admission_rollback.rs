use std::sync::{atomic::AtomicU64, Arc, Mutex};

use super::super::{
    handle_join_request, handle_reconnect_request, HostRuntimeHealth, PendingAdmissionRollback,
};
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{ConnectionState, ParticipantState},
    protocol::{test_support::sample_reconnect_request, ReconnectTournamentRequest},
};

use super::support::*;

#[test]
fn snapshot_construction_failure_rolls_back_new_join() {
    let provider = DefaultCryptoProvider;
    let host_signing = provider.generate_signing_keypair();
    let encryption = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let payload =
        sample_join_payload_for_tests("table-failed-join", 46, host_signing.public_key_base64());
    let state = Arc::new(Mutex::new(sample_tournament_state("table-failed-join", 46)));
    let sequence = Arc::new(AtomicU64::new(0));
    let signing = provider.generate_signing_keypair();
    let keys = provider.generate_encryption_keypair();
    let poison = Arc::clone(&encryption);
    assert!(std::thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("synthetic snapshot failure");
    })
    .join()
    .is_err());
    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &signing,
            &keys,
            &payload,
            "failed-join",
            "Failed Join",
            &payload.join_token,
        ),
        &payload,
        &state,
        &sequence,
        &host_signing,
        &encryption,
    );
    assert!(result.is_err());
    assert!(!state
        .lock()
        .unwrap()
        .participants
        .contains_key("failed-join"));
}

#[test]
fn failed_reconnect_snapshot_restores_prior_state() {
    let provider = DefaultCryptoProvider;
    let host_signing = provider.generate_signing_keypair();
    let encryption = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let payload = sample_join_payload_for_tests(
        "table-failed-reconnect",
        47,
        host_signing.public_key_base64(),
    );
    let signing = provider.generate_signing_keypair();
    let keys = provider.generate_encryption_keypair();
    let mut initial = sample_tournament_state("table-failed-reconnect", 47);
    initial.participants.insert(
        "reconnecting-player".to_string(),
        sample_participant_entry(
            "reconnecting-player",
            "Reconnect",
            signing.public_key_base64(),
            keys.public_key_base64(),
            ParticipantState::Reconnecting,
            ConnectionState::Reconnecting,
            None,
            "valid-token",
        ),
    );
    let original = initial.participants["reconnecting-player"].clone();
    let state = Arc::new(Mutex::new(initial));
    let sequence = Arc::new(AtomicU64::new(0));
    let poison = Arc::clone(&encryption);
    assert!(std::thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("synthetic snapshot failure");
    })
    .join()
    .is_err());
    let result = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &signing,
            &payload,
            ReconnectTournamentRequest {
                player_id: "reconnecting-player".to_string(),
                reconnect_token: "valid-token".to_string(),
                ..sample_reconnect_request(Some(1))
            },
            1,
            "failed-reconnect",
        ),
        &payload,
        &state,
        &sequence,
        &host_signing,
        &encryption,
    );
    assert!(result.is_err());
    assert_eq!(
        state.lock().unwrap().participants["reconnecting-player"],
        original
    );
}

#[test]
fn dropping_unregistered_session_reclaims_join_capacity() {
    let provider = DefaultCryptoProvider;
    let host_signing = provider.generate_signing_keypair();
    let encryption = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let payload =
        sample_join_payload_for_tests("table-unregistered", 48, host_signing.public_key_base64());
    let state = Arc::new(Mutex::new(sample_tournament_state(
        "table-unregistered",
        48,
    )));
    let sequence = Arc::new(AtomicU64::new(0));
    let signing = provider.generate_signing_keypair();
    let keys = provider.generate_encryption_keypair();
    let accepted = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &signing,
            &keys,
            &payload,
            "unregistered-player",
            "Unregistered",
            &payload.join_token,
        ),
        &payload,
        &state,
        &sequence,
        &host_signing,
        &encryption,
    )
    .expect("join admission");
    assert!(state
        .lock()
        .unwrap()
        .participants
        .contains_key("unregistered-player"));
    let health = Arc::new(Mutex::new(HostRuntimeHealth::default()));
    drop(PendingAdmissionRollback::new(&state, &accepted, &health));
    assert!(!state
        .lock()
        .unwrap()
        .participants
        .contains_key("unregistered-player"));
}
