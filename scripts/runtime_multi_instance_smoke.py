#!/usr/bin/env python3
"""Validate three real release instances through independent tauri-driver ports."""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any, Callable

from runtime_webdriver_smoke import (
    WebDriverClient,
    assert_source_contains,
    wait_for,
    wait_for_route,
    xpath_clickable_text,
)


def invoke(client: WebDriverClient, command: str, payload: dict[str, Any] | None = None) -> Any:
    script = """
const done = arguments[arguments.length - 1];
const command = arguments[0];
const payload = arguments[1] || {};
if (!window.__TAURI_INTERNALS__ || typeof window.__TAURI_INTERNALS__.invoke !== 'function') {
  done({ ok: false, error: 'Tauri invoke bridge is unavailable.' });
  return;
}
window.__TAURI_INTERNALS__.invoke(command, payload)
  .then((value) => done({ ok: true, value }))
  .catch((error) => done({ ok: false, error: String(error) }));
"""
    result = client.session_request(
        "POST",
        "/execute/async",
        {"script": script, "args": [command, payload or {}]},
    )
    if not isinstance(result, dict) or result.get("ok") is not True:
        raise AssertionError(
            f"Tauri command {command!r} failed: "
            f"{result.get('error') if isinstance(result, dict) else result!r}"
        )
    return result.get("value")


def set_labeled_control(client: WebDriverClient, label_text: str, value: str) -> None:
    result = client.execute(
        """
const labelText = arguments[0];
const value = String(arguments[1]);
const label = Array.from(document.querySelectorAll('label')).find((candidate) =>
  (candidate.textContent || '').trim().startsWith(labelText)
);
if (!label) return { ok: false, error: `Label not found: ${labelText}` };
const control = label.querySelector('input, select, textarea');
if (!control) return { ok: false, error: `Control not found: ${labelText}` };
let prototype;
if (control instanceof HTMLSelectElement) prototype = HTMLSelectElement.prototype;
else if (control instanceof HTMLTextAreaElement) prototype = HTMLTextAreaElement.prototype;
else prototype = HTMLInputElement.prototype;
const setter = Object.getOwnPropertyDescriptor(prototype, 'value')?.set;
if (!setter) return { ok: false, error: `Value setter unavailable: ${labelText}` };
setter.call(control, value);
control.dispatchEvent(new Event('input', { bubbles: true }));
control.dispatchEvent(new Event('change', { bubbles: true }));
return { ok: true, value: control.value };
""",
        [label_text, value],
    )
    if not isinstance(result, dict) or result.get("ok") is not True:
        raise AssertionError(f"Could not set {label_text!r}: {result!r}")


def click_text(client: WebDriverClient, text: str) -> None:
    client.click(client.find_xpath(xpath_clickable_text(text)))


def click_first_enabled_text(client: WebDriverClient, text: str) -> None:
    quoted = json.dumps(text)
    xpath = (
        f"(//*[normalize-space(.)={quoted}]"
        "/ancestor-or-self::button[not(@disabled)][1])[1]"
    )
    client.click(client.find_xpath(xpath))


def wait_for_source(client: WebDriverClient, text: str, timeout: float = 30.0) -> None:
    wait_for(f"source text {text!r}", lambda: text in client.source(), timeout=timeout)


def wait_for_command(
    client: WebDriverClient,
    command: str,
    predicate: Callable[[Any], bool],
    *,
    timeout: float = 30.0,
) -> Any:
    return wait_for(
        f"Tauri command {command}",
        lambda: (value := invoke(client, command)) if predicate(value) else None,
        timeout=timeout,
    )


def local_participant(status: dict[str, Any], player_id: str) -> dict[str, Any] | None:
    participants = status.get("participants")
    if not isinstance(participants, list):
        return None
    for participant in participants:
        if isinstance(participant, dict) and participant.get("playerId") == player_id:
            return participant
    return None


def verify_private_projection(view: dict[str, Any], label: str) -> None:
    seats = view.get("seats")
    if not isinstance(seats, list):
        raise AssertionError(f"{label} table projection has no seats")
    local = [seat for seat in seats if isinstance(seat, dict) and seat.get("isLocal")]
    remote = [seat for seat in seats if isinstance(seat, dict) and not seat.get("isLocal")]
    if len(local) != 1:
        raise AssertionError(f"{label} projection expected one local seat, got {len(local)}")
    if len(local[0].get("holeCards") or []) != 2:
        raise AssertionError(f"{label} local player did not receive exactly two hole cards")
    leaked = [seat for seat in remote if seat.get("holeCards")]
    if leaked:
        raise AssertionError(f"{label} projection leaked remote private cards: {leaked}")


