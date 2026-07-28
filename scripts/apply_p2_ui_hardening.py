#!/usr/bin/env python3
"""Apply the audited P2 UI failure-path changes exactly once."""

from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one old anchor, found {count}")
    return text.replace(old, new, 1)


def patch_join_screen() -> None:
    path = Path("src/screens/JoinTournamentScreen.tsx")
    text = path.read_text(encoding="utf-8")

    old_helper = '''function normaliseError(error: unknown) {
  return error instanceof Error
    ? error.message
    : "The invite could not be checked.";
}
'''
    new_helper = old_helper + '''
function describeJoinFailure(
  error: unknown,
  payload: JoinPayload | null | undefined,
) {
  const endpoint = payload
    ? `${payload.hostAddress}:${payload.hostPort}`
    : "the advertised host";
  return `Unable to connect to ${endpoint}. ${normaliseError(error)} The invite can be valid even when the host is offline or blocked by a firewall.`;
}
'''
    text = replace_once(text, old_helper, new_helper, "join failure helper")
    text = replace_once(
        text,
        '''        setJoinError(normaliseError(error));
        setInviteBanner("Invite already attached to this launch.");
''',
        '''        setJoinError(
          describeJoinFailure(error, bootstrap.parsedLaunchJoinPayload),
        );
        setInviteBanner("Invite already attached to this launch.");
''',
        "automatic join failure",
    )
    text = replace_once(
        text,
        '''    } catch (error) {
      setJoinError(normaliseError(error));
    } finally {
''',
        '''    } catch (error) {
      setJoinError(describeJoinFailure(error, validationState.payload));
    } finally {
''',
        "manual join failure",
    )

    replacements = {
        "`Ready: ${parsedPayload.hostAddress}:${parsedPayload.hostPort}`":
            "`Invite decoded: ${parsedPayload.hostAddress}:${parsedPayload.hostPort}`",
        '      : "Invite checked. Join when ready."':
            '      : "Invite decoded. Connecting is the host reachability check."',
        "Checking the invite and host details…":
            "Checking the invite format and signed host details…",
        '<p className="kicker">Invite looks good</p>':
            '<p className="kicker">Invite decoded</p>',
        '<strong>Lobby ready</strong>':
            '<strong>Reachability not checked</strong>',
    }
    for old, new in replacements.items():
        text = replace_once(text, old, new, f"join copy: {old}")

    path.write_text(text, encoding="utf-8")


def patch_join_tests() -> None:
    path = Path("src/screens/JoinTournamentScreen.test.tsx")
    text = path.read_text(encoding="utf-8")
    text = replace_once(
        text,
        'expect(screen.getByText("Ready: 192.168.1.10:43818")).toBeTruthy();',
        '''expect(
      screen.getByText("Invite decoded: 192.168.1.10:43818"),
    ).toBeTruthy();''',
        "decoded invite assertion",
    )
    text = replace_once(
        text,
        'expect(screen.getByText(/invite checked\\. join when ready/i)).toBeTruthy();',
        '''expect(
      screen.getByText(/connecting is the host reachability check/i),
    ).toBeTruthy();
    expect(screen.getByText(/reachability not checked/i)).toBeTruthy();''',
        "reachability assertion",
    )
    text = replace_once(
        text,
        'expect(await screen.findByText(/connection refused/i)).toBeTruthy();',
        '''expect(
      await screen.findByText(
        /unable to connect to 192\\.168\\.1\\.10:43818.*connection refused.*offline or blocked by a firewall/i,
      ),
    ).toBeTruthy();''',
        "unreachable host assertion",
    )
    path.write_text(text, encoding="utf-8")


def patch_host_screen() -> None:
    path = Path("src/screens/HostTournamentSetupScreen.tsx")
    text = path.read_text(encoding="utf-8")

    old_helper = '''function clampPort(value: string, fallbackPort: number) {
  const parsedValue = Number.parseInt(value, 10);
  if (Number.isNaN(parsedValue)) {
    return fallbackPort;
  }

  return Math.max(1, Math.min(65535, parsedValue));
}
'''
    new_helper = old_helper + '''
function describeHostStartError(error: unknown, hostPort: number) {
  const message =
    error instanceof Error
      ? error.message.trim()
      : "Unable to start hosting.";
  const normalized = message.toLowerCase();
  const portIsBusy =
    normalized.includes("address already in use") ||
    normalized.includes("port is already in use") ||
    normalized.includes("os error 98") ||
    normalized.includes("os error 48") ||
    normalized.includes("wsaeaddrinuse");

  if (portIsBusy) {
    return `Unable to start hosting on TCP port ${hostPort} because that port is already in use. Choose a different port and try again.`;
  }

  return `Unable to start hosting on TCP port ${hostPort}. ${message}`;
}
'''
    text = replace_once(text, old_helper, new_helper, "host error helper")
    text = replace_once(
        text,
        '''    } catch (error) {
      setHostError(
        error instanceof Error ? error.message : "Unable to start hosting.",
      );
      setHostSession(null);
''',
        '''    } catch (error) {
      setHostError(describeHostStartError(error, hostDraft.hostPort));
      setHostSession(null);
''',
        "host start catch",
    )
    path.write_text(text, encoding="utf-8")


def patch_host_tests() -> None:
    path = Path("src/screens/HostTournamentSetupScreen.test.tsx")
    text = path.read_text(encoding="utf-8")
    text = replace_once(
        text,
        'expect(await screen.findByText("Address already in use")).toBeTruthy();',
        '''expect(
      await screen.findByText(
        /unable to start hosting on tcp port 43818 because that port is already in use.*choose a different port/i,
      ),
    ).toBeTruthy();''',
        "port conflict assertion",
    )

    generic_test = '''  it("includes the attempted port for other safe host-start failures", async () => {
    const bootstrap = createBootstrap({ debugToolsEnabled: false });
    mockedStartHostSession.mockRejectedValueOnce(
      new Error("Permission denied"),
    );

    renderWithProviders(<HostTournamentSetupScreen bootstrap={bootstrap} />, {
      bootstrap,
      initialEntries: ["/host"],
    });

    fireEvent.click(
      await screen.findByRole("button", { name: /start hosting/i }),
    );

    expect(
      await screen.findByText(
        "Unable to start hosting on TCP port 43818. Permission denied",
      ),
    ).toBeTruthy();
  });

'''
    if generic_test not in text:
        anchor = '  it("keeps critical setup options visible for legacy persisted host drafts", async () => {'
        if text.count(anchor) != 1:
            raise SystemExit("generic host failure test insertion anchor missing")
        text = text.replace(anchor, generic_test + anchor, 1)

    path.write_text(text, encoding="utf-8")


def verify_postconditions() -> None:
    checks = {
        Path("src/screens/JoinTournamentScreen.tsx"): [
            "describeJoinFailure",
            "Invite decoded:",
            "Reachability not checked",
            "host reachability check",
        ],
        Path("src/screens/JoinTournamentScreen.test.tsx"): [
            "offline or blocked by a firewall",
            "reachability not checked",
        ],
        Path("src/screens/HostTournamentSetupScreen.tsx"): [
            "describeHostStartError",
            "Choose a different port and try again",
        ],
        Path("src/screens/HostTournamentSetupScreen.test.tsx"): [
            "includes the attempted port",
            "port is already in use",
        ],
    }
    for path, markers in checks.items():
        text = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in text:
                raise SystemExit(f"{path}: missing postcondition {marker!r}")


if __name__ == "__main__":
    patch_join_screen()
    patch_join_tests()
    patch_host_screen()
    patch_host_tests()
    verify_postconditions()
