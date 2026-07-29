from pathlib import Path

path = Path("src-tauri/src/app_state/tests/sessions_lifecycle.rs")
text = path.read_text(encoding="utf-8")
old = '''    assert!(
        err.contains("no open action window")
            || err.contains("no active client session")
            || err.contains("action tray"),
        "error should describe why the action failed; got: {err}"
    );'''
new = '''    assert_eq!(
        err.code(),
        DesktopTableActionErrorCode::StaleActionWindow,
        "a joined client without an active action window should receive the stable stale-window code"
    );
    assert!(
        err.message().contains("no open action window"),
        "error should describe why the action failed; got: {err}"
    );'''

if old in text:
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
elif new not in text:
    raise SystemExit("expected lifecycle table-action assertion was not found")
