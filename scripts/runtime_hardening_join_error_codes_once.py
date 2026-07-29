from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise SystemExit(f"expected {label} anchor was not found")


mod_path = Path("src-tauri/src/app_state/mod.rs")
mod_text = mod_path.read_text(encoding="utf-8")
mod_anchor = '''impl std::error::Error for DesktopTableActionError {}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]'''
mod_replacement = '''impl std::error::Error for DesktopTableActionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopJoinSessionErrorCode {
    InvalidJoinPayload,
    NetworkTimeout,
    ClientRuntimeDisconnected,
    CommandFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopJoinSessionError {
    code: DesktopJoinSessionErrorCode,
    message: String,
}

impl DesktopJoinSessionError {
    #[must_use]
    pub fn new(code: DesktopJoinSessionErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn code(&self) -> DesktopJoinSessionErrorCode {
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

impl std::fmt::Display for DesktopJoinSessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DesktopJoinSessionError {}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]'''
mod_text = replace_once(
    mod_text,
    mod_anchor,
    mod_replacement,
    "DesktopJoinSessionError insertion",
)
mod_path.write_text(mod_text, encoding="utf-8")

app_path = Path("src-tauri/src/app_state/app.rs")
app_text = app_path.read_text(encoding="utf-8")
app_text = replace_once(
    app_text,
    '''    pub fn join_host_session(
        &self,
        request: JoinHostSessionRequest,
    ) -> Result<ClientSessionStatus, String> {''',
    '''    pub fn join_host_session(
        &self,
        request: JoinHostSessionRequest,
    ) -> Result<ClientSessionStatus, DesktopJoinSessionError> {''',
    "public join_host_session signature",
)
old_join = '''    fn join_host_session_with_player_id(
        &self,
        request: JoinHostSessionRequest,
        player_id: String,
    ) -> Result<ClientSessionStatus, String> {
        let payload = request.join_payload.trim();
        let display_name = request.display_name.trim();

        if payload.is_empty() {
            return Err("joinPayload must be non-blank".to_string());
        }

        if display_name.is_empty() {
            return Err("displayName must be non-blank".to_string());
        }

        if self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .is_some()
        {
            return Err("stop the active host session before joining another table".to_string());
        }

        if self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .is_some()
        {
            return Err("leave the active client session before joining another table".to_string());
        }

        let join_payload =
            protocol::decode_join_payload(payload).map_err(|error| error.to_string())?;
        let provider = crypto::DefaultCryptoProvider;
        let runtime = networking::ClientRuntime::connect(networking::ClientRuntimeConfig {
            join_payload: payload.to_string(),
            player_id,
            display_name: display_name.to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .map_err(|error| error.to_string())?;

        let latest_snapshot = match runtime.poll_event(INITIAL_JOIN_SNAPSHOT_TIMEOUT) {
            Ok(networking::ClientRuntimeEvent::Snapshot(snapshot)) => {
                client_snapshot_state_from_event(&snapshot)
            }
            Ok(other) => {
                return Err(format!(
                    "expected an initial snapshot event after join, got {other:?}"
                ));
            }
            Err(networking::ClientRuntimePollError::Timeout) => {
                return Err(format!(
                    "initial join timed out: host snapshot was not available within {} seconds",
                    INITIAL_JOIN_SNAPSHOT_TIMEOUT.as_secs()
                ));
            }
            Err(networking::ClientRuntimePollError::Disconnected) => {
                return Err(
                    "client runtime disconnected before the initial snapshot was available"
                        .to_string(),
                );
            }
        };

        let mut client_session = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?;
        *client_session = Some(DesktopClientSession {
            runtime,
            join_payload,
            latest_snapshot,
            reconnecting: false,
            terminated: false,
            last_error: None,
            event_feed: Vec::new(),
        });

        Ok(client_session
            .as_mut()
            .expect("client session was just inserted")
            .status())
    }'''
