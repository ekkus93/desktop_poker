from pathlib import Path

commands_path = Path("src-tauri/src/commands.rs")
commands = commands_path.read_text(encoding="utf-8")

old_error_block = '''#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl DesktopCommandError {
    fn new(code: &'static str, message: String, recoverable: bool) -> Self {
        Self {
            code: code.to_string(),
            message,
            recoverable,
        }
    }

    fn invalid_join_payload(message: String) -> Self {
        Self::new("INVALID_JOIN_PAYLOAD", message, true)
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
}
'''

new_error_block = '''#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DesktopCommandErrorCode {
    NoActiveSession,
    ObserverReadOnly,
    NotActingPlayer,
    StaleActionWindow,
    NetworkTimeout,
    ClientRuntimeDisconnected,
    InvalidJoinPayload,
    HostRejectedAction,
    CommandFailed,
}

impl DesktopCommandErrorCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveSession => "NO_ACTIVE_SESSION",
            Self::ObserverReadOnly => "OBSERVER_READ_ONLY",
            Self::NotActingPlayer => "NOT_ACTING_PLAYER",
            Self::StaleActionWindow => "STALE_ACTION_WINDOW",
            Self::NetworkTimeout => "NETWORK_TIMEOUT",
            Self::ClientRuntimeDisconnected => "CLIENT_RUNTIME_DISCONNECTED",
            Self::InvalidJoinPayload => "INVALID_JOIN_PAYLOAD",
            Self::HostRejectedAction => "HOST_REJECTED_ACTION",
            Self::CommandFailed => "COMMAND_FAILED",
        }
    }

    const fn recoverable(self) -> bool {
        !matches!(Self::ClientRuntimeDisconnected | Self::CommandFailed, self)
    }
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl DesktopCommandError {
    fn new(code: DesktopCommandErrorCode, message: String) -> Self {
        Self {
            code: code.as_str().to_string(),
            message,
            recoverable: code.recoverable(),
        }
    }

    fn invalid_join_payload(message: String) -> Self {
        Self::new(DesktopCommandErrorCode::InvalidJoinPayload, message)
    }

    fn from_table_action(error: DesktopTableActionError) -> Self {
        let code = match error.code() {
            DesktopTableActionErrorCode::NoActiveSession => DesktopCommandErrorCode::NoActiveSession,
            DesktopTableActionErrorCode::ObserverReadOnly => DesktopCommandErrorCode::ObserverReadOnly,
            DesktopTableActionErrorCode::NotActingPlayer => DesktopCommandErrorCode::NotActingPlayer,
            DesktopTableActionErrorCode::StaleActionWindow => DesktopCommandErrorCode::StaleActionWindow,
            DesktopTableActionErrorCode::NetworkTimeout => DesktopCommandErrorCode::NetworkTimeout,
            DesktopTableActionErrorCode::ClientRuntimeDisconnected => {
                DesktopCommandErrorCode::ClientRuntimeDisconnected
            }
            DesktopTableActionErrorCode::HostRejectedAction => DesktopCommandErrorCode::HostRejectedAction,
            DesktopTableActionErrorCode::CommandFailed => DesktopCommandErrorCode::CommandFailed,
        };
        Self::new(code, error.into_message())
    }
}
'''

if old_error_block in commands:
    commands = commands.replace(old_error_block, new_error_block, 1)
elif new_error_block not in commands:
    raise SystemExit("expected DesktopCommandError block was not found")

replacements = {
    'DesktopCommandError::new("HOST_REJECTED_ACTION", message, true)': 'DesktopCommandError::new(DesktopCommandErrorCode::HostRejectedAction, message)',
}
for old, new in replacements.items():
    commands = commands.replace(old, new)

if 'DesktopCommandError::new("' in commands:
    raise SystemExit("raw string DesktopCommandError construction remains")

commands_path.write_text(commands, encoding="utf-8")

contract_path = Path("src-tauri/tests/desktop_contract.rs")
contract = contract_path.read_text(encoding="utf-8")

old_import = '''    npc::{
        profile_store::NpcProfileListResult, LlmProviderConfig, LlmProviderSettings,
        LlmProviderType,
    },
};'''
new_import = '''    networking::HostRuntimeHealth,
    npc::{
        profile_store::NpcProfileListResult, LlmProviderConfig, LlmProviderSettings,
        LlmProviderType,
    },
};'''
if old_import in contract:
    contract = contract.replace(old_import, new_import, 1)
elif new_import not in contract:
    raise SystemExit("expected desktop contract import block was not found")

anchor = '''    assert_contract_keys(
        "NpcProfileListResult",
        &NpcProfileListResult {
'''
insertion = '''    assert_contract_keys("HostRuntimeHealth", &HostRuntimeHealth::default());

    assert_contract_keys(
        "NpcProfileListResult",
        &NpcProfileListResult {
'''
if anchor in contract:
    contract = contract.replace(anchor, insertion, 1)
elif 'assert_contract_keys("HostRuntimeHealth"' not in contract:
    raise SystemExit("expected desktop contract assertion anchor was not found")

contract_path.write_text(contract, encoding="utf-8")
