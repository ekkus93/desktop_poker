from pathlib import Path

path = Path("src-tauri/src/networking/runtime/handlers.rs")
text = path.read_text(encoding="utf-8")
old = '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
            NetworkingError::new(format!("invalid join request payload: {error}"))
        })?;'''
new = '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
        NetworkingError::new(format!("invalid join request payload: {error}"))
    })?;'''
if old in text:
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
elif new not in text:
    raise SystemExit("expected handlers rustfmt block not found")
