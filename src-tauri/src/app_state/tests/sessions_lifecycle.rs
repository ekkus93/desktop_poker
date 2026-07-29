use super::super::{
    ClaimLobbySeatRequest, DesktopAppState, DesktopJoinSessionErrorCode,
    DesktopTableActionErrorCode, DesktopTableActionKind, SetLobbyReadyStateRequest,
    TableViewerMode,
};
use crate::{
    networking::HostRuntimeMode,
    npc::{LlmProviderConfig, LlmProviderType},
    protocol::decode_join_payload,
};

use super::support::*;

#[test]
fn start_host_session_returns_a_live_invite_from_the_running_host() {
    let state = DesktopAppState::detect();

    let status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    assert_eq!(status.tournament_name, "Friday Finals");
    assert_eq!(status.table_name, "Main Table");
    assert_eq!(status.advertised_host, "127.0.0.1");
    assert_eq!(status.phase, "waitingForPlayers");
    assert_eq!(status.active_seat_count, 1);
    assert_eq!(status.open_seat_count, 5);
    assert_eq!(status.participants.len(), 1);
    assert_eq!(status.participants[0].display_name, "Host Alpha");
    assert_eq!(status.participants[0].connection_state, "connected");
    assert!(status.invite.starts_with("pkr1_"));

    let decoded = decode_join_payload(&status.invite).expect("invite should decode");
    assert_eq!(decoded.host_address, "127.0.0.1");
    assert_eq!(decoded.host_port, status.host_port);
    assert_eq!(decoded.table_name.as_deref(), Some("Friday Finals"));
    assert_eq!(decoded.table_id, status.table_id);
    assert_eq!(decoded.session_epoch, status.session_epoch);

    let active_status = state
        .host_session_status()
        .expect("host session status should resolve")
        .expect("host session should remain active");
    assert_eq!(active_status.invite, status.invite);
}

#[test]
fn stop_host_session_clears_active_host_status() {
    let state = DesktopAppState::detect();

    state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");
    state.stop_host_session().expect("host session should stop");

    assert!(state
        .host_session_status()
        .expect("host session status should resolve")
        .is_none());
}

#[test]
fn detect_starts_a_fresh_host_state_after_a_prior_instance_was_hosting() {
    let prior_state = DesktopAppState::detect();

    prior_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("prior host session should start");

    let restarted_state = DesktopAppState::detect();

    assert!(restarted_state
        .host_session_status()
        .expect("restarted host status should resolve")
        .is_none());
    assert!(restarted_state
        .client_session_status()
        .expect("restarted client status should resolve")
        .is_none());
}

#[test]
fn start_host_session_rejects_replacing_an_active_host_session() {
    let state = DesktopAppState::detect();

    let first_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    let error = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect_err("second host session should be rejected");

    assert_eq!(
        error,
        "stop the active host session before starting a new table"
    );
    assert_eq!(
        state
            .host_session_status()
            .expect("host session status should resolve")
            .expect("original host session should remain active")
            .invite,
        first_status.invite,
    );
}

#[test]
fn join_host_session_returns_the_initial_live_snapshot() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    let client_state = DesktopAppState::detect();
    let client_status = client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join live host");

    assert_eq!(client_status.host_address, "127.0.0.1");
    assert_eq!(client_status.host_port, host_status.host_port);
    assert_eq!(client_status.phase, "waitingForPlayers");
    assert_eq!(client_status.active_seat_count, 1);
    assert_eq!(client_status.open_seat_count, 5);
    assert_eq!(client_status.tournament_name, "Friday Finals");
    assert!(client_status
        .participants
        .iter()
        .any(|participant| participant.is_host && participant.display_name == "Host Alpha"));
    assert!(client_status
        .participants
        .iter()
        .any(|participant| participant.display_name == "Client Bravo"));

    let refreshed_host_status = host_state
        .host_session_status()
        .expect("host session status should resolve")
        .expect("host session should remain active");
    assert!(refreshed_host_status
        .participants
        .iter()
        .any(|participant| participant.display_name == "Client Bravo"));
}

