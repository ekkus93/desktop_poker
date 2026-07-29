from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "src-tauri/src/app_state/mod.rs",
    """#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopTableActionKind {
    Fold,
    CheckOrCall,
    BetOrRaise,
    AllIn,
}
""",
    """#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopTableActionKind {
    Fold,
    CheckOrCall,
    BetOrRaise,
    AllIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopTableActionErrorCode {
    NoActiveSession,
    ObserverReadOnly,
    NotActingPlayer,
    StaleActionWindow,
    NetworkTimeout,
    ClientRuntimeDisconnected,
    HostRejectedAction,
    CommandFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopTableActionError {
    code: DesktopTableActionErrorCode,
    message: String,
}

impl DesktopTableActionError {
    #[must_use]
    pub fn new(code: DesktopTableActionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn code(&self) -> DesktopTableActionErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn into_message(self) -> String {
        self.message
    }
}

impl std::fmt::Display for DesktopTableActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DesktopTableActionError {}
""",
)

replace_once(
    "src-tauri/src/app_state/app.rs",
    """    pub fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        if let Some(table_view) = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        if let Some(table_view) = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        let _ = (viewer_mode, action_kind, raise_to_amount);
        Err("no active live session is available for table actions".to_string())
    }""",
    """    pub fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, DesktopTableActionError> {
        if let Some(table_view) = self
            .host_session
            .lock()
            .map_err(|_| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::CommandFailed,
                    "host session lock poisoned",
                )
            })?
            .as_ref()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        if let Some(table_view) = self
            .client_session
            .lock()
            .map_err(|_| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::CommandFailed,
                    "client session lock poisoned",
                )
            })?
            .as_mut()
            .map(|session| session.submit_table_action(viewer_mode, action_kind, raise_to_amount))
            .transpose()?
        {
            return Ok(table_view);
        }

        let _ = (viewer_mode, action_kind, raise_to_amount);
        Err(DesktopTableActionError::new(
            DesktopTableActionErrorCode::NoActiveSession,
            "no active live session is available for table actions",
        ))
    }""",
)

replace_once(
    "src-tauri/src/app_state/session.rs",
    """    pub(crate) fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        if matches!(viewer_mode, TableViewerMode::Observer) {
            return Err("observer mode cannot submit actions".to_string());
        }

        let current_window = self
            .host_server
            .authoritative_state()
            .map_err(|error| error.to_string())?
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .ok_or_else(|| "no open action window".to_string())?;

        if current_window.player_id != LOCAL_PLAYER_ID {
            return Err("action tray is disabled until the local player owns the turn".to_string());
        }

        let (action_type, action_amount, _) =
            resolve_action_request(&current_window, action_kind, raise_to_amount)?;
        self.host_server
            .submit_action(
                LOCAL_PLAYER_ID,
                current_window.action_window_id,
                action_type,
                action_amount,
            )
            .map_err(|error| error.to_string())?;
        self.table_view(viewer_mode)
    }""",
    """    pub(crate) fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, DesktopTableActionError> {
        if matches!(viewer_mode, TableViewerMode::Observer) {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::ObserverReadOnly,
                "observer mode cannot submit actions",
            ));
        }

        let current_window = self
            .host_server
            .authoritative_state()
            .map_err(|error| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::CommandFailed,
                    error.to_string(),
                )
            })?
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .ok_or_else(|| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::StaleActionWindow,
                    "no open action window",
                )
            })?;

        if current_window.player_id != LOCAL_PLAYER_ID {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NotActingPlayer,
                "action tray is disabled until the local player owns the turn",
            ));
        }

        let (action_type, action_amount, _) =
            resolve_action_request(&current_window, action_kind, raise_to_amount).map_err(
                |message| {
                    DesktopTableActionError::new(
                        DesktopTableActionErrorCode::HostRejectedAction,
                        message,
                    )
                },
            )?;
        self.host_server
            .submit_action(
                LOCAL_PLAYER_ID,
                current_window.action_window_id,
                action_type,
                action_amount,
            )
            .map_err(|error| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::HostRejectedAction,
                    error.to_string(),
                )
            })?;
        self.table_view(viewer_mode).map_err(|message| {
            DesktopTableActionError::new(DesktopTableActionErrorCode::CommandFailed, message)
        })
    }""",
)