new_join = '''    fn join_host_session_with_player_id(
        &self,
        request: JoinHostSessionRequest,
        player_id: String,
    ) -> Result<ClientSessionStatus, DesktopJoinSessionError> {
        let payload = request.join_payload.trim();
        let display_name = request.display_name.trim();

        if payload.is_empty() {
            return Err(DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::InvalidJoinPayload,
                "joinPayload must be non-blank",
            ));
        }

        if display_name.is_empty() {
            return Err(DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::InvalidJoinPayload,
                "displayName must be non-blank",
            ));
        }

        if self
            .host_session
            .lock()
            .map_err(|_| {
                DesktopJoinSessionError::new(
                    DesktopJoinSessionErrorCode::CommandFailed,
                    "host session lock poisoned",
                )
            })?
            .is_some()
        {
            return Err(DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::CommandFailed,
                "stop the active host session before joining another table",
            ));
        }

        if self
            .client_session
            .lock()
            .map_err(|_| {
                DesktopJoinSessionError::new(
                    DesktopJoinSessionErrorCode::CommandFailed,
                    "client session lock poisoned",
                )
            })?
            .is_some()
        {
            return Err(DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::CommandFailed,
                "leave the active client session before joining another table",
            ));
        }

        let join_payload = protocol::decode_join_payload(payload).map_err(|error| {
            DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::InvalidJoinPayload,
                error.to_string(),
            )
        })?;
        let provider = crypto::DefaultCryptoProvider;
        let runtime = networking::ClientRuntime::connect(networking::ClientRuntimeConfig {
            join_payload: payload.to_string(),
            player_id,
            display_name: display_name.to_string(),
            signing_keys: provider.generate_signing_keypair(),
            encryption_keys: provider.generate_encryption_keypair(),
        })
        .map_err(|error| {
            DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::CommandFailed,
                error.to_string(),
            )
        })?;

        let latest_snapshot = match runtime.poll_event(INITIAL_JOIN_SNAPSHOT_TIMEOUT) {
            Ok(networking::ClientRuntimeEvent::Snapshot(snapshot)) => {
                client_snapshot_state_from_event(&snapshot)
            }
            Ok(other) => {
                return Err(DesktopJoinSessionError::new(
                    DesktopJoinSessionErrorCode::CommandFailed,
                    format!("expected an initial snapshot event after join, got {other:?}"),
                ));
            }
            Err(networking::ClientRuntimePollError::Timeout) => {
                return Err(DesktopJoinSessionError::new(
                    DesktopJoinSessionErrorCode::NetworkTimeout,
                    format!(
                        "initial join timed out: host snapshot was not available within {} seconds",
                        INITIAL_JOIN_SNAPSHOT_TIMEOUT.as_secs()
                    ),
                ));
            }
            Err(networking::ClientRuntimePollError::Disconnected) => {
                return Err(DesktopJoinSessionError::new(
                    DesktopJoinSessionErrorCode::ClientRuntimeDisconnected,
                    "client runtime disconnected before the initial snapshot was available",
                ));
            }
        };

        let mut client_session = self.client_session.lock().map_err(|_| {
            DesktopJoinSessionError::new(
                DesktopJoinSessionErrorCode::CommandFailed,
                "client session lock poisoned",
            )
        })?;
        *client_session = Some(DesktopClientSession {
            runtime,
            join_payload,
            latest_snapshot,
            reconnecting: false,
            terminated: false,
            last_error: None,
            event_feed: Vec::new(),
        });

        Ok(client_session
            .as_mut()
            .expect("client session was just inserted")
            .status())
    }'''
app_text = replace_once(app_text, old_join, new_join, "typed join_host_session implementation")
app_path.write_text(app_text, encoding="utf-8")

