#!/usr/bin/env python3
"""Apply the timeout/action transaction repair and its focused regressions."""

from __future__ import annotations

import re
from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    if new in text:
        return
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one source block, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_regex_once(
    path: Path, pattern: re.Pattern[str], replacement: str, label: str
) -> None:
    text = path.read_text(encoding="utf-8")
    updated, count = pattern.subn(replacement, text, count=1)
    if count != 1:
        raise SystemExit(f"{label}: expected one regex match, replaced {count}")
    path.write_text(updated, encoding="utf-8")


def patch_controller_clone() -> None:
    path = Path("crates/poker-core/src/tournament/mod.rs")
    replace_once(
        path,
        "pub struct TournamentController {\n",
        "#[derive(Clone)]\npub struct TournamentController {\n",
        "TournamentController Clone derive",
    )


def patch_controller_submit_action() -> None:
    path = Path("crates/poker-core/src/tournament/controller_core.rs")
    old = '''    pub fn submit_action(
        &mut self,
        request: ActionRequest,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        let current_window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("stale action window rejected"))?;

        if now_ms >= current_window.deadline_epoch_ms {
            self.commit_timeout(now_ms)?;
            return Err(TournamentError::new("stale action window rejected"));
        }

        if request.player_id != current_window.player_id {
            return Err(TournamentError::new(
                "action rejected: player does not own the action window",
            ));
        }

        if request.action_window_id != current_window.action_window_id {
            return Err(TournamentError::new("stale action window rejected"));
        }

        self.apply_action(
            request.player_id,
            request.action_type,
            request.raise_to_amount,
            now_ms,
        )?;
        self.validate_state()?;
        Ok(())
    }
'''
    new = '''    pub fn submit_action(
        &mut self,
        request: ActionRequest,
        now_ms: u64,
    ) -> Result<(), TournamentError> {
        let rollback_state = self.clone();
        let current_window = self
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .cloned()
            .ok_or_else(|| TournamentError::new("stale action window rejected"))?;

        if now_ms >= current_window.deadline_epoch_ms {
            if let Err(error) = self.commit_timeout(now_ms) {
                *self = rollback_state;
                return Err(error);
            }
            if let Err(error) = self.validate_state() {
                *self = rollback_state;
                return Err(error);
            }
            return Err(TournamentError::new("stale action window rejected"));
        }

        if request.player_id != current_window.player_id {
            return Err(TournamentError::new(
                "action rejected: player does not own the action window",
            ));
        }

        if request.action_window_id != current_window.action_window_id {
            return Err(TournamentError::new("stale action window rejected"));
        }

        if let Err(error) = self.apply_action(
            request.player_id,
            request.action_type,
            request.raise_to_amount,
            now_ms,
        ) {
            *self = rollback_state;
            return Err(error);
        }
        if let Err(error) = self.validate_state() {
            *self = rollback_state;
            return Err(error);
        }
        Ok(())
    }
'''
    replace_once(path, old, new, "TournamentController::submit_action")


def insert_core_regression() -> None:
    path = Path("crates/poker-core/src/tournament/tests/core.rs")
    text = path.read_text(encoding="utf-8")
    test_name = "submit_action_invalid_raise_rolls_back_controller_state"
    if test_name in text:
        return
    marker = "// T3.3 — submit_action rejects stale window by ID after action was already consumed\n"
    test = '''#[test]
fn submit_action_invalid_raise_rolls_back_controller_state() {
    let mut controller = started_two_player_controller();
    let before = controller.state().clone();
    let window = action_window(&controller);

    let error = controller
        .submit_action(
            ActionRequest {
                player_id: window.player_id,
                action_window_id: window.action_window_id,
                action_type: ActionType::Raise,
                raise_to_amount: Some(1_500),
            },
            0,
        )
        .expect_err("out-of-bounds raise must be rejected");

    assert!(error.to_string().contains("exceeds remaining stack"));
    assert_eq!(
        controller.state(),
        &before,
        "a rejected action must leave the complete controller state unchanged"
    );
}

'''
    if text.count(marker) != 1:
        raise SystemExit("core regression insertion marker is missing or duplicated")
    path.write_text(text.replace(marker, test + marker, 1), encoding="utf-8")