replace_once(
    "src-tauri/src/app_state/session.rs",
    """    pub(crate) fn submit_table_action(
        &mut self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        if matches!(viewer_mode, TableViewerMode::Observer) {
            return Err("observer mode cannot submit actions".to_string());
        }

        self.refresh();
        let current_window = self
            .latest_snapshot
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .ok_or_else(|| "no open action window".to_string())?;

        if current_window.player_id != self.latest_snapshot.local_player_id {
            return Err("action tray is disabled until the local player owns the turn".to_string());
        }

        let (action_type, action_amount, _) =
            resolve_action_request(&current_window, action_kind, raise_to_amount)?;
        self.last_error = None;
        let prior_action_window_id = current_window.action_window_id.clone();
        let prior_hand_number = self
            .latest_snapshot
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.hand_number);
        self.runtime
            .submit_action(
                current_window.action_window_id,
                current_window.seat_index,
                action_type,
                action_amount,
            )
            .map_err(|error| error.to_string())?;
        let observed = self.await_condition(CLIENT_ACTION_ACK_TIMEOUT, |session| {
            session.last_error.is_some()
                || session
                    .latest_snapshot
                    .state
                    .current_hand
                    .as_ref()
                    .map(|hand| hand.hand_number)
                    != prior_hand_number
                || session
                    .latest_snapshot
                    .state
                    .current_hand
                    .as_ref()
                    .and_then(|hand| hand.action_window.as_ref())
                    .map(|window| window.action_window_id.as_str())
                    != Some(prior_action_window_id.as_str())
        });

        if let Some(error) = self.last_error.clone() {
            return Err(error);
        }
        if !observed {
            return Err(format!(
                "table action timed out: host did not acknowledge within {} seconds",
                CLIENT_ACTION_ACK_TIMEOUT.as_secs()
            ));
        }

        self.table_view(viewer_mode)
    }""",
    """    pub(crate) fn submit_table_action(
        &mut self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, DesktopTableActionError> {
        if matches!(viewer_mode, TableViewerMode::Observer) {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::ObserverReadOnly,
                "observer mode cannot submit actions",
            ));
        }

        self.refresh();
        if self.terminated {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::ClientRuntimeDisconnected,
                self.last_error
                    .clone()
                    .unwrap_or_else(|| "Disconnected from host".to_string()),
            ));
        }
        let current_window = self
            .latest_snapshot
            .state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .ok_or_else(|| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::StaleActionWindow,
                    "no open action window",
                )
            })?;

        if current_window.player_id != self.latest_snapshot.local_player_id {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NotActingPlayer,
                "action tray is disabled until the local player owns the turn",
            ));
        }

        let (action_type, action_amount, _) =
            resolve_action_request(&current_window, action_kind, raise_to_amount).map_err(
                |message| {
                    DesktopTableActionError::new(
                        DesktopTableActionErrorCode::HostRejectedAction,
                        message,
                    )
                },
            )?;
        self.last_error = None;
        let prior_action_window_id = current_window.action_window_id.clone();
        let prior_hand_number = self
            .latest_snapshot
            .state
            .current_hand
            .as_ref()
            .map(|hand| hand.hand_number);
        self.runtime
            .submit_action(
                current_window.action_window_id,
                current_window.seat_index,
                action_type,
                action_amount,
            )
            .map_err(|error| {
                DesktopTableActionError::new(
                    DesktopTableActionErrorCode::ClientRuntimeDisconnected,
                    error.to_string(),
                )
            })?;
        let observed = self.await_condition(CLIENT_ACTION_ACK_TIMEOUT, |session| {
            session.last_error.is_some()
                || session
                    .latest_snapshot
                    .state
                    .current_hand
                    .as_ref()
                    .map(|hand| hand.hand_number)
                    != prior_hand_number
                || session
                    .latest_snapshot
                    .state
                    .current_hand
                    .as_ref()
                    .and_then(|hand| hand.action_window.as_ref())
                    .map(|window| window.action_window_id.as_str())
                    != Some(prior_action_window_id.as_str())
        });

        if let Some(error) = self.last_error.clone() {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::HostRejectedAction,
                error,
            ));
        }
        if !observed {
            return Err(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NetworkTimeout,
                format!(
                    "table action timed out: host did not acknowledge within {} seconds",
                    CLIENT_ACTION_ACK_TIMEOUT.as_secs()
                ),
            ));
        }

        self.table_view(viewer_mode).map_err(|message| {
            DesktopTableActionError::new(DesktopTableActionErrorCode::CommandFailed, message)
        })
    }""",
)