def capture(client: WebDriverClient, directory: Path, name: str) -> None:
    (directory / f"{name}.png").write_bytes(client.screenshot())
    (directory / f"{name}.html").write_text(client.source(), encoding="utf-8")


def run(
    application: Path,
    host_url: str,
    client_url: str,
    conflict_url: str,
    evidence_dir: Path,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    host = WebDriverClient(host_url)
    client = WebDriverClient(client_url)
    conflict = WebDriverClient(conflict_url)
    steps: list[dict[str, str]] = []

    def record(name: str, result: str = "PASS") -> None:
        steps.append({"name": name, "result": result})
        print(f"[{result}] {name}", flush=True)

    try:
        for label, driver in (("host", host), ("client", client), ("conflict host", conflict)):
            driver.wait_until_ready()
            driver.start_session(application)
            wait_for_route(driver, "/", "Choose a table")
            record(f"{label} release instance launched")

        host_bootstrap = invoke(host, "get_bootstrap_state")
        client_bootstrap = invoke(client, "get_bootstrap_state")
        conflict_bootstrap = invoke(conflict, "get_bootstrap_state")
        bootstraps = [host_bootstrap, client_bootstrap, conflict_bootstrap]
        instance_ids = [str(item.get("instanceId")) for item in bootstraps]
        profile_directories = [str(item.get("profileDirectory")) for item in bootstraps]
        if len(set(instance_ids)) != 3:
            raise AssertionError(f"Instance IDs are not isolated: {instance_ids}")
        if len(set(profile_directories)) != 3:
            raise AssertionError(
                f"Profile directories are not isolated: {profile_directories}"
            )
        record("three instance IDs and profile directories are distinct")

        client.navigate("/host")
        wait_for_route(client, "/host", "Host Tournament Setup")
        client_default_tournament = f"Desktop Sit 'n Go {client_bootstrap['instanceLabel']}"
        wait_for_source(client, client_default_tournament)
        record("client host draft is namespaced independently before joining")
        client.navigate("/")
        wait_for_route(client, "/", "Choose a table")

        host.navigate("/host")
        wait_for_route(host, "/host", "Host Tournament Setup")
        set_labeled_control(host, "Max players", "2")
        click_text(host, "Start hosting")
        host_status = wait_for_command(
            host,
            "get_host_session_status",
            lambda value: isinstance(value, dict) and value.get("hostPort") == 43818,
            timeout=45.0,
        )
        invite = str(host_status.get("invite") or "")
        if not invite.startswith("pkr1_"):
            raise AssertionError("Host did not publish a compact pkr1_ invite")
        wait_for_source(host, "Live on")
        record("host started a real TCP session and produced a pkr1_ invite")

        click_text(host, "Continue to lobby")
        wait_for_route(host, "/lobby", "Lobby")

        client.navigate("/join")
        wait_for_route(client, "/join", "Join Tournament")
        invite_input = client.find_css("textarea")
        client.clear(invite_input)
        client.type_text(invite_input, invite)
        click_text(client, "Check invite")
        wait_for_source(client, "Invite looks good", timeout=30.0)
        click_text(client, "Continue to lobby")
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
        record("client joined the real host session over TCP")

        click_first_enabled_text(host, "Take seat")
        host_status = wait_for_command(
            host,
            "get_host_session_status",
            lambda value: isinstance(value, dict)
            and (participant := local_participant(value, "local-player")) is not None
            and participant.get("seatIndex") is not None,
        )
        host_seat = local_participant(host_status, "local-player")["seatIndex"]

        wait_for(
            "client sees host seat claim",
            lambda: f"Seat {int(host_seat) + 1}" in client.source()
            and "Take seat" in client.source(),
            timeout=30.0,
        )
        click_first_enabled_text(client, "Take seat")
        wait_for_source(client, "lobby-seat-local")
        record("host and client claimed different authoritative seats")

        click_text(client, "I'm ready")
        wait_for_source(client, "You: Ready")
        click_text(host, "I'm ready")
        wait_for_source(host, "You: Ready")
        wait_for(
            "host start tournament control",
            lambda: "Ready to deal" in host.source()
            and "Start tournament" in host.source(),
            timeout=30.0,
        )
        record("ready state propagated to both release instances")

        click_first_enabled_text(host, "Start tournament")
        wait_for_route(host, "/table", "Main Table")
        wait_for_route(client, "/table", "Main Table")
        record("host started the tournament and both instances reached Main Table")

        host_view = wait_for_command(
            host,
            "get_table_view",
            lambda value: isinstance(value, dict) and value.get("currentHandNumber") == 1,
            timeout=45.0,
        )
        client_view = wait_for_command(
            client,
            "get_table_view",
            lambda value: isinstance(value, dict) and value.get("currentHandNumber") == 1,
            timeout=45.0,
        )
        # get_table_view requires the camelCase enum argument even though status commands do not.
        # Retry with the explicit payload if a backend version rejected the empty payload.
        if host_view is None or client_view is None:
            raise AssertionError("Table projections were unavailable")

        verify_private_projection(host_view, "host")
        verify_private_projection(client_view, "client")
        public_fields = (
            "tableId",
            "currentHandNumber",
            "streetLabel",
            "potTotal",
            "boardCards",
        )
        mismatches = {
            field: (host_view.get(field), client_view.get(field))
            for field in public_fields
            if host_view.get(field) != client_view.get(field)
        }
        if mismatches:
            raise AssertionError(f"Public table projections diverged: {mismatches}")
        record("private cards are isolated and public table state is synchronized")

        conflict.navigate("/host")
        wait_for_route(conflict, "/host", "Host Tournament Setup")
        set_labeled_control(conflict, "Max players", "2")
        click_text(conflict, "Start hosting")
        error_element = wait_for(
            "same-port bind error",
            lambda: conflict.find_css(".inline-banner.error"),
            timeout=30.0,
        )
        bind_error = conflict.element_text(error_element).strip()
        if not bind_error:
            raise AssertionError("Port conflict produced an empty error")
        if invoke(conflict, "get_host_session_status") is not None:
            raise AssertionError("Conflicting host incorrectly reported a live session")
        record(f"same-port host failed explicitly: {bind_error}")

        set_labeled_control(conflict, "Host port", "43819")
        click_text(conflict, "Start hosting")
        conflict_status = wait_for_command(
            conflict,
            "get_host_session_status",
            lambda value: isinstance(value, dict) and value.get("hostPort") == 43819,
            timeout=45.0,
        )
        if conflict_status.get("hostPort") != 43819:
            raise AssertionError("Conflict host did not recover on port 43819")
        record("conflicting host recovered successfully on port 43819")

        capture(host, evidence_dir, "host-main-table")
        capture(client, evidence_dir, "client-main-table")
        capture(conflict, evidence_dir, "port-conflict-recovery")

        return {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "hostInstanceId": host_bootstrap.get("instanceId"),
            "clientInstanceId": client_bootstrap.get("instanceId"),
            "conflictInstanceId": conflict_bootstrap.get("instanceId"),
            "profileDirectories": profile_directories,
            "tableId": host_status.get("tableId"),
            "hostPort": host_status.get("hostPort"),
            "recoveryPort": conflict_status.get("hostPort"),
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001 - retain cross-instance evidence
        steps.append({"name": "multi-instance runtime smoke", "result": "FAIL"})
        failure: dict[str, Any] = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "error": f"{type(error).__name__}: {error}",
            "steps": steps,
        }
        for name, driver in (("host", host), ("client", client), ("conflict", conflict)):
            try:
                if driver.session_id is not None:
                    capture(driver, evidence_dir, f"failure-{name}")
            except Exception as evidence_error:  # noqa: BLE001
                failure[f"{name}EvidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        for driver in (conflict, client, host):
            driver.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--host-driver-url", default="http://127.0.0.1:4444")
    parser.add_argument("--client-driver-url", default="http://127.0.0.1:4445")
    parser.add_argument("--conflict-driver-url", default="http://127.0.0.1:4446")
    parser.add_argument(
        "--evidence-dir", type=Path, default=Path("runtime-validation-evidence")
    )
    args = parser.parse_args()

    application = args.application.resolve()
    result_path = args.evidence_dir / "release-multi-instance-result.json"
    try:
        result = run(
            application,
            args.host_driver_url,
            args.client_driver_url,
            args.conflict_driver_url,
            args.evidence_dir,
        )
        result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return 0
    except Exception as error:  # noqa: BLE001
        args.evidence_dir.mkdir(parents=True, exist_ok=True)
        result_path.write_text(f"{error}\n", encoding="utf-8")
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