def patch_host_submit_action() -> None:
    path = Path("src-tauri/src/networking/runtime/host.rs")
    text = path.read_text(encoding="utf-8")
    if "let (next_state, action_result)" in text:
        return
    pattern = re.compile(
        r"    pub fn submit_action\(\n"
        r"        &self,\n"
        r"        player_id: &str,\n"
        r"        action_window_id: String,\n"
        r"        action_type: crate::domain::ActionType,\n"
        r"        raise_to_amount: Option<u32>,\n"
        r"    \) -> Result<\(\), NetworkingError> \{\n"
        r".*?"
        r"\n    \}\n\n"
        r"    /// Attempt to broadcast updated snapshots after a successful lobby mutation\.",
        re.DOTALL,
    )
    replacement = '''    pub fn submit_action(
        &self,
        player_id: &str,
        action_window_id: String,
        action_type: crate::domain::ActionType,
        raise_to_amount: Option<u32>,
    ) -> Result<(), NetworkingError> {
        let before_state = self.authoritative_state()?;
        let (next_state, action_result) = {
            let mut runtime = self
                .tournament_runtime
                .lock()
                .map_err(|_| NetworkingError::new("tournament runtime lock poisoned"))?;
            let controller = runtime
                .as_mut()
                .ok_or_else(|| NetworkingError::new("live tournament runtime is unavailable"))?;

            let action_result = controller
                .submit_action(
                    ActionRequest {
                        player_id: player_id.to_string(),
                        action_window_id,
                        action_type,
                        raise_to_amount,
                    },
                    now_epoch_ms(),
                )
                .map_err(|error| NetworkingError::new(error.to_string()));
            (controller.state().clone(), action_result)
        };

        let publish_result = if next_state != before_state {
            self.authoritative_state
                .lock()
                .map_err(|_| NetworkingError::new("authoritative state lock poisoned"))
                .map(|mut authoritative_state| {
                    let mut merged = next_state;
                    merge_networking_state(&authoritative_state, &mut merged);
                    *authoritative_state = merged;
                })?;
            let after_state = self.authoritative_state()?;
            publish_runtime_transition(
                &self.join_payload,
                &self.authoritative_state,
                &before_state,
                &after_state,
                &self.clients,
                &self.server_sequence,
                &self.host_signing_keys,
                &self.host_encryption_keys,
                &self.public_events,
            )
        } else {
            Ok(())
        };

        match (action_result, publish_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(action_error), Ok(())) => Err(action_error),
            (Ok(()), Err(publish_error)) => Err(publish_error),
            (Err(action_error), Err(publish_error)) => Err(NetworkingError::new(format!(
                "{action_error}; additionally failed to publish committed runtime state: {publish_error}"
            ))),
        }
    }

    /// Attempt to broadcast updated snapshots after a successful lobby mutation.'''
    replace_regex_once(path, pattern, replacement, "HostServer::submit_action")


def insert_host_regression() -> None:
    path = Path("src-tauri/src/networking/runtime/tests/tournament.rs")
    text = path.read_text(encoding="utf-8")
    test_name = "stale_submission_that_commits_timeout_synchronizes_authoritative_state"
    if test_name in text:
        return
    marker = "// P0.4 — NPC runner reports failure (returns false) when submit_action is rejected.\n"
    test = '''#[test]
fn stale_submission_that_commits_timeout_synchronizes_authoritative_state() {
    let provider = DefaultCryptoProvider;
    let mut initial_state = sample_tournament_state("table-timeout-sync", 89);
    initial_state.config.max_players = 2;
    initial_state.config.turn_timer_seconds = 1;
    let host = bind_test_host_with_state(
        &provider,
        "table-timeout-sync",
        89,
        initial_state,
    );

    for (player_id, display_name, seat_index) in
        [("player-a", "Alice", 0_u8), ("player-b", "Bob", 1_u8)]
    {
        host.register_npc_participant(player_id, display_name)
            .expect("participant registers");
        host.claim_seat(player_id, seat_index)
            .expect("participant claims seat");
        host.set_ready_state(player_id, true)
            .expect("participant becomes ready");
    }

    host.start_tournament().expect("tournament starts");
    let original_window = host
        .authoritative_state()
        .expect("running state")
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.clone())
        .expect("initial action window");

    host.stop_signal.store(true, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(1_100));

    let error = host
        .submit_action(
            &original_window.player_id,
            original_window.action_window_id.clone(),
            ActionType::Fold,
            None,
        )
        .expect_err("expired action is rejected after committing the timeout");
    assert!(error.to_string().contains("stale action window rejected"));

    let synchronized = host
        .authoritative_state()
        .expect("authoritative state after timeout rejection");
    assert_eq!(
        synchronized.hand_results.len(),
        1,
        "the timeout fold must settle the heads-up hand in authoritative state"
    );
    assert_ne!(
        synchronized
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| window.action_window_id.as_str()),
        Some(original_window.action_window_id.as_str()),
        "the expired action window must not remain visible after rejection"
    );
}

'''
    if text.count(marker) != 1:
        raise SystemExit("host regression insertion marker is missing or duplicated")
    path.write_text(text.replace(marker, test + marker, 1), encoding="utf-8")


def main() -> None:
    patch_controller_clone()
    patch_controller_submit_action()
    insert_core_regression()
    patch_host_submit_action()
    insert_host_regression()


if __name__ == "__main__":
    main()