replace_once(
    "src-tauri/src/commands.rs",
    """        ClaimLobbySeatRequest, ClientSessionStatus, DebugInspectorState, DesktopAppState,
        DesktopBootstrapState, DesktopTableActionKind, HostSessionStatus, JoinHostSessionRequest,
        ScreenDescriptor, SetLobbyReadyStateRequest, StartHostSessionRequest, TableViewSnapshot,
        TableViewerMode,""",
    """        ClaimLobbySeatRequest, ClientSessionStatus, DebugInspectorState, DesktopAppState,
        DesktopBootstrapState, DesktopTableActionError, DesktopTableActionErrorCode,
        DesktopTableActionKind, HostSessionStatus, JoinHostSessionRequest, ScreenDescriptor,
        SetLobbyReadyStateRequest, StartHostSessionRequest, TableViewSnapshot, TableViewerMode,""",
)

replace_once(
    "src-tauri/src/commands.rs",
    """impl DesktopCommandError {
    fn from_message(
        message: String,
        fallback_code: &'static str,
        fallback_recoverable: bool,
    ) -> Self {
        let normalized = message.to_ascii_lowercase();
        let (code, recoverable) = if normalized.contains("no active") {
            ("NO_ACTIVE_SESSION", true)
        } else if normalized.contains("observer mode")
            || normalized.contains("observer") && normalized.contains("cannot submit")
        {
            ("OBSERVER_READ_ONLY", true)
        } else if normalized.contains("does not own the action window")
            || normalized.contains("until the local player owns the turn")
        {
            ("NOT_ACTING_PLAYER", true)
        } else if normalized.contains("stale action window")
            || normalized.contains("no open action window")
        {
            ("STALE_ACTION_WINDOW", true)
        } else if normalized.contains("timed out") || normalized.contains("timeout") {
            ("NETWORK_TIMEOUT", true)
        } else if normalized.contains("runtime stopped unexpectedly")
            || normalized.contains("event channel disconnected")
            || normalized.contains("disconnected from host")
        {
            ("CLIENT_RUNTIME_DISCONNECTED", false)
        } else if normalized.contains("joinpayload")
            || normalized.contains("join payload")
            || normalized.contains("invalid rejection envelope")
            || normalized.contains("failed to decode")
        {
            ("INVALID_JOIN_PAYLOAD", true)
        } else {
            (fallback_code, fallback_recoverable)
        };

        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }
}""",
    """impl DesktopCommandError {
    fn new(code: &'static str, message: String, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {
        let (code, recoverable) = match error.code() {
            DesktopTableActionErrorCode::NoActiveSession => ("NO_ACTIVE_SESSION", true),
            DesktopTableActionErrorCode::ObserverReadOnly => ("OBSERVER_READ_ONLY", true),
            DesktopTableActionErrorCode::NotActingPlayer => ("NOT_ACTING_PLAYER", true),
            DesktopTableActionErrorCode::StaleActionWindow => ("STALE_ACTION_WINDOW", true),
            DesktopTableActionErrorCode::NetworkTimeout => ("NETWORK_TIMEOUT", true),
            DesktopTableActionErrorCode::ClientRuntimeDisconnected => {
                ("CLIENT_RUNTIME_DISCONNECTED", false)
            }
            DesktopTableActionErrorCode::HostRejectedAction => ("HOST_REJECTED_ACTION", true),
            DesktopTableActionErrorCode::CommandFailed => ("COMMAND_FAILED", false),
        };
        Self::new(code, error.into_message(), recoverable)
    }
}""",
)