#[test]
fn join_host_session_classifies_invalid_payload_without_message_matching() {
    let state = DesktopAppState::detect();
    let error = state
        .join_host_session(sample_join_host_session_request("not-a-valid-invite"))
        .expect_err("invalid invite should be rejected");

    assert_eq!(
        error.code(),
        DesktopJoinSessionErrorCode::InvalidJoinPayload
    );
    assert!(!error.message().is_empty());
}

#[test]
fn join_host_session_rejects_replacing_an_active_client_session() {
    let first_host_state = DesktopAppState::detect();
    let first_host_status = first_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");
    let second_host_state = DesktopAppState::detect();
    let second_host_status = second_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    let client_state = DesktopAppState::detect();
    let first_client_status = client_state
        .join_host_session(sample_join_host_session_request(&first_host_status.invite))
        .expect("client should join the first host");

    let error = client_state
        .join_host_session(sample_join_host_session_request(&second_host_status.invite))
        .expect_err("second client join should be rejected");

    assert_eq!(error.code(), DesktopJoinSessionErrorCode::CommandFailed);
    assert_eq!(
        error.message(),
        "leave the active client session before joining another table"
    );

    let active_client_status = client_state
        .client_session_status()
        .expect("client session status should resolve")
        .expect("original client session should remain active");
    assert_eq!(active_client_status.table_id, first_client_status.table_id);
    assert_eq!(
        active_client_status.host_port,
        first_client_status.host_port
    );

    assert_eq!(
        second_host_state
            .host_session_status()
            .expect("second host session status should resolve")
            .expect("second host session should remain active")
            .participants
            .len(),
        1,
    );
}

#[test]
fn host_mutations_reject_missing_host_sessions_clearly() {
    let state = DesktopAppState::detect();

    assert_eq!(
        state
            .host_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 0 })
            .expect_err("claiming a host seat without a host session should fail"),
        "no active host session",
    );
    assert_eq!(
        state
            .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("setting host ready state without a host session should fail"),
        "no active host session",
    );
    assert_eq!(
        state
            .host_start_tournament()
            .expect_err("starting a tournament without a host session should fail"),
        "no active host session",
    );
}

#[test]
fn client_mutations_reject_missing_client_sessions_clearly() {
    let state = DesktopAppState::detect();

    assert_eq!(
        state
            .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
            .expect_err("claiming a client seat without a client session should fail"),
        "no active client session",
    );
    assert_eq!(
        state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("setting client ready state without a client session should fail"),
        "no active client session",
    );
}

#[test]
fn client_ready_state_requires_a_claimed_seat() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");
    let client_state = DesktopAppState::detect();

    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join the host before claiming a seat");

    assert_eq!(
        client_state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect_err("readying before claiming a seat should fail"),
        "ready state requires a claimed seat",
    );

    let refreshed_client_status = client_state
        .client_session_status()
        .expect("client session status should resolve")
        .expect("client session should remain active");
    let local_participant = refreshed_client_status
        .participants
        .iter()
        .find(|participant| participant.display_name == "Client Bravo")
        .expect("client participant should remain visible");
    assert_eq!(local_participant.seat_index, None);
    assert!(!local_participant.is_ready);
}

// ── Host lifecycle (11.1) ─────────────────────────────────────────────────────

#[test]
fn start_host_session_rejects_invalid_host_address() {
    let state = DesktopAppState::detect();

    let error = state
        .start_host_session_with_mode(
            sample_host_session_request("not-a-valid-ip"),
            HostRuntimeMode::Test,
        )
        .expect_err("host session with invalid address must fail");

    assert!(
        !error.is_empty(),
        "bind failure must produce a non-empty error; got: {error}"
    );
}

#[test]
fn stop_and_restart_host_session_succeeds() {
    let state = DesktopAppState::detect();

    let first_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    state.stop_host_session().expect("host session should stop");

    assert!(
        state
            .host_session_status()
            .expect("status should resolve")
            .is_none(),
        "host session should be absent after stop"
    );

    let second_status = state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    assert_ne!(
        first_status.table_id, second_status.table_id,
        "restart must produce a fresh session with a new table ID"
    );
    assert!(second_status.invite.starts_with("pkr1_"));
}

