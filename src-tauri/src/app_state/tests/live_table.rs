use super::super::{
    ClaimLobbySeatRequest, DesktopAppState, DesktopTableActionKind, SetLobbyReadyStateRequest,
    TableViewerMode, LOCAL_PLAYER_ID,
};
use crate::networking::HostRuntimeMode;

use super::support::*;

#[test]
fn live_table_view_prefers_the_authoritative_session_snapshot() {
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
        .expect("client should join live host");

    host_state
        .host_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 0 })
        .expect("host seat claim should succeed");
    client_state
        .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
        .expect("client seat claim should succeed");
    host_state
        .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
        .expect("host ready should succeed");
    client_state
        .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
        .expect("client ready should succeed");
    host_state
        .host_start_tournament()
        .expect("host should start the live tournament");

    let host_table_view = (0..20)
        .find_map(|_| {
            let next_view = host_state
                .table_view(TableViewerMode::Local)
                .expect("host live table view");
            if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
                Some(next_view)
            } else {
                std::thread::sleep(std::time::Duration::from_millis(20));
                None
            }
        })
        .expect("host live table view should expose the running snapshot");
    assert_eq!(host_table_view.tournament_name, "Friday Finals");
    assert_eq!(host_table_view.table_id, host_status.table_id);
    assert_eq!(host_table_view.phase_label, "Running");
    assert_eq!(host_table_view.current_hand_number, Some(1));
    assert!(host_table_view
        .seats
        .iter()
        .all(|seat| seat.display_name != "Waiting for player"));

    let client_table_view = (0..40)
        .find_map(|_| {
            let next_view = client_state
                .table_view(TableViewerMode::Local)
                .expect("client live table view");
            let local_has_cards = next_view
                .seats
                .iter()
                .any(|s| s.is_local && s.hole_cards.len() == 2);
            if next_view.phase_label == "Running"
                && next_view.current_hand_number == Some(1)
                && local_has_cards
            {
                Some(next_view)
            } else {
                std::thread::sleep(std::time::Duration::from_millis(25));
                None
            }
        })
        .expect("client should observe the running live table view with hole cards");
    assert_eq!(client_table_view.phase_label, "Running");
    let local_client_seat = client_table_view
        .seats
        .iter()
        .find(|seat| seat.is_local)
        .expect("client local seat should exist");
    assert_eq!(local_client_seat.seat_index, 2);
    assert_eq!(local_client_seat.hole_cards.len(), 2);
    assert!(client_table_view
        .seats
        .iter()
        .filter(|seat| !seat.is_local && seat.display_name != "Open seat")
        .all(|seat| seat.hole_cards.is_empty()));
}

#[test]
fn live_table_actions_route_through_the_real_session_runtime() {
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
        .expect("client should join live host");

    client_state
        .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
        .expect("client seat claim should succeed");
    host_state
        .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
        .expect("host ready should succeed");
    client_state
        .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
        .expect("client ready should succeed");
    host_state
        .host_start_tournament()
        .expect("host should start the live tournament");

    let running_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let next_view = host_state
            .table_view(TableViewerMode::Local)
            .expect("host running table view before live action");
        if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
            break;
        }

        assert!(
            std::time::Instant::now() < running_deadline,
            "host should expose the running table before live action"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    loop {
        let next_view = client_state
            .table_view(TableViewerMode::Local)
            .expect("client running table view before live action");
        if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
            break;
        }

        assert!(
            std::time::Instant::now() < running_deadline,
            "client should expose the running table before live action"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // Decide who must act from the host's always-fresh authoritative state rather
    // than from whichever instance happens to surface an open window first. The host
    // id is the constant `LOCAL_PLAYER_ID`; the client's authoritative id is
    // `player-{instance_id}`. Reading the host authority directly never depends on a
    // network-delivered snapshot, so it is robust under heavy parallel test load that
    // can starve the client's snapshot-receive thread.
    let window_deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let acting_player_id = loop {
        let window_owner = host_state
            .host_session
            .lock()
            .expect("host session lock")
            .as_ref()
            .and_then(|session| {
                session
                    .host_server
                    .authoritative_state()
                    .ok()
                    .and_then(|state| state.current_hand)
                    .and_then(|hand| hand.action_window)
                    .map(|window| window.player_id)
            });
        if let Some(player_id) = window_owner {
            break player_id;
        }

        assert!(
            std::time::Instant::now() < window_deadline,
            "host authority should expose an open action window"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };

    let host_is_acting = acting_player_id == LOCAL_PLAYER_ID;

    // When the client is the actor, wait for its snapshot to catch up to the
    // authoritative window before acting through it. This bounded active wait is the
    // fix for the prior flakiness: it no longer shares a single deadline with window
    // detection, so a delayed snapshot under load can't make the test miss the window.
    if !host_is_acting {
        let sync_deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let client_sees_window = client_state
                .client_session
                .lock()
                .expect("client session lock")
                .as_ref()
                .and_then(|session| {
                    session
                        .latest_snapshot
                        .state
                        .current_hand
                        .as_ref()
                        .and_then(|hand| {
                            hand.action_window.as_ref().map(|window| {
                                window.player_id == session.latest_snapshot.local_player_id
                            })
                        })
                })
                .unwrap_or(false);
            if client_sees_window {
                break;
            }

            assert!(
                std::time::Instant::now() < sync_deadline,
                "client snapshot should reflect its open action window"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    let (acting_state, observing_state) = if host_is_acting {
        (&host_state, &client_state)
    } else {
        (&client_state, &host_state)
    };
    let acting_view_before = acting_state
        .table_view(TableViewerMode::Local)
        .expect("acting table view before live action");
    let observing_view_before = observing_state
        .table_view(TableViewerMode::Local)
        .expect("observing table view before live action");
    assert_eq!(acting_view_before.phase_label, "Running");

    let acting_after_action = acting_state
        .submit_table_action(
            TableViewerMode::Local,
            DesktopTableActionKind::CheckOrCall,
            None,
        )
        .expect("live action should succeed");
    assert_eq!(acting_after_action.phase_label, "Running");
    assert!(
        acting_after_action.action_owner_label != acting_view_before.action_owner_label
            || acting_after_action.current_hand_number != acting_view_before.current_hand_number
            || acting_after_action.event_feed.len() > acting_view_before.event_feed.len()
            || acting_after_action.action_tray.is_none(),
        "live action should change the acting view"
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let observer_after_action = loop {
        let next_view = observing_state
            .table_view(TableViewerMode::Local)
            .expect("observer table view after live action");
        if next_view.action_owner_label != observing_view_before.action_owner_label
            || next_view.current_hand_number != observing_view_before.current_hand_number
            || next_view.event_feed.len() > observing_view_before.event_feed.len()
        {
            break next_view;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "observer should observe the live action update"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(observer_after_action.phase_label, "Running");
    assert!(
        observer_after_action.action_owner_label != observing_view_before.action_owner_label
            || observer_after_action.current_hand_number
                != observing_view_before.current_hand_number
            || observer_after_action.event_feed.len() > observing_view_before.event_feed.len(),
        "observer should observe the live action update"
    );
}