commands_path = Path("src-tauri/src/commands.rs")
commands = commands_path.read_text(encoding="utf-8")
commands = replace_once(
    commands,
    '''        DesktopBootstrapState, DesktopTableActionError, DesktopTableActionErrorCode,
        DesktopTableActionKind, HostSessionStatus, JoinHostSessionRequest, ScreenDescriptor,''',
    '''        DesktopBootstrapState, DesktopJoinSessionError, DesktopJoinSessionErrorCode,
        DesktopTableActionError, DesktopTableActionErrorCode, DesktopTableActionKind,
        HostSessionStatus, JoinHostSessionRequest, ScreenDescriptor,''',
    "commands join error imports",
)
commands = replace_once(
    commands,
    '''    fn invalid_join_payload(message: String) -> Self {
        Self::new(DesktopCommandErrorCode::InvalidJoinPayload, message)
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {''',
    '''    fn from_join_session(error: DesktopJoinSessionError) -> Self {
        let code = match error.code() {
            DesktopJoinSessionErrorCode::InvalidJoinPayload => {
                DesktopCommandErrorCode::InvalidJoinPayload
            }
            DesktopJoinSessionErrorCode::NetworkTimeout => DesktopCommandErrorCode::NetworkTimeout,
            DesktopJoinSessionErrorCode::ClientRuntimeDisconnected => {
                DesktopCommandErrorCode::ClientRuntimeDisconnected
            }
            DesktopJoinSessionErrorCode::CommandFailed => DesktopCommandErrorCode::CommandFailed,
        };
        Self::new(code, error.into_message())
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {''',
    "join error conversion",
)
commands = replace_once(
    commands,
    '''    let result = state
        .join_host_session(request)
        .map_err(DesktopCommandError::invalid_join_payload)?;''',
    '''    let result = state
        .join_host_session(request)
        .map_err(DesktopCommandError::from_join_session)?;''',
    "join command error mapping",
)
commands = replace_once(
    commands,
    '''mod command_error_tests {
    use super::{DesktopCommandError, DesktopTableActionError, DesktopTableActionErrorCode};''',
    '''mod command_error_tests {
    use super::{DesktopCommandError, DesktopTableActionError, DesktopTableActionErrorCode};
    use crate::app_state::{DesktopJoinSessionError, DesktopJoinSessionErrorCode};''',
    "command error test imports",
)
commands = replace_once(
    commands,
    '''    #[test]
    fn invalid_join_payload_code_is_recoverable_independent_of_wording() {
        for message in ["invite envelope was invalid", "rewritten join failure copy"] {
            let error = DesktopCommandError::invalid_join_payload(message.to_string());
            assert_eq!(error.code, "INVALID_JOIN_PAYLOAD");
            assert!(error.recoverable);
            assert_eq!(error.message, message);
        }
    }''',
    '''    #[test]
    fn typed_join_session_codes_are_independent_of_wording() {
        let cases = [
            (
                DesktopJoinSessionErrorCode::InvalidJoinPayload,
                "invite envelope was invalid",
                "INVALID_JOIN_PAYLOAD",
                true,
            ),
            (
                DesktopJoinSessionErrorCode::NetworkTimeout,
                "host snapshot deadline expired",
                "NETWORK_TIMEOUT",
                true,
            ),
            (
                DesktopJoinSessionErrorCode::ClientRuntimeDisconnected,
                "runtime channel closed",
                "CLIENT_RUNTIME_DISCONNECTED",
                false,
            ),
            (
                DesktopJoinSessionErrorCode::CommandFailed,
                "internal join setup failed",
                "COMMAND_FAILED",
                false,
            ),
        ];

        for (source_code, message, expected_code, expected_recoverable) in cases {
            let error = DesktopCommandError::from_join_session(DesktopJoinSessionError::new(
                source_code,
                message,
            ));
            assert_eq!(error.code, expected_code);
            assert_eq!(error.recoverable, expected_recoverable);
            assert_eq!(error.message, message);
        }
    }''',
    "typed join command tests",
)
commands_path.write_text(commands, encoding="utf-8")

sessions_path = Path("src-tauri/src/app_state/tests/sessions_lifecycle.rs")
sessions = sessions_path.read_text(encoding="utf-8")
sessions = replace_once(
    sessions,
    '''    ClaimLobbySeatRequest, DesktopAppState, DesktopTableActionErrorCode, DesktopTableActionKind,
    SetLobbyReadyStateRequest, TableViewerMode,''',
    '''    ClaimLobbySeatRequest, DesktopAppState, DesktopJoinSessionErrorCode,
    DesktopTableActionErrorCode, DesktopTableActionKind, SetLobbyReadyStateRequest,
    TableViewerMode,''',
    "session lifecycle join error import",
)
sessions = replace_once(
    sessions,
    '''    assert_eq!(
        error,
        "leave the active client session before joining another table"
    );''',
    '''    assert_eq!(error.code(), DesktopJoinSessionErrorCode::CommandFailed);
    assert_eq!(
        error.message(),
        "leave the active client session before joining another table"
    );''',
    "active client join assertion",
)
insert_anchor = '''#[test]
fn join_host_session_rejects_replacing_an_active_client_session() {'''
insert_test = '''#[test]
fn join_host_session_classifies_invalid_payload_without_message_matching() {
    let state = DesktopAppState::detect();
    let error = state
        .join_host_session(sample_join_host_session_request("not-a-valid-invite"))
        .expect_err("invalid invite should be rejected");

    assert_eq!(
        error.code(),
        DesktopJoinSessionErrorCode::InvalidJoinPayload
    );
    assert!(!error.message().is_empty());
}

#[test]
fn join_host_session_rejects_replacing_an_active_client_session() {'''
sessions = replace_once(
    sessions,
    insert_anchor,
    insert_test,
    "invalid join classification test insertion",
)
sessions_path.write_text(sessions, encoding="utf-8")