#[test]
fn stop_host_session_is_idempotent_when_no_session_is_active() {
    let state = DesktopAppState::detect();

    // No session started — stop should not error.
    state
        .stop_host_session()
        .expect("stopping a non-existent host session must not error");
}

// ── Client lifecycle (11.1) ───────────────────────────────────────────────────

#[test]
fn leave_client_session_clears_the_active_session() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");

    assert!(
        client_state
            .client_session_status()
            .expect("client session status should resolve")
            .is_some(),
        "client session should be active before leaving"
    );

    client_state
        .leave_client_session()
        .expect("leave must succeed");

    assert!(
        client_state
            .client_session_status()
            .expect("client session status should resolve")
            .is_none(),
        "client session should be absent after leaving"
    );
}

#[test]
fn leave_client_session_permits_rejoining_a_new_host() {
    let host_state = DesktopAppState::detect();
    let first_host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("first host session should start");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&first_host_status.invite))
        .expect("client should join first host");
    client_state
        .leave_client_session()
        .expect("client should leave first host");

    let second_host_state = DesktopAppState::detect();
    let second_host_status = second_host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("second host session should start");

    let rejoined = client_state
        .join_host_session(sample_join_host_session_request(&second_host_status.invite))
        .expect("client should rejoin after leaving");

    assert_eq!(rejoined.table_id, second_host_status.table_id);
}

#[test]
fn leave_client_session_is_idempotent_when_no_session_is_active() {
    let state = DesktopAppState::detect();

    // No session started — leave should not error.
    state
        .leave_client_session()
        .expect("leaving a non-existent client session must not error");
}

#[test]
fn leave_client_session_does_not_clear_llm_provider_config() {
    let _lock = PROVIDER_CFG_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let state = DesktopAppState::detect();

    state
        .set_llm_provider_config(LlmProviderConfig {
            settings: crate::npc::LlmProviderSettings {
                provider: LlmProviderType::Ollama,
                endpoint_url: None,
                model: None,
            },
            api_key: None,
        })
        .expect("set provider config");

    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");
    state.leave_client_session().expect("client should leave");

    let config_after_leave = state
        .get_llm_provider_config()
        .expect("provider config should be accessible after client leave");

    assert!(
        config_after_leave.is_some(),
        "leaving a client session must not clear the LLM provider config"
    );
}

// ── 3.4 Teardown isolation ────────────────────────────────────────────────────

#[test]
fn stop_host_session_does_not_clear_client_session_if_both_existed() {
    // Hosts and clients are mutually exclusive in one instance (the join command
    // rejects if a host session is active), but we can verify at the struct level
    // that stop_host_session only touches the host_session field.
    let state = DesktopAppState::detect();

    let host_a = DesktopAppState::detect();
    let host_status = host_a
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session should start");

    // Join from `state` (it is a separate instance that has no host).
    state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client should join");

    // Stop the *host* from host_a — this must not clear state's client session.
    host_a.stop_host_session().expect("host should stop");

    // state's client session must still be present (though the connection may
    // be degrading in the background — we only check the field is set).
    assert!(
        state
            .client_session_status()
            .expect("client session status should resolve")
            .is_some(),
        "stopping a host on a separate instance must not clear the client session on this instance"
    );
}

// P0.5 — table action returns an explicit error; never silently succeeds without a game in progress.

#[test]
fn client_table_action_returns_explicit_error_when_no_action_window_is_open() {
    let host_state = DesktopAppState::detect();
    let host_status = host_state
        .start_host_session_with_mode(
            sample_host_session_request("127.0.0.1"),
            HostRuntimeMode::Test,
        )
        .expect("host session starts");

    let client_state = DesktopAppState::detect();
    client_state
        .join_host_session(sample_join_host_session_request(&host_status.invite))
        .expect("client joins");

    // No tournament has started yet — there is no open action window.
    // The command must return an explicit descriptive error, not a silent no-op.
    let err = client_state
        .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
        .expect_err("table action with no open window must fail");

    assert_eq!(
        err.code(),
        DesktopTableActionErrorCode::StaleActionWindow,
        "a joined client without an active action window should receive the stable stale-window code"
    );
    assert!(
        err.message().contains("no open action window"),
        "error should describe why the action failed; got: {err}"
    );
}
