from pathlib import Path

path = Path("src-tauri/src/networking/runtime/tests/end_to_end/basic.rs")
text = path.read_text(encoding="utf-8")

helper = r'''fn wait_for_safe_error(client: &crate::networking::ClientRuntime) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match client.next_event(Duration::from_millis(200)) {
            Ok(crate::networking::ClientRuntimeEvent::SafeError { message, .. }) => return message,
            Ok(_) => {}
            Err(crate::networking::ClientRuntimePollError::Timeout) => {}
            Err(crate::networking::ClientRuntimePollError::Disconnected) => {
                panic!("client runtime disconnected before the rejection arrived")
            }
        }
        assert!(
            Instant::now() < deadline,
            "remote action rejection should arrive before timeout"
        );
    }
}

'''
if "fn wait_for_safe_error(client:" not in text:
    marker = "/// A whole Sit 'n Go played end-to-end"
    if marker not in text:
        raise SystemExit("expected first end-to-end test marker was not found")
    text = text.replace(marker, helper + marker, 1)

old = '''    wait_for_client_command_connection(actor);
    wait_for_client_command_connection(offender);

    // (1) Out-of-turn: the non-acting client submits for the open window. The host
    // attributes the action to the sender's own id (it never trusts a claimed
    // player), so the controller rejects it — it is not that player's turn.
    let _ = offender.submit_action(
        window.action_window_id.clone(),
        window.seat_index,
        ActionType::Call,
        None,
    );
    // (2) Stale/unknown window: the real actor submits with a bogus window id.
    let _ = actor.submit_action(
        "not-a-real-window".to_string(),
        window.seat_index,
        ActionType::Call,
        None,
    );

    // Give the host time to process and reject both. The open window must be
    // unchanged — neither bad submission advanced authoritative state.
    thread::sleep(Duration::from_millis(400));'''
new = '''    wait_for_client_command_connection(actor);
    wait_for_client_command_connection(offender);
    let public_event_count_before = host.public_events().expect("public event log").len();

    // (1) Out-of-turn: the non-acting client submits for the open window. The host
    // attributes the action to the sender's own id (it never trusts a claimed
    // player), so the controller rejects it — it is not that player's turn.
    offender
        .submit_action(
            window.action_window_id.clone(),
            window.seat_index,
            ActionType::Call,
            None,
        )
        .expect("out-of-turn request is written");
    let out_of_turn_error = wait_for_safe_error(offender);
    assert!(
        out_of_turn_error.contains("does not own the action window"),
        "wrong-player rejection should remain explicit; got: {out_of_turn_error}"
    );

    // (2) Stale/unknown window: the real actor submits with a bogus window id.
    actor
        .submit_action(
            "not-a-real-window".to_string(),
            window.seat_index,
            ActionType::Call,
            None,
        )
        .expect("stale-window request is written");
    let stale_window_error = wait_for_safe_error(actor);
    assert!(
        stale_window_error.contains("stale action window"),
        "stale-window rejection should remain explicit; got: {stale_window_error}"
    );

    // (3) Pick a raise-style action that is legal for this window, then submit an
    // amount below the minimum. This proves sizing validation rather than merely
    // exercising the generic illegal-action branch.
    let invalid_sized_action = if window.legal_actions.contains(&ActionType::Raise) {
        ActionType::Raise
    } else if window.legal_actions.contains(&ActionType::Bet) {
        ActionType::Bet
    } else {
        panic!("test action window must permit bet or raise sizing validation")
    };
    actor
        .submit_action(
            window.action_window_id.clone(),
            window.seat_index,
            invalid_sized_action,
            Some(0),
        )
        .expect("invalid-size raise request is written");
    let invalid_raise_error = wait_for_safe_error(actor);
    assert!(
        invalid_raise_error.contains("minimum full raise sizing"),
        "invalid raise sizing rejection should remain explicit; got: {invalid_raise_error}"
    );

    // Give the host a short scheduling window. None of the rejected submissions
    // may create a publishable transition or advance the authoritative window.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        host.public_events().expect("public event log after rejections").len(),
        public_event_count_before,
        "no-state-change rejections must publish zero runtime events"
    );'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("expected adversarial remote-action block was not found")

path.write_text(text, encoding="utf-8")