replace_once(
    "src-tauri/src/commands.rs",
    """    let result = state.host_start_tournament().map_err(|message| {
        DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
    })?;""",
    """    let result = state.host_start_tournament().map_err(|message| {
        DesktopCommandError::new("HOST_REJECTED_ACTION", message, true)
    })?;""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """    let result = state.join_host_session(request).map_err(|message| {
        DesktopCommandError::from_message(message, "INVALID_JOIN_PAYLOAD", true)
    })?;""",
    """    let result = state
        .join_host_session(request)
        .map_err(|message| DesktopCommandError::new("INVALID_JOIN_PAYLOAD", message, true))?;""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """    let result = state.client_claim_lobby_seat(request).map_err(|message| {
        DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
    })?;""",
    """    let result = state.client_claim_lobby_seat(request).map_err(|message| {
        DesktopCommandError::new("HOST_REJECTED_ACTION", message, true)
    })?;""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """        .map_err(|message| {
            DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true)
        })?;""",
    """        .map_err(|message| DesktopCommandError::new("HOST_REJECTED_ACTION", message, true))?;""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """    )
    .map_err(|message| DesktopCommandError::from_message(message, "HOST_REJECTED_ACTION", true))?;""",
    """    )
    .map_err(DesktopCommandError::from_table_action)?;""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """    ) -> Result<TableViewSnapshot, String>,
) -> Result<TableViewSnapshot, String> {
    submit_table_action(viewer_mode, action_kind, raise_to_amount)
}""",
    """    ) -> Result<TableViewSnapshot, DesktopTableActionError>,
) -> Result<TableViewSnapshot, DesktopTableActionError> {
    submit_table_action(viewer_mode, action_kind, raise_to_amount)
}""",
)

replace_once(
    "src-tauri/src/app_state/tests/sessions.rs",
    """    detect_profile_directory, screen_catalog, DesktopAppState, DesktopTableActionKind,
    TableViewerMode, INSTANCE_ID_ENV_VAR, JOIN_PAYLOAD_ENV_VAR,""",
    """    detect_profile_directory, screen_catalog, DesktopAppState, DesktopTableActionErrorCode,
    DesktopTableActionKind, TableViewerMode, INSTANCE_ID_ENV_VAR, JOIN_PAYLOAD_ENV_VAR,""",
)
replace_once(
    "src-tauri/src/app_state/tests/sessions.rs",
    """    assert_eq!(
        DesktopAppState::detect()
            .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
            .expect_err("table actions should require an active live session"),
        "no active live session is available for table actions"
    );""",
    """    let error = DesktopAppState::detect()
        .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
        .expect_err("table actions should require an active live session");

    assert_eq!(error.code(), DesktopTableActionErrorCode::NoActiveSession);
    assert_eq!(
        error.message(),
        "no active live session is available for table actions"
    );""",
)

replace_once(
    "src-tauri/src/commands.rs",
    """                Err("invalid action".to_string())
            },
        );""",
    """                Err(DesktopTableActionError::new(
                    DesktopTableActionErrorCode::HostRejectedAction,
                    "invalid action",
                ))
            },
        );""",
)
replace_once(
    "src-tauri/src/commands.rs",
    """        assert_eq!(
            submit_error.expect_err("submit should fail"),
            "invalid action"
        );""",
    """        assert_eq!(
            submit_error
                .expect_err("submit should fail")
                .message(),
            "invalid action"
        );""",
)

# Replace message-classification tests with stable typed-code tests.
commands = Path("src-tauri/src/commands.rs")
text = commands.read_text(encoding="utf-8")
start = text.index("#[cfg(test)]\nmod command_error_tests {")
new_tests = r'''#[cfg(test)]
mod command_error_tests {
    use super::{
        DesktopCommandError, DesktopTableActionError, DesktopTableActionErrorCode,
    };

    #[test]
    fn typed_timeout_code_is_recoverable_independent_of_wording() {
        for message in ["host acknowledgement deadline expired", "rewritten timeout copy"] {
            let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
                DesktopTableActionErrorCode::NetworkTimeout,
                message,
            ));
            assert_eq!(error.code, "NETWORK_TIMEOUT");
            assert!(error.recoverable);
        }
    }

    #[test]
    fn typed_dead_runtime_code_is_fatal_independent_of_wording() {
        for message in ["runtime unavailable", "connection ended"] {
            let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
                DesktopTableActionErrorCode::ClientRuntimeDisconnected,
                message,
            ));
            assert_eq!(error.code, "CLIENT_RUNTIME_DISCONNECTED");
            assert!(!error.recoverable);
        }
    }

    #[test]
    fn command_error_serializes_stable_fields() {
        let error = DesktopCommandError::from_table_action(DesktopTableActionError::new(
            DesktopTableActionErrorCode::ObserverReadOnly,
            "spectators cannot act",
        ));
        let value = serde_json::to_value(error).expect("command error serializes");
        assert_eq!(value["code"], "OBSERVER_READ_ONLY");
        assert_eq!(value["recoverable"], true);
        assert_eq!(value["message"], "spectators cannot act");
    }
}
'''
commands.write_text(text[:start] + new_tests, encoding="utf-8")
