from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "src-tauri/src/networking/runtime/handlers.rs",
    """    if controller.state() != &before_state {
        return Err(NetworkingError::new(
            "tournament runtime diverged from authoritative state before remote action",
        ));
    }""",
    """    let mut normalized_controller_state = controller.state().clone();
    merge_networking_state(&before_state, &mut normalized_controller_state);
    if normalized_controller_state != before_state {
        return Err(NetworkingError::new(
            "tournament runtime diverged from authoritative gameplay state before remote action",
        ));
    }""",
)

path = Path("src-tauri/src/networking/runtime/tests/action_outcomes.rs")
text = path.read_text(encoding="utf-8")
test = r'''

#[test]
fn networking_only_authoritative_fields_do_not_trigger_false_divergence() {
    let fixture = started_runtime(now_epoch_ms());
    let window = current_window(&fixture);
    {
        let mut authoritative = fixture
            .authoritative_state
            .lock()
            .expect("authoritative state");
        let participant = authoritative
            .participants
            .get_mut(&window.player_id)
            .expect("acting participant");
        participant.reconnect_token = Some("network-only-token".to_string());
        participant.admitted_at_ms = 777;
    }
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
    .expect("networking-only metadata must not look like gameplay divergence");

    assert!(matches!(
        outcome,
        RemoteActionSubmissionOutcome::Committed { .. }
    ));
    let authoritative = fixture
        .authoritative_state
        .lock()
        .expect("authoritative state after action");
    let participant = authoritative
        .participants
        .get(&window.player_id)
        .expect("acting participant after action");
    assert_eq!(
        participant.reconnect_token.as_deref(),
        Some("network-only-token")
    );
    assert_eq!(participant.admitted_at_ms, 777);
}
'''
if "networking_only_authoritative_fields_do_not_trigger_false_divergence" not in text:
    path.write_text(text + test, encoding="utf-8")
