#!/usr/bin/env python3
"""Drive the Linux release binary through tauri-driver using only stdlib HTTP.

This intentionally tests the packaged Tauri/WebKit runtime rather than the Vite
browser-mock surface. Evidence is written even when a validation step fails.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

W3C_ELEMENT_KEY = "element-6066-11e4-a52e-4f735466cecf"


class WebDriverError(RuntimeError):
    pass


class WebDriverClient:
    def __init__(self, base_url: str) -> None:
        self.base_url = base_url.rstrip("/")
        self.session_id: str | None = None

    def request(
        self,
        method: str,
        path: str,
        payload: dict[str, Any] | None = None,
        *,
        timeout: float = 30.0,
    ) -> Any:
        body = None
        headers: dict[str, str] = {}
        if payload is not None:
            body = json.dumps(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"

        request = urllib.request.Request(
            f"{self.base_url}{path}", data=body, headers=headers, method=method
        )
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                raw = response.read().decode("utf-8")
        except urllib.error.HTTPError as error:
            raw = error.read().decode("utf-8", errors="replace")
            raise WebDriverError(
                f"WebDriver {method} {path} returned HTTP {error.code}: {raw}"
            ) from error
        except urllib.error.URLError as error:
            raise WebDriverError(
                f"WebDriver {method} {path} failed: {error.reason}"
            ) from error

        if not raw:
            return None
        decoded = json.loads(raw)
        value = decoded.get("value")
        if isinstance(value, dict) and value.get("error"):
            raise WebDriverError(
                f"WebDriver {method} {path} error {value.get('error')}: "
                f"{value.get('message')}"
            )
        return value

    def wait_until_ready(self, timeout: float = 60.0) -> None:
        deadline = time.monotonic() + timeout
        last_error: Exception | None = None
        while time.monotonic() < deadline:
            try:
                self.request("GET", "/status", timeout=2.0)
                return
            except Exception as error:  # noqa: BLE001 - retain final driver error
                last_error = error
                time.sleep(0.5)
        raise WebDriverError(f"tauri-driver did not become ready: {last_error}")

    def start_session(self, application: Path) -> None:
        value = self.request(
            "POST",
            "/session",
            {
                "capabilities": {
                    "alwaysMatch": {
                        "browserName": "wry",
                        "tauri:options": {"application": str(application)},
                    },
                    "firstMatch": [{}],
                }
            },
            timeout=90.0,
        )
        if not isinstance(value, dict) or not value.get("sessionId"):
            raise WebDriverError(f"Unexpected new-session response: {value!r}")
        self.session_id = str(value["sessionId"])

    def close(self) -> None:
        if self.session_id is None:
            return
        try:
            self.request("DELETE", f"/session/{self.session_id}", timeout=15.0)
        finally:
            self.session_id = None

    def session_request(
        self, method: str, suffix: str, payload: dict[str, Any] | None = None
    ) -> Any:
        if self.session_id is None:
            raise WebDriverError("No active WebDriver session")
        return self.request(
            method, f"/session/{self.session_id}{suffix}", payload, timeout=30.0
        )

    def source(self) -> str:
        value = self.session_request("GET", "/source")
        return str(value)

    def pathname(self) -> str:
        value = self.execute("return window.location.pathname;")
        return str(value)

    def execute(self, script: str, args: list[Any] | None = None) -> Any:
        return self.session_request(
            "POST", "/execute/sync", {"script": script, "args": args or []}
        )

    def navigate(self, pathname: str) -> None:
        self.execute(
            "window.history.pushState({}, '', arguments[0]);"
            "window.dispatchEvent(new PopStateEvent('popstate'));"
            "return window.location.pathname;",
            [pathname],
        )

    def find(self, using: str, value: str) -> str:
        result = self.session_request(
            "POST", "/element", {"using": using, "value": value}
        )
        if not isinstance(result, dict) or W3C_ELEMENT_KEY not in result:
            raise WebDriverError(f"Unexpected element response: {result!r}")
        return str(result[W3C_ELEMENT_KEY])

    def find_xpath(self, xpath: str) -> str:
        return self.find("xpath", xpath)

    def find_css(self, selector: str) -> str:
        return self.find("css selector", selector)

    def click(self, element_id: str) -> None:
        self.session_request("POST", f"/element/{element_id}/click", {})

    def clear(self, element_id: str) -> None:
        self.session_request("POST", f"/element/{element_id}/clear", {})

    def type_text(self, element_id: str, text: str) -> None:
        self.session_request(
            "POST",
            f"/element/{element_id}/value",
            {"text": text, "value": list(text)},
        )

    def element_text(self, element_id: str) -> str:
        return str(self.session_request("GET", f"/element/{element_id}/text"))

    def screenshot(self) -> bytes:
        value = self.session_request("GET", "/screenshot")
        return base64.b64decode(str(value))


def wait_for(
    description: str,
    predicate: Any,
    *,
    timeout: float = 30.0,
    interval: float = 0.25,
) -> Any:
    deadline = time.monotonic() + timeout
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            value = predicate()
            if value:
                return value
        except Exception as error:  # noqa: BLE001 - retry transient render/driver state
            last_error = error
        time.sleep(interval)
    raise AssertionError(f"Timed out waiting for {description}; last error: {last_error}")


def xpath_clickable_text(text: str) -> str:
    quoted = json.dumps(text)
    return (
        f"//*[normalize-space(.)={quoted}]"
        "/ancestor-or-self::*[self::a or self::button][1]"
    )


def assert_source_contains(client: WebDriverClient, *needles: str) -> None:
    source = client.source()
    missing = [needle for needle in needles if needle not in source]
    if missing:
        raise AssertionError(f"Page source missing expected text: {missing}")


def wait_for_route(client: WebDriverClient, pathname: str, text: str) -> None:
    wait_for(
        f"route {pathname} with text {text!r}",
        lambda: client.pathname() == pathname and text in client.source(),
    )


def run(application: Path, driver_url: str, evidence_dir: Path) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    client = WebDriverClient(driver_url)
    steps: list[dict[str, str]] = []

    def record(name: str, result: str = "PASS") -> None:
        steps.append({"name": name, "result": result})
        print(f"[{result}] {name}", flush=True)

    try:
        client.wait_until_ready()
        record("tauri-driver became ready")
        client.start_session(application)
        record("release binary created a WebDriver session")

        wait_for_route(client, "/", "Choose a table")
        assert_source_contains(
            client,
            "Host Tournament",
            "Join Tournament",
            "Help",
            "Settings",
        )
        record("Home rendered through the real Tauri backend")

        browser_mocks = client.execute(
            "return Boolean(window.__DESKTOP_POKER_BROWSER_MOCKS__);"
        )
        if browser_mocks:
            raise AssertionError("Release runtime exposed browser mocks")
        record("browser mocks are unavailable in release mode")

        client.click(client.find_xpath(xpath_clickable_text("Host Tournament")))
        wait_for_route(client, "/host", "Host Tournament Setup")
        assert_source_contains(client, "Start hosting", "Continue to lobby")
        record("Host route rendered")

        client.navigate("/")
        wait_for_route(client, "/", "Choose a table")
        client.click(client.find_xpath(xpath_clickable_text("Join Tournament")))
        wait_for_route(client, "/join", "Join Tournament")
        assert_source_contains(client, "Check invite", "Continue to lobby")
        record("Join route rendered")

        invite = client.find_css("textarea")
        client.clear(invite)
        client.type_text(invite, "not-a-valid-poker-invite")
        client.click(client.find_xpath(xpath_clickable_text("Check invite")))
        error_element = wait_for(
            "invalid invite inline error",
            lambda: client.find_css(".inline-banner.error"),
        )
        error_text = client.element_text(error_element).strip()
        if not error_text:
            raise AssertionError("Invalid invite produced an empty error banner")
        if client.pathname() != "/join":
            raise AssertionError("Invalid invite navigated away from the Join screen")
        record(f"invalid invite failed explicitly: {error_text}")

        for pathname, expected in (("/settings", "Settings"), ("/rules", "Help")):
            client.navigate(pathname)
            wait_for_route(client, pathname, expected)
            record(f"{pathname} route rendered")

        for guarded in ("/lobby", "/table"):
            client.navigate(guarded)
            wait_for_route(client, "/", "Choose a table")
            record(f"session guard redirected {guarded} without a live session")

        client.navigate("/debug")
        wait_for_route(client, "/", "Choose a table")
        record("release /debug route is not reachable")

        screenshot_path = evidence_dir / "release-runtime-home.png"
        screenshot_path.write_bytes(client.screenshot())
        (evidence_dir / "release-runtime-page-source.html").write_text(
            client.source(), encoding="utf-8"
        )
        record("runtime screenshot and final page source captured")

        result = {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "instanceId": os.environ.get("DESKTOP_POKER_INSTANCE_ID"),
            "steps": steps,
        }
        return result
    except Exception as error:  # noqa: BLE001 - evidence must include all failures
        steps.append({"name": "runtime smoke", "result": "FAIL"})
        failure = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "instanceId": os.environ.get("DESKTOP_POKER_INSTANCE_ID"),
            "error": f"{type(error).__name__}: {error}",
            "steps": steps,
        }
        try:
            if client.session_id is not None:
                (evidence_dir / "release-runtime-failure-source.html").write_text(
                    client.source(), encoding="utf-8"
                )
                (evidence_dir / "release-runtime-failure.png").write_bytes(
                    client.screenshot()
                )
        except Exception as evidence_error:  # noqa: BLE001
            failure["evidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--driver-url", default="http://127.0.0.1:4444")
    parser.add_argument(
        "--evidence-dir", type=Path, default=Path("runtime-validation-evidence")
    )
    args = parser.parse_args()

    application = args.application.resolve()
    if not application.is_file():
        raise SystemExit(f"Release binary does not exist: {application}")
    if not os.access(application, os.X_OK):
        raise SystemExit(f"Release binary is not executable: {application}")

    result_path = args.evidence_dir / "release-runtime-result.json"
    try:
        result = run(application, args.driver_url, args.evidence_dir)
        result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return 0
    except Exception as error:  # noqa: BLE001
        args.evidence_dir.mkdir(parents=True, exist_ok=True)
        result_path.write_text(f"{error}\n", encoding="utf-8")
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
