use std::collections::{BTreeMap, BTreeSet};

use desktop_poker_lib::{
    app_state::{
        ClientSessionStatus, DebugInspectorState, DesktopBootstrapState,
        HostSessionParticipantView, HostSessionStatus, ModuleDescriptor, ScreenDescriptor,
        TableViewSnapshot, TableViewerMode,
    },
    networking::HostRuntimeHealth,
    npc::{
        profile_store::NpcProfileListResult, LlmProviderConfig, LlmProviderSettings,
        LlmProviderType,
    },
};
use serde::Serialize;
use serde_json::Value;

fn contract() -> Value {
    serde_json::from_str(include_str!("../../src/fixtures/desktop-contract.json"))
        .expect("desktop DTO contract fixture must be valid JSON")
}

fn assert_contract_keys(name: &str, value: &impl Serialize) {
    let actual = serde_json::to_value(value)
        .expect("DTO should serialize")
        .as_object()
        .expect("DTO should serialize as an object")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected = contract()[name]
        .as_array()
        .expect("contract entry must be a key array")
        .iter()
        .map(|key| {
            key.as_str()
                .expect("contract key must be a string")
                .to_string()
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected, "serialized key drift for {name}");
}

fn participant() -> HostSessionParticipantView {
    HostSessionParticipantView {
        player_id: "player-a".to_string(),
        display_name: "Alice".to_string(),
        seat_index: Some(0),
        is_host: false,
        is_ready: true,
        connection_state: "connected".to_string(),
        participant_state: "seated".to_string(),
    }
}

#[test]
fn rust_serialization_matches_desktop_contract_fixture() {
    assert_contract_keys(
        "DesktopBootstrapState",
        &DesktopBootstrapState {
            app_name: "Desktop Poker",
            protocol_version: 1,
            default_host_port: 43_818,
            frontend_stack: "React + TypeScript",
            serialization_strategy: "serde",
            framing_strategy: "length-prefixed JSON",
            join_payload_encoding: "pkr1_",
            runtime_transport: "raw TCP over LAN",
            crypto_stack: vec!["ed25519", "x25519"],
            instance_id: "instance-a".to_string(),
            instance_label: "Instance A".to_string(),
            storage_namespace: "desktop-poker:instance-a".to_string(),
            session_identity: "desktop-session:instance-a".to_string(),
            reconnect_namespace: "desktop-reconnect:instance-a".to_string(),
            profile_directory: "/tmp/profile".to_string(),
            launch_join_payload: None,
            parsed_launch_join_payload: None,
            launch_join_payload_error: None,
            debug_tools_enabled: true,
            llm_api_key_configured: false,
            llm_provider_type: None,
            provider_config_error: None,
            backend_modules: vec![ModuleDescriptor {
                name: "protocol",
                responsibility: "Owns protocol behavior.",
            }],
            screens: vec![ScreenDescriptor {
                id: "home",
                title: "Home",
                route: "/",
                surface: "home",
            }],
        },
    );

    assert_contract_keys(
        "HostSessionStatus",
        &HostSessionStatus {
            tournament_name: "Friday Night".to_string(),
            table_name: "Main Table".to_string(),
            table_id: "table-1".to_string(),
            session_epoch: 1,
            advertised_host: "192.168.1.10".to_string(),
            host_port: 43_818,
            invite: "pkr1_fixture".to_string(),
            phase: "readyCheck".to_string(),
            active_seat_count: 1,
            open_seat_count: 1,
            participants: vec![participant()],
        },
    );

    assert_contract_keys(
        "ClientSessionStatus",
        &ClientSessionStatus {
            tournament_name: "Friday Night".to_string(),
            table_name: "Main Table".to_string(),
            table_id: "table-1".to_string(),
            session_epoch: 1,
            host_address: "192.168.1.10".to_string(),
            host_port: 43_818,
            local_player_id: "player-a".to_string(),
            phase: "readyCheck".to_string(),
            active_seat_count: 1,
            open_seat_count: 1,
            reconnecting: false,
            terminated: false,
            last_error: None,
            participants: vec![participant()],
        },
    );

    assert_contract_keys(
        "TableViewSnapshot",
        &TableViewSnapshot {
            viewer_mode: TableViewerMode::Local,
            session_connection: "normal".to_string(),
            tournament_name: "Friday Night".to_string(),
            table_name: "Main Table".to_string(),
            table_id: "table-1".to_string(),
            tournament_phase: "running".to_string(),
            phase_label: "Running".to_string(),
            street_label: "Flop".to_string(),
            blind_level_label: "10 / 20".to_string(),
            current_hand_number: Some(1),
            board_cards: vec![],
            pot_total: 30,
            action_owner_label: "Alice".to_string(),
            elimination_summary: String::new(),
            observer_banner: None,
            seats: vec![],
            standings: vec![],
            hand_history: vec![],
            event_feed: vec![],
            action_tray: None,
        },
    );

    assert_contract_keys(
        "DebugInspectorState",
        &DebugInspectorState {
            protocol_log: vec![],
            snapshot_json: "{}".to_string(),
            current_sequence: 0,
            current_hand_number: None,
            action_window_summary: None,
            launch_hint: "hint".to_string(),
            npc_tilt_levels: BTreeMap::new(),
            last_llm_fallback: None,
            last_npc_action_error: None,
            host_runtime_health: None,
        },
    );

    assert_contract_keys("HostRuntimeHealth", &HostRuntimeHealth::default());

    assert_contract_keys(
        "NpcProfileListResult",
        &NpcProfileListResult {
            profiles: vec![],
            errors: vec![],
        },
    );

    let settings = LlmProviderSettings {
        provider: LlmProviderType::EmbeddedLocal,
        endpoint_url: None,
        model: Some("/tmp/model.gguf".to_string()),
    };
    assert_contract_keys("LlmProviderSettings", &settings);
    assert_contract_keys(
        "LlmProviderConfig",
        &LlmProviderConfig {
            settings,
            api_key: None,
        },
    );
}
