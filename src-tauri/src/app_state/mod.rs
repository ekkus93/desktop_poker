use std::{
    env,
    path::{Path, PathBuf},
};

use dirs::data_local_dir;
use serde::Serialize;

use crate::{crypto, domain, engine, interop, networking, protocol, storage, tournament};

pub const INSTANCE_ID_ENV_VAR: &str = "DESKTOP_POKER_INSTANCE_ID";
pub const JOIN_PAYLOAD_ENV_VAR: &str = "DESKTOP_POKER_JOIN_PAYLOAD";
const INSTANCE_ID_ARG: &str = "--instance-id";
const JOIN_PAYLOAD_ARG: &str = "--join-payload";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDescriptor {
    pub name: &'static str,
    pub responsibility: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub route: &'static str,
    pub surface: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopBootstrapState {
    pub app_name: &'static str,
    pub protocol_version: u32,
    pub default_host_port: u16,
    pub frontend_stack: &'static str,
    pub serialization_strategy: &'static str,
    pub framing_strategy: &'static str,
    pub join_payload_encoding: &'static str,
    pub runtime_transport: &'static str,
    pub crypto_stack: Vec<&'static str>,
    pub instance_id: String,
    pub profile_directory: String,
    pub launch_join_payload: Option<String>,
    pub debug_tools_enabled: bool,
    pub backend_modules: Vec<ModuleDescriptor>,
    pub screens: Vec<ScreenDescriptor>,
}

#[derive(Clone, Debug)]
pub struct DesktopAppState {
    bootstrap: DesktopBootstrapState,
}

impl DesktopAppState {
    #[must_use]
    pub fn detect() -> Self {
        let instance_id = detect_instance_id();
        let profile_directory = detect_profile_directory(&instance_id);
        let debug_tools_enabled = cfg!(debug_assertions);

        Self {
            bootstrap: DesktopBootstrapState {
                app_name: "Desktop Poker",
                protocol_version: protocol::PROTOCOL_VERSION,
                default_host_port: networking::DEFAULT_HOST_PORT,
                frontend_stack: "React + TypeScript",
                serialization_strategy: protocol::SERIALIZATION_STRATEGY,
                framing_strategy: networking::FRAMING_STRATEGY,
                join_payload_encoding: interop::JOIN_PAYLOAD_ENCODING,
                runtime_transport: networking::RUNTIME_TRANSPORT,
                crypto_stack: crypto::stack(),
                instance_id,
                profile_directory: profile_directory.display().to_string(),
                launch_join_payload: detect_launch_join_payload(),
                debug_tools_enabled,
                backend_modules: backend_modules(),
                screens: screen_catalog(debug_tools_enabled),
            },
        }
    }

    #[must_use]
    pub fn bootstrap(&self) -> DesktopBootstrapState {
        self.bootstrap.clone()
    }

    #[must_use]
    pub fn screen_catalog(&self) -> Vec<ScreenDescriptor> {
        self.bootstrap.screens.clone()
    }
}

#[must_use]
pub fn screen_catalog(debug_tools_enabled: bool) -> Vec<ScreenDescriptor> {
    let mut screens = vec![
        ScreenDescriptor {
            id: "home",
            title: "Home",
            route: "/",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "host-tournament-setup",
            title: "Host Tournament Setup",
            route: "/host",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "join-tournament",
            title: "Join Tournament",
            route: "/join",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "tournament-lobby",
            title: "Tournament Lobby",
            route: "/lobby",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "ready-room",
            title: "Ready Room",
            route: "/ready-room",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "main-table",
            title: "Main Table",
            route: "/table",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "hand-history",
            title: "Hand History",
            route: "/history",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "tournament-complete",
            title: "Tournament Complete",
            route: "/complete",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "rules-help",
            title: "Rules / Help",
            route: "/rules",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "reconnect-errors",
            title: "Reconnect / Errors",
            route: "/errors",
            surface: "dialog",
        },
    ];

    if debug_tools_enabled {
        screens.push(ScreenDescriptor {
            id: "debug-tools",
            title: "Debug / Internal Tools",
            route: "/debug",
            surface: "panel",
        });
    }

    screens
}

fn backend_modules() -> Vec<ModuleDescriptor> {
    vec![
        app_state_descriptor(),
        domain::descriptor(),
        engine::descriptor(),
        tournament::descriptor(),
        protocol::descriptor(),
        networking::descriptor(),
        crypto::descriptor(),
        storage::descriptor(),
        interop::descriptor(),
    ]
}

fn app_state_descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "app_state",
        responsibility:
            "Detects launch context, profile namespace, backend module map, and screen catalog for the frontend bootstrap payload.",
    }
}

fn detect_instance_id() -> String {
    parse_arg_value(INSTANCE_ID_ARG)
        .or_else(|| env::var(INSTANCE_ID_ENV_VAR).ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "default".to_string())
}

fn detect_launch_join_payload() -> Option<String> {
    parse_arg_value(JOIN_PAYLOAD_ARG)
        .or_else(|| env::var(JOIN_PAYLOAD_ENV_VAR).ok())
        .filter(|value| !value.trim().is_empty())
}

fn parse_arg_value(flag: &str) -> Option<String> {
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        if argument == flag {
            return args.next();
        }

        if let Some(value) = argument.strip_prefix(&format!("{flag}=")) {
            return Some(value.to_string());
        }
    }

    None
}

fn detect_profile_directory(instance_id: &str) -> PathBuf {
    let base = data_local_dir()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    base.join("desktop-poker")
        .join("profiles")
        .join(instance_id)
}

#[cfg(test)]
mod tests {
    use super::{
        detect_profile_directory, screen_catalog, DesktopAppState, INSTANCE_ID_ENV_VAR,
        JOIN_PAYLOAD_ENV_VAR,
    };

    #[test]
    fn detect_uses_android_compatible_defaults() {
        std::env::remove_var(INSTANCE_ID_ENV_VAR);
        std::env::remove_var(JOIN_PAYLOAD_ENV_VAR);

        let state = DesktopAppState::detect().bootstrap();

        assert_eq!(state.protocol_version, 1);
        assert_eq!(state.default_host_port, 43_818);
        assert_eq!(state.instance_id, "default");
        assert!(state
            .profile_directory
            .ends_with("desktop-poker/profiles/default"));
    }

    #[test]
    fn screen_catalog_hides_debug_tools_in_release_mode() {
        let screens = screen_catalog(false);
        assert!(!screens.iter().any(|screen| screen.id == "debug-tools"));
    }

    #[test]
    fn screen_catalog_exposes_debug_tools_when_enabled() {
        let screens = screen_catalog(true);
        assert!(screens.iter().any(|screen| screen.id == "debug-tools"));
    }

    #[test]
    fn profile_directory_is_namespaced_by_instance() {
        let profile_directory = detect_profile_directory("client-b");
        assert!(profile_directory.ends_with("desktop-poker/profiles/client-b"));
    }
}
