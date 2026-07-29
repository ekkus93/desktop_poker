#!/usr/bin/env python3
"""Drive a complete two-player release tournament through real Tauri/WebKit UIs."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Callable

from runtime_multi_instance_smoke import (
    WebDriverClient,
    assert_shared_participant_seats,
    capture,
    click_first_enabled_text,
    click_text,
    invoke,
    participant_by_id,
    set_labeled_control,
    wait_for_command,
    wait_for_source,
)
from runtime_webdriver_smoke import WebDriverError, wait_for, wait_for_route

VIEW_PAYLOAD = {"viewerMode": "local"}


def table_view(client: WebDriverClient) -> dict[str, Any]:
    value = invoke(client, "get_table_view", VIEW_PAYLOAD)
    if not isinstance(value, dict):
        raise AssertionError(f"get_table_view returned {value!r}")
    return value


def wait_for_views(
    host: WebDriverClient,
    client: WebDriverClient,
    description: str,
    predicate: Callable[[dict[str, Any], dict[str, Any]], bool],
    *,
    timeout: float = 45.0,
) -> tuple[dict[str, Any], dict[str, Any]]:
    def probe() -> tuple[dict[str, Any], dict[str, Any]] | None:
        host_view = table_view(host)
        client_view = table_view(client)
        return (host_view, client_view) if predicate(host_view, client_view) else None

    return wait_for(description, probe, timeout=timeout)


def history(view: dict[str, Any]) -> list[dict[str, Any]]:
    value = view.get("handHistory")
    if not isinstance(value, list):
        raise AssertionError("Table view has no handHistory list")
    return [entry for entry in value if isinstance(entry, dict)]


def assert_history_sync(
    host_view: dict[str, Any], client_view: dict[str, Any]
) -> list[dict[str, Any]]:
    host_history = history(host_view)
    client_history = history(client_view)
    if host_history != client_history:
        raise AssertionError(
            "Host and client hand histories diverged: "
            f"host={host_history!r}, client={client_history!r}"
        )
    numbers = [entry.get("handNumber") for entry in host_history]
    if len(numbers) != len(set(numbers)):
        raise AssertionError(f"Duplicate hand history numbers: {numbers}")
    return host_history


def normalized_standings(view: dict[str, Any]) -> list[dict[str, Any]]:
    standings = view.get("standings")
    if not isinstance(standings, list):
        raise AssertionError("Table view has no standings list")
    normalized: list[dict[str, Any]] = []
    for entry in standings:
        if not isinstance(entry, dict):
            continue
        normalized.append(
            {key: value for key, value in entry.items() if key not in {"isLocal"}}
        )
    return normalized


def assert_public_sync(
    host_view: dict[str, Any], client_view: dict[str, Any]
) -> None:
    fields = (
        "tableId",
        "tournamentPhase",
        "currentHandNumber",
        "streetLabel",
        "potTotal",
        "boardCards",
        "actionOwnerLabel",
    )
    mismatches = {
        field: (host_view.get(field), client_view.get(field))
        for field in fields
        if host_view.get(field) != client_view.get(field)
    }
    if mismatches:
        raise AssertionError(f"Public table state diverged: {mismatches}")


def sole_actor(
    host: WebDriverClient,
    client: WebDriverClient,
    host_view: dict[str, Any],
    client_view: dict[str, Any],
) -> tuple[str, WebDriverClient, dict[str, Any], WebDriverClient, dict[str, Any]]:
    actors = []
    if host_view.get("actionTray") is not None:
        actors.append(("host", host, host_view, client, client_view))
    if client_view.get("actionTray") is not None:
        actors.append(("client", client, client_view, host, host_view))
    if len(actors) != 1:
        raise AssertionError(
            "Expected exactly one local action tray, got "
            f"host={host_view.get('actionTray') is not None}, "
            f"client={client_view.get('actionTray') is not None}"
        )
    name, actor, actor_view, other, other_view = actors[0]
    wait_for_source(actor, "Play this spot")
    wait_for(
        "non-actor UI to omit the action tray",
        lambda: "Play this spot" not in other.source(),
        timeout=20.0,
    )
    return name, actor, actor_view, other, other_view


def invoke_expect_error(
    client: WebDriverClient,
    command: str,
    payload: dict[str, Any],
) -> str:
    try:
        invoke(client, command, payload)
    except (AssertionError, WebDriverError) as error:
        return str(error)
    raise AssertionError(f"{command} unexpectedly succeeded for invalid input")


def click_all_in_and_confirm(client: WebDriverClient) -> None:
    click_first_enabled_text(client, "All-in")
    wait_for_source(client, "Confirm all-in")
    click_first_enabled_text(client, "Confirm")


def setup_tournament(
    host: WebDriverClient,
    client: WebDriverClient,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    for driver in (host, client):
        driver.wait_until_ready()
        driver.start_session(APPLICATION)
        wait_for_route(driver, "/", "Choose a table")

    host_bootstrap = invoke(host, "get_bootstrap_state")
    client_bootstrap = invoke(client, "get_bootstrap_state")
    if host_bootstrap.get("profileDirectory") == client_bootstrap.get(
        "profileDirectory"
    ):
        raise AssertionError("Host and client profile directories are not isolated")

    host.navigate("/host")
    wait_for_route(host, "/host", "Host Tournament Setup")
    set_labeled_control(host, "Max players", "2")
    set_labeled_control(host, "Starting stack", "1000")
    set_labeled_control(host, "Turn timer", "60")
    click_text(host, "Start hosting")
    host_status = wait_for_command(
        host,
        "get_host_session_status",
        lambda value: isinstance(value, dict) and value.get("hostPort") == 43818,
        timeout=45.0,
    )
    invite = str(host_status.get("invite") or "")
    if not invite.startswith("pkr1_"):
        raise AssertionError("Host did not produce a pkr1_ invite")
    click_first_enabled_text(host, "Continue to lobby")
    wait_for_route(host, "/lobby", "Lobby")

    client.navigate("/join")
    wait_for_route(client, "/join", "Join Tournament")
    invite_input = client.find_css("textarea")
    client.clear(invite_input)
    client.type_text(invite_input, invite)
    click_text(client, "Check invite")
    wait_for_source(client, "Invite decoded")
    click_first_enabled_text(client, "Continue to lobby")
    wait_for_route(client, "/lobby", "Lobby")

    host_status = wait_for_command(
        host,
        "get_host_session_status",
        lambda value: isinstance(value, dict)
        and isinstance(value.get("participants"), list)
        and len(value["participants"]) == 2,
        timeout=45.0,
    )
    client_status = wait_for_command(
        client,
        "get_client_session_status",
        lambda value: isinstance(value, dict)
        and value.get("tableId") == host_status.get("tableId"),
        timeout=45.0,
    )
    assert_shared_participant_seats(host_status, client_status)

    client_player_id = str(client_status.get("localPlayerId"))
    client_local = participant_by_id(client_status, client_player_id)
    if client_local is None:
        raise AssertionError("Client local participant is missing")
    if client_local.get("seatIndex") is None:
        wait_for_source(client, "Take seat")
        click_first_enabled_text(client, "Take seat")
        client_status = wait_for_command(
            client,
            "get_client_session_status",
            lambda value: isinstance(value, dict)
            and (participant := participant_by_id(value, client_player_id)) is not None
            and participant.get("seatIndex") is not None,
            timeout=45.0,
        )

    host_status = wait_for_command(
        host,
        "get_host_session_status",
        lambda value: isinstance(value, dict)
        and (participant := participant_by_id(value, client_player_id)) is not None
        and participant.get("seatIndex") is not None,
        timeout=45.0,
    )
    assert_shared_participant_seats(host_status, client_status)

    click_text(client, "I'm ready")
    wait_for_source(client, "You: Ready")
    click_text(host, "I'm ready")
    wait_for_source(host, "You: Ready")
    wait_for_command(
        host,
        "get_host_session_status",
        lambda value: isinstance(value, dict) and value.get("phase") == "readyCheck",
        timeout=30.0,
    )

    click_first_enabled_text(host, "Start tournament")
    wait_for_route(host, "/table", "Main Table")
    wait_for_route(client, "/table", "Main Table")
    host_view, client_view = wait_for_views(
        host,
        client,
        "initial running hand",
        lambda left, right: left.get("currentHandNumber") == 1
        and right.get("currentHandNumber") == 1,
        timeout=45.0,
    )
    return host_bootstrap, client_bootstrap, host_view, client_view


def validate_gameplay(
    host: WebDriverClient,
    client: WebDriverClient,
    steps: list[dict[str, str]],
) -> tuple[dict[str, Any], dict[str, Any]]:
    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    host_view, client_view = wait_for_views(
        host,
        client,
        "one authoritative initial actor",
        lambda left, right: (
            int(left.get("actionTray") is not None)
            + int(right.get("actionTray") is not None)
        )
        == 1,
    )
    assert_public_sync(host_view, client_view)
    actor_name, actor, actor_view, _, _ = sole_actor(
        host, client, host_view, client_view
    )
    record(f"exactly one action tray is visible; initial actor is {actor_name}")

    tray = actor_view.get("actionTray")
    if not isinstance(tray, dict):
        raise AssertionError("Initial actor action tray is missing")
    before_history = len(history(actor_view))
    before_deadline = tray.get("deadlineEpochMs")
    illegal_raise_to = int(tray.get("maxRaiseTo") or tray.get("currentBet") or 0) + 10_000
    rejection = invoke_expect_error(
        actor,
        "submit_table_action",
        {
            "viewerMode": "local",
            "actionKind": "betOrRaise",
            "raiseToAmount": illegal_raise_to,
        },
    )
    post_reject_actor = table_view(actor)
    if len(history(post_reject_actor)) != before_history:
        raise AssertionError("Rejected raise advanced hand history")
    if (
        isinstance(post_reject_actor.get("actionTray"), dict)
        and post_reject_actor["actionTray"].get("deadlineEpochMs") != before_deadline
    ):
        raise AssertionError("Rejected raise replaced the action window")
    record(f"out-of-bounds raise was rejected without advancing state: {rejection}")

    if tray.get("minRaiseTo") is not None and tray.get("maxRaiseTo") is not None:
        click_first_enabled_text(actor, "Min")
        expected = str(tray["minRaiseTo"])
        wait_for(
            "quick-size Min to update the raise slider",
            lambda: str(
                actor.execute(
                    "return document.querySelector('#raise-slider')?.value ?? null;"
                )
            )
            == expected,
        )
        record("quick-size Min updates the legal raise amount without submitting")

    click_first_enabled_text(actor, "Fold")
    host_view, client_view = wait_for_views(
        host,
        client,
        "first folded hand to settle on both instances",
        lambda left, right: len(history(left)) >= before_history + 1
        and history(left) == history(right),
        timeout=45.0,
    )
    first_history = assert_history_sync(host_view, client_view)
    assert_public_sync(host_view, client_view)
    record(
        f"Fold completed hand {first_history[0].get('handNumber')} with synchronized duplicate-free history"
    )

    # A called heads-up all-in does not guarantee elimination: the shorter
    # stack can win or the board can split. Keep a finite safety bound, but
    # make the probabilistic completion check resistant to rare win streaks.
    max_showdowns = 20
    for attempt in range(1, max_showdowns + 1):
        host_view, client_view = wait_for_views(
            host,
            client,
            "next action window or tournament completion",
            lambda left, right: left.get("tournamentPhase") == "complete"
            or (
                int(left.get("actionTray") is not None)
                + int(right.get("actionTray") is not None)
            )
            == 1,
            timeout=45.0,
        )
        if host_view.get("tournamentPhase") == "complete":
            break

        history_before = len(assert_history_sync(host_view, client_view))
        actor_name, actor, actor_view, _, _ = sole_actor(
            host, client, host_view, client_view
        )
        tray = actor_view.get("actionTray")
        if not isinstance(tray, dict) or "allIn" not in (
            tray.get("legalActions") or []
        ):
            raise AssertionError(f"{actor_name} has no legal all-in action: {tray!r}")

        click_all_in_and_confirm(actor)

        def response_or_settlement() -> (
            tuple[str, dict[str, Any], dict[str, Any]] | None
        ):
            left = table_view(host)
            right = table_view(client)
            if (
                left.get("tournamentPhase") == "complete"
                or len(history(left)) > history_before
            ):
                return ("settled", left, right)
            left_actor = left.get("actionTray") is not None
            right_actor = right.get("actionTray") is not None
            if left_actor != right_actor:
                next_name = "host" if left_actor else "client"
                if next_name != actor_name:
                    return ("response", left, right)
            return None

        state, host_view, client_view = wait_for(
            "opponent response or all-in settlement",
            response_or_settlement,
            timeout=45.0,
        )

        if state == "response":
            response_name, responder, response_view, _, _ = sole_actor(
                host, client, host_view, client_view
            )
            response_tray = response_view.get("actionTray")
            if not isinstance(response_tray, dict) or "checkOrCall" not in (
                response_tray.get("legalActions") or []
            ):
                raise AssertionError(
                    f"{response_name} cannot call the all-in: {response_tray!r}"
                )
            call_label = str(response_tray.get("checkOrCallLabel"))
            click_first_enabled_text(responder, call_label)
            host_view, client_view = wait_for_views(
                host,
                client,
                "all-in showdown settlement",
                lambda left, right: left.get("tournamentPhase") == "complete"
                or (
                    len(history(left)) > history_before
                    and history(left) == history(right)
                ),
                timeout=60.0,
            )

        settled_history = assert_history_sync(host_view, client_view)
        assert_public_sync(host_view, client_view)
        record(
            f"all-in showdown attempt {attempt} settled with {len(settled_history)} synchronized hands"
        )
        if host_view.get("tournamentPhase") == "complete":
            break
    else:
        raise AssertionError(
            f"Tournament did not complete after {max_showdowns} all-in showdowns"
        )

    if host_view.get("tournamentPhase") != "complete":
        raise AssertionError("Tournament did not reach complete phase")
    if client_view.get("tournamentPhase") != "complete":
        raise AssertionError("Client did not reach complete phase")
    if host_view.get("actionTray") is not None or client_view.get("actionTray") is not None:
        raise AssertionError("Completed tournament retained an action tray")

    final_history = assert_history_sync(host_view, client_view)
    if len(final_history) < 2:
        raise AssertionError("Expected at least a fold hand and a showdown hand")
    host_standings = normalized_standings(host_view)
    client_standings = normalized_standings(client_view)
    if host_standings != client_standings:
        raise AssertionError(
            f"Final standings diverged: host={host_standings!r}, client={client_standings!r}"
        )
    local_observers = 0
    for view in (host_view, client_view):
        local_entries = [
            entry
            for entry in view.get("standings", [])
            if isinstance(entry, dict) and entry.get("isLocal")
        ]
        if len(local_entries) != 1:
            raise AssertionError("Each completion projection must identify one local player")
        local_observers += int(bool(local_entries[0].get("isObserver")))
    if local_observers != 1:
        raise AssertionError(
            f"Expected exactly one eliminated local observer, got {local_observers}"
        )
    record("tournament completed with matching standings and one eliminated observer")

    for driver in (host, client):
        driver.navigate("/complete")
        wait_for_route(driver, "/complete", "Tournament Complete")
        wait_for_source(driver, str(host_standings[0].get("displayName")))
    record("both release instances render the same Tournament Complete winner")

    return host_view, client_view


def validate_restart_persistence(
    host: WebDriverClient,
    client: WebDriverClient,
    fresh: WebDriverClient,
    host_history: list[dict[str, Any]],
    steps: list[dict[str, str]],
) -> None:
    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    latest_summary = str(host_history[0].get("summary"))
    for driver in (host, client):
        driver.navigate("/history")
        wait_for_route(driver, "/history", "Hand History")
        wait_for_source(driver, latest_summary)

    fresh.wait_until_ready()
    fresh.start_session(APPLICATION)
    wait_for_route(fresh, "/", "Choose a table")
    fresh.navigate("/history")
    wait_for_route(fresh, "/history", "Hand History")
    wait_for_source(fresh, "No settled hands yet.")
    record("fresh third profile contains no host/client hand history")

    host.close()
    client.close()
    host.start_session(APPLICATION)
    client.start_session(APPLICATION)
    for driver in (host, client):
        driver.navigate("/")
        wait_for_route(driver, "/", "Choose a table")
        driver.navigate("/history")
        wait_for_route(driver, "/history", "Hand History")
        wait_for_source(driver, "Saved on this device.")
        wait_for_source(driver, latest_summary)
    record("host and client history restore after release-process restart")


def run(
    application: Path,
    host_url: str,
    client_url: str,
    fresh_url: str,
    evidence_dir: Path,
) -> dict[str, Any]:
    global APPLICATION
    APPLICATION = application

    evidence_dir.mkdir(parents=True, exist_ok=True)
    host = WebDriverClient(host_url)
    client = WebDriverClient(client_url)
    fresh = WebDriverClient(fresh_url)
    steps: list[dict[str, str]] = []

    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    try:
        host_bootstrap, client_bootstrap, host_view, client_view = setup_tournament(
            host, client
        )
        record("two isolated release instances completed host/join/seat/ready/start")
        assert_public_sync(host_view, client_view)
        record("initial running-hand public state is synchronized")

        host_view, client_view = validate_gameplay(host, client, steps)
        final_history = assert_history_sync(host_view, client_view)
        validate_restart_persistence(host, client, fresh, final_history, steps)

        capture(host, evidence_dir, "restarted-host-history")
        capture(client, evidence_dir, "restarted-client-history")
        capture(fresh, evidence_dir, "fresh-profile-history")

        return {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "hostInstanceId": host_bootstrap.get("instanceId"),
            "clientInstanceId": client_bootstrap.get("instanceId"),
            "hostProfileDirectory": host_bootstrap.get("profileDirectory"),
            "clientProfileDirectory": client_bootstrap.get("profileDirectory"),
            "completedHands": len(final_history),
            "finalHistory": final_history,
            "finalStandings": normalized_standings(host_view),
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001 - preserve release failure evidence
        steps.append({"name": "full-game runtime smoke", "result": "FAIL"})
        failure: dict[str, Any] = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "error": f"{type(error).__name__}: {error}",
            "steps": steps,
        }
        for name, driver in (("host", host), ("client", client), ("fresh", fresh)):
            try:
                if driver.session_id is not None:
                    capture(driver, evidence_dir, f"failure-{name}")
            except Exception as evidence_error:  # noqa: BLE001
                failure[f"{name}EvidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        for driver in (fresh, client, host):
            driver.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--host-driver-url", default="http://127.0.0.1:4544")
    parser.add_argument("--client-driver-url", default="http://127.0.0.1:4554")
    parser.add_argument("--fresh-driver-url", default="http://127.0.0.1:4564")
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=Path("runtime-gameplay-evidence"),
    )
    args = parser.parse_args()

    application = args.application.resolve()
    result_path = args.evidence_dir / "release-full-game-result.json"
    try:
        result = run(
            application,
            args.host_driver_url,
            args.client_driver_url,
            args.fresh_driver_url,
            args.evidence_dir,
        )
        result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return 0
    except Exception as error:  # noqa: BLE001
        args.evidence_dir.mkdir(parents=True, exist_ok=True)
        result_path.write_text(f"{error}\n", encoding="utf-8")
        print(error, file=sys.stderr)
        return 1


APPLICATION = Path()


if __name__ == "__main__":
    raise SystemExit(main())
