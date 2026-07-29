from pathlib import Path

path = Path("src-tauri/src/app_state/tests/sessions_lifecycle.rs")
text = path.read_text(encoding="utf-8")

old_import = '''use super::super::{
    ClaimLobbySeatRequest, DesktopAppState, DesktopTableActionKind, SetLobbyReadyStateRequest,
    TableViewerMode,
};'''
new_import = '''use super::super::{
    ClaimLobbySeatRequest, DesktopAppState, DesktopTableActionErrorCode, DesktopTableActionKind,
    SetLobbyReadyStateRequest, TableViewerMode,
};'''
if old_import in text:
    text = text.replace(old_import, new_import, 1)
elif new_import not in text:
    raise SystemExit("expected lifecycle test import block was not found")

old_assertion = '''    assert!(
        err.contains("no open action window")
            || err.contains("no active client session")
            || err.contains("action tray"),
        "error should describe why the action failed; got: {err}"
    );'''
new_assertion = '''    assert_eq!(
        err.code(),
        DesktopTableActionErrorCode::StaleActionWindow,
        "a joined client without an active action window should receive the stable stale-window code"
    );
    assert!(
        err.message().contains("no open action window"),
        "error should describe why the action failed; got: {err}"
    );'''
if old_assertion in text:
    text = text.replace(old_assertion, new_assertion, 1)
elif new_assertion not in text:
    raise SystemExit("expected lifecycle table-action assertion was not found")

path.write_text(text, encoding="utf-8")
