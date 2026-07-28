#!/usr/bin/env python3
"""Validate release credential storage against a real or unavailable keychain."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from runtime_multi_instance_smoke import WebDriverClient, capture, invoke
from runtime_webdriver_smoke import wait_for_route

FORBIDDEN_KEY_FILES = {"llm-provider-key.dat", "claude-api-key.txt"}


def invoke_result(
    client: WebDriverClient,
    command: str,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    result = client.session_request(
        "POST",
        "/execute/async",
        {
            "script": """
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
""",
            "args": [command, payload or {}],
        },
    )
    if not isinstance(result, dict) or not isinstance(result.get("ok"), bool):
        raise AssertionError(f"Malformed Tauri result for {command!r}: {result!r}")
    return result


def expect_error(
    client: WebDriverClient,
    command: str,
    payload: dict[str, Any] | None = None,
) -> str:
    result = invoke_result(client, command, payload)
    if result.get("ok") is True:
        raise AssertionError(f"{command!r} unexpectedly succeeded")
    error = str(result.get("error") or "").strip()
    if not error:
        raise AssertionError(f"{command!r} failed without an error message")
    return error


def assert_non_secret_config(config: Any) -> dict[str, Any]:
    if not isinstance(config, dict):
        raise AssertionError(f"provider config is not an object: {config!r}")
    serialized = json.dumps(config, sort_keys=True)
    if "apiKey" in serialized or "api_key" in serialized:
        raise AssertionError("get_llm_provider_config exposed an API-key field")
    if config.get("provider") != "anthropic":
        raise AssertionError(f"unexpected provider settings: {config!r}")
    return config


def storage_paths(bootstrap: dict[str, Any]) -> tuple[Path, Path]:
    profile_dir = Path(str(bootstrap.get("profileDirectory") or ""))
    if profile_dir.parent.name != "profiles":
        raise AssertionError(f"unexpected profile directory layout: {profile_dir}")
    return profile_dir, profile_dir.parent.parent


def scan_app_data(
    app_data_dir: Path,
    secret: str,
    *,
    require_exists: bool,
) -> list[dict[str, Any]]:
    if not app_data_dir.exists():
        if require_exists:
            raise AssertionError(f"app-data directory does not exist: {app_data_dir}")
        return []
    if not app_data_dir.is_dir():
        raise AssertionError(f"app-data path is not a directory: {app_data_dir}")

    inventory: list[dict[str, Any]] = []
    secret_bytes = secret.encode("utf-8")
    for path in sorted(app_data_dir.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(app_data_dir).as_posix()
        data = path.read_bytes()
        inventory.append({"path": relative, "size": len(data)})
        if path.name in FORBIDDEN_KEY_FILES:
            raise AssertionError(f"release app data contains forbidden plaintext key file: {relative}")
        if secret_bytes in data:
            raise AssertionError(f"release credential leaked into app-data file: {relative}")
    return inventory


def assert_settings_file(inventory: list[dict[str, Any]], expected: bool) -> None:
    paths = {str(entry.get("path")) for entry in inventory}
    present = "llm-provider.json" in paths
    if present != expected:
        raise AssertionError(
            f"llm-provider.json presence was {present}, expected {expected}; files={sorted(paths)}"
        )


def assert_log_redacted(runtime_log: Path, secret: str) -> None:
    if not runtime_log.is_file():
        raise AssertionError(f"runtime log is missing: {runtime_log}")
    text = runtime_log.read_text(encoding="utf-8", errors="replace")
    if secret in text:
        raise AssertionError("release credential leaked into tauri-driver/application log")


def start_release(client: WebDriverClient, application: Path) -> dict[str, Any]:
    client.start_session(application)
    wait_for_route(client, "/", "Choose a table")
    bootstrap = invoke(client, "get_bootstrap_state")
    if not isinstance(bootstrap, dict):
        raise AssertionError(f"get_bootstrap_state returned {bootstrap!r}")
    return bootstrap


def run_success(
    application: Path,
    driver_url: str,
    evidence_dir: Path,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    runtime_log = evidence_dir / "tauri-driver.log"
    client = WebDriverClient(driver_url)
    secret = f"dp-ci-key-{os.environ.get('GITHUB_RUN_ID', 'local')}-not-a-real-key"
    steps: list[dict[str, str]] = []

    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    try:
        client.wait_until_ready()
        first_bootstrap = start_release(client, application)
        profile_dir, app_data_dir = storage_paths(first_bootstrap)
        if first_bootstrap.get("llmApiKeyConfigured") is True:
            raise AssertionError("fresh release keychain profile was already configured")
        record("fresh release process started without a configured provider")

        invoke(client, "set_llm_api_key", {"key": secret})
        configured = invoke(client, "get_bootstrap_state")
        if configured.get("llmApiKeyConfigured") is not True:
            raise AssertionError(f"bootstrap did not report stored key: {configured!r}")
        settings = assert_non_secret_config(invoke(client, "get_llm_provider_config"))
        inventory_after_set = scan_app_data(app_data_dir, secret, require_exists=True)
        assert_settings_file(inventory_after_set, True)
        assert_log_redacted(runtime_log, secret)
        capture(client, evidence_dir, "keychain-configured")
        record("release credential was accepted while public config remained non-secret")
        record("global app-data files and runtime log contain no credential or plaintext key file")

        client.close()
        restarted_bootstrap = start_release(client, application)
        if restarted_bootstrap.get("llmApiKeyConfigured") is not True:
            raise AssertionError(
                f"release restart did not reload keychain credential: {restarted_bootstrap!r}"
            )
        restarted_settings = assert_non_secret_config(
            invoke(client, "get_llm_provider_config")
        )
        if restarted_settings != settings:
            raise AssertionError(
                f"provider settings changed after restart: {settings!r} -> {restarted_settings!r}"
            )
        scan_app_data(app_data_dir, secret, require_exists=True)
        assert_log_redacted(runtime_log, secret)
        record("release restart recovered the credential through the OS keychain")

        invoke(client, "clear_llm_api_key")
        cleared = invoke(client, "get_bootstrap_state")
        if cleared.get("llmApiKeyConfigured") is True:
            raise AssertionError(f"bootstrap still reports a configured key: {cleared!r}")
        if invoke(client, "get_llm_provider_config") is not None:
            raise AssertionError("provider config remained visible after clear")
        inventory_after_clear = scan_app_data(app_data_dir, secret, require_exists=True)
        assert_settings_file(inventory_after_clear, False)
        assert_log_redacted(runtime_log, secret)
        record("clear removed provider settings and the keychain credential")

        client.close()
        final_bootstrap = start_release(client, application)
        if final_bootstrap.get("llmApiKeyConfigured") is True:
            raise AssertionError(
                f"cleared credential reappeared after restart: {final_bootstrap!r}"
            )
        if invoke(client, "get_llm_provider_config") is not None:
            raise AssertionError("cleared provider settings reappeared after restart")
        final_inventory = scan_app_data(app_data_dir, secret, require_exists=True)
        assert_settings_file(final_inventory, False)
        assert_log_redacted(runtime_log, secret)
        capture(client, evidence_dir, "keychain-cleared-after-restart")
        record("second release restart confirmed durable keychain deletion")

        (evidence_dir / "app-data-inventory-after-set.json").write_text(
            json.dumps(inventory_after_set, indent=2) + "\n", encoding="utf-8"
        )
        (evidence_dir / "app-data-inventory-after-clear.json").write_text(
            json.dumps(inventory_after_clear, indent=2) + "\n", encoding="utf-8"
        )
        return {
            "result": "PASS",
            "mode": "success",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "instanceId": first_bootstrap.get("instanceId"),
            "profileDirectory": str(profile_dir),
            "appDataDirectory": str(app_data_dir),
            "provider": settings.get("provider"),
            "secretLength": len(secret),
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001
        steps.append({"name": "release keychain persistence", "result": "FAIL"})
        failure = {
            "result": "FAIL",
            "mode": "success",
            "error": f"{type(error).__name__}: {error}",
            "steps": steps,
        }
        try:
            if client.session_id is not None:
                capture(client, evidence_dir, "keychain-success-failure")
        except Exception as evidence_error:  # noqa: BLE001
            failure["evidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()


def run_failure(
    application: Path,
    driver_url: str,
    evidence_dir: Path,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    runtime_log = evidence_dir / "tauri-driver.log"
    client = WebDriverClient(driver_url)
    secret = f"dp-ci-key-failure-{os.environ.get('GITHUB_RUN_ID', 'local')}"
    steps: list[dict[str, str]] = []

    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    try:
        client.wait_until_ready()
        bootstrap = start_release(client, application)
        profile_dir, app_data_dir = storage_paths(bootstrap)
        error = expect_error(client, "set_llm_api_key", {"key": secret})
        lowered = error.lower()
        if "keychain" not in lowered and "secret service" not in lowered and "dbus" not in lowered:
            raise AssertionError(f"keychain failure was not explicit: {error!r}")
        after = invoke(client, "get_bootstrap_state")
        if after.get("llmApiKeyConfigured") is True:
            raise AssertionError("failed keychain write still marked provider configured")
        if invoke(client, "get_llm_provider_config") is not None:
            raise AssertionError("failed keychain write retained provider settings")
        inventory = scan_app_data(app_data_dir, secret, require_exists=False)
        assert_settings_file(inventory, False)
        assert_log_redacted(runtime_log, secret)
        capture(client, evidence_dir, "keychain-unavailable")
        record("unavailable keychain produced an explicit release error")
        record("failed keychain write created no provider state or plaintext fallback")
        (evidence_dir / "app-data-inventory.json").write_text(
            json.dumps(inventory, indent=2) + "\n", encoding="utf-8"
        )
        return {
            "result": "PASS",
            "mode": "failure",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "instanceId": bootstrap.get("instanceId"),
            "profileDirectory": str(profile_dir),
            "appDataDirectory": str(app_data_dir),
            "error": error,
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001
        steps.append({"name": "release keychain failure path", "result": "FAIL"})
        failure = {
            "result": "FAIL",
            "mode": "failure",
            "error": f"{type(error).__name__}: {error}",
            "steps": steps,
        }
        try:
            if client.session_id is not None:
                capture(client, evidence_dir, "keychain-failure-path-failure")
        except Exception as evidence_error:  # noqa: BLE001
            failure["evidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--driver-url", required=True)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    parser.add_argument("--mode", choices=("success", "failure"), required=True)
    args = parser.parse_args()

    application = args.application.resolve()
    if not application.is_file():
        parser.error(f"application does not exist: {application}")
    result_path = args.evidence_dir / f"release-keychain-{args.mode}-result.json"
    try:
        result = (
            run_success(application, args.driver_url, args.evidence_dir)
            if args.mode == "success"
            else run_failure(application, args.driver_url, args.evidence_dir)
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
