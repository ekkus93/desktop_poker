use std::time::{Duration, Instant};

use super::support::*;
use crate::{crypto::DefaultCryptoProvider, networking::ClientRuntimeEvent};

#[test]
fn host_shutdown_closes_client_socket_and_reconnect_failure_becomes_terminal() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-host-shutdown", 141);
    let client = connect_test_client(&provider, &host, "player-shutdown", "Shutdown");
    let _ = expect_snapshot_event(&client);

    host.request_shutdown();
    drop(host);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_reconnecting = false;
    let mut safe_error = None;
    let mut saw_disconnected = false;

    while Instant::now() < deadline && !saw_disconnected {
        match client.next_event(Duration::from_millis(200)) {
            Ok(ClientRuntimeEvent::Reconnecting { player_id }) => {
                assert_eq!(player_id, "player-shutdown");
                saw_reconnecting = true;
            }
            Ok(ClientRuntimeEvent::SafeError { player_id, message }) => {
                assert_eq!(player_id, "player-shutdown");
                safe_error = Some(message);
            }
            Ok(ClientRuntimeEvent::Disconnected { player_id }) => {
                assert_eq!(player_id, "player-shutdown");
                saw_disconnected = true;
            }
            Ok(other) => panic!("unexpected client event during host shutdown: {other:?}"),
            Err(_) => {}
        }
    }

    assert!(
        saw_reconnecting,
        "client must enter reconnecting after host shutdown"
    );
    let safe_error = safe_error.expect("failed reconnect must produce an explicit safe error");
    assert!(
        safe_error.contains("failed to connect to host"),
        "unexpected reconnect failure: {safe_error}"
    );
    assert!(
        saw_disconnected,
        "client must receive a terminal disconnected event after reconnect fails"
    );
}
