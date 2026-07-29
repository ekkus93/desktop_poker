from pathlib import Path


def replace_if_needed(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old in text:
        file_path.write_text(text.replace(old, new, 1), encoding="utf-8")
        return
    if new in text:
        return
    raise SystemExit(f"expected rustfmt text not found in {path}")


replace_if_needed(
    "src-tauri/src/networking/runtime/handlers.rs",
    '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| NetworkingError::new(format!("invalid join request payload: {error}")))?;''',
    '''    let request: JoinTournamentRequest = serde_json::from_value(request_envelope.payload.clone())
        .map_err(|error| {
            NetworkingError::new(format!("invalid join request payload: {error}"))
        })?;''',
)

replace_if_needed(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    "use std::{collections::HashMap, sync::{Arc, Mutex}};",
    '''use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};''',
)

replace_if_needed(
    "src-tauri/src/networking/runtime/tests/action_outcomes.rs",
    '''    let mut controller = TournamentController::new(
        "table-action-outcomes",
        401,
        config,
        registered_players,
    )
    .expect("controller builds");''',
    '''    let mut controller =
        TournamentController::new("table-action-outcomes", 401, config, registered_players)
            .expect("controller builds");''',
)
