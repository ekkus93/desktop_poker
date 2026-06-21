use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{domain, networking, tournament::TournamentController};

#[cfg(test)]
use crate::tournament::ActionRequest;

mod app;
mod app_npc;
mod config;
mod debug;
mod instance;
mod live_events;
mod projection;
mod session;

// Re-export the moved free functions so the rest of `app_state` keeps the
// original flat namespace.
pub(crate) use config::*;
pub(crate) use debug::*;
pub(crate) use instance::*;
pub(crate) use live_events::*;
pub(crate) use projection::*;

pub const INSTANCE_ID_ENV_VAR: &str = "DESKTOP_POKER_INSTANCE_ID";
pub const JOIN_PAYLOAD_ENV_VAR: &str = "DESKTOP_POKER_JOIN_PAYLOAD";
const INSTANCE_ID_ARG: &str = "--instance-id";
const JOIN_PAYLOAD_ARG: &str = "--join-payload";
const LOCAL_PLAYER_ID: &str = "local-player";
const RESERVED_PLAYER_ID: &str = "reserved-player";
const DEFAULT_INSTANCE_LABEL: &str = "default";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstanceProfile {
    instance_label: String,
    profile_id: String,
    storage_namespace: String,
    session_identity: String,
    reconnect_namespace: String,
    profile_directory: PathBuf,
}

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
    pub instance_label: String,
    pub storage_namespace: String,
    pub session_identity: String,
    pub reconnect_namespace: String,
    pub profile_directory: String,
    pub launch_join_payload: Option<String>,
    pub parsed_launch_join_payload: Option<domain::JoinPayload>,
    pub launch_join_payload_error: Option<String>,
    pub debug_tools_enabled: bool,
    /// True when a usable LLM provider config is present (any provider).
    pub llm_api_key_configured: bool,
    /// Which LLM provider is configured, e.g. "anthropic", "openAi", "ollama", "llamaServer".
    /// None when no provider is set.
    pub llm_provider_type: Option<String>,
    /// Populated when the provider config file exists but is corrupt or unreadable.
    /// Distinct from None (not configured) — means "configured but broken."
    pub provider_config_error: Option<String>,
    pub backend_modules: Vec<ModuleDescriptor>,
    pub screens: Vec<ScreenDescriptor>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TableViewerMode {
    Local,
    Observer,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DesktopTableActionKind {
    Fold,
    CheckOrCall,
    BetOrRaise,
    AllIn,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableCardView {
    pub label: String,
    pub compact_label: String,
    pub suit_symbol: &'static str,
    pub tone: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableSeatView {
    pub seat_index: u8,
    pub display_name: String,
    pub chip_count: Option<u32>,
    pub status_label: String,
    pub marker_label: Option<String>,
    pub contribution: u32,
    pub is_local: bool,
    pub is_acting: bool,
    pub is_observer: bool,
    pub is_eliminated: bool,
    pub is_compact: bool,
    pub cards_hidden: bool,
    pub hole_cards: Vec<TableCardView>,
    pub detail_lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableStandingView {
    pub rank: u8,
    pub display_name: String,
    pub chip_count: Option<u32>,
    pub status_label: String,
    pub note: Option<String>,
    pub is_local: bool,
    pub is_observer: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableHistoryEntryView {
    pub hand_number: u32,
    pub summary: String,
    pub pot_total: u32,
    pub winning_players: Vec<String>,
    pub eliminated_players: Vec<String>,
    pub board_cards: Vec<TableCardView>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableEventView {
    pub sequence: u64,
    pub kind: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableActionTrayView {
    pub owner_label: String,
    pub check_or_call_label: String,
    pub bet_or_raise_label: String,
    pub call_amount: u32,
    pub current_bet: u32,
    pub pot_total: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub deadline_epoch_ms: u64,
    pub legal_actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableViewSnapshot {
    pub viewer_mode: TableViewerMode,
    /// `"normal"` for a healthy session, `"reconnecting"` while TCP recovery
    /// is in progress, or `"terminated"` after a permanent disconnect.
    pub session_connection: String,
    pub tournament_name: String,
    pub table_name: String,
    pub table_id: String,
    /// Raw phase token: `"waitingForPlayers"`, `"readyCheck"`, `"running"`,
    /// `"complete"`, or `"cancelled"`. Use this for branching; `phase_label`
    /// is the human-readable equivalent.
    pub tournament_phase: String,
    pub phase_label: String,
    pub street_label: String,
    pub blind_level_label: String,
    pub current_hand_number: Option<u32>,
    pub board_cards: Vec<TableCardView>,
    pub pot_total: u32,
    pub action_owner_label: String,
    pub elimination_summary: String,
    pub observer_banner: Option<String>,
    pub seats: Vec<TableSeatView>,
    pub standings: Vec<TableStandingView>,
    pub hand_history: Vec<TableHistoryEntryView>,
    pub event_feed: Vec<TableEventView>,
    pub action_tray: Option<TableActionTrayView>,
}

/// Reason category for a recorded NPC action failure.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NpcActionErrorReason {
    Rejected,
    StaleWindow,
    RuntimeUnavailable,
    NoConfig,
    ProviderStateUnavailable,
    InvalidAction,
    InternalError,
}

/// Structured NPC action failure record for the debug inspector.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NpcActionErrorDebug {
    pub player_id: Option<String>,
    pub action: Option<String>,
    pub reason: NpcActionErrorReason,
    pub message: String,
    pub hand_number: Option<u32>,
    /// Monotonically increasing per-runner error counter.
    pub sequence: u64,
    /// True if the action was submitted to the host before the failure.
    pub submitted: bool,
    pub occurred_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebugInspectorState {
    pub protocol_log: Vec<TableEventView>,
    pub snapshot_json: String,
    pub current_sequence: u64,
    pub current_hand_number: Option<u32>,
    pub action_window_summary: Option<String>,
    pub launch_hint: String,
    /// Maps NPC player_id → tilt level string ("none", "mild", "full").
    /// Only populated when debug tools are enabled and a host session is active.
    pub npc_tilt_levels: std::collections::BTreeMap<String, String>,
    /// Most recent LLM fallback event for any NPC, if any occurred this session.
    /// Format: "<player_id>: <reason>". Cleared when a new game starts.
    pub last_llm_fallback: Option<String>,
    /// Most recent NPC action failure, structured for debug rendering.
    pub last_npc_action_error: Option<NpcActionErrorDebug>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartHostSessionRequest {
    pub host_address: String,
    pub host_port: u16,
    pub tournament_name: String,
    pub max_players: u8,
    pub starting_stack: u32,
    pub blind_preset_id: String,
    pub turn_timer_seconds: u32,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostSessionParticipantView {
    pub player_id: String,
    pub display_name: String,
    pub seat_index: Option<u8>,
    pub is_host: bool,
    pub is_ready: bool,
    pub connection_state: String,
    pub participant_state: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostSessionStatus {
    pub tournament_name: String,
    pub table_name: String,
    pub table_id: String,
    pub session_epoch: u64,
    pub advertised_host: String,
    pub host_port: u16,
    pub invite: String,
    pub phase: String,
    pub active_seat_count: u8,
    pub open_seat_count: u8,
    pub participants: Vec<HostSessionParticipantView>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JoinHostSessionRequest {
    pub join_payload: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimLobbySeatRequest {
    pub seat_index: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SetLobbyReadyStateRequest {
    pub is_ready: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientSessionStatus {
    pub tournament_name: String,
    pub table_name: String,
    pub table_id: String,
    pub session_epoch: u64,
    pub host_address: String,
    pub host_port: u16,
    pub local_player_id: String,
    pub phase: String,
    pub active_seat_count: u8,
    pub open_seat_count: u8,
    pub reconnecting: bool,
    /// `true` after the runtime emits `Disconnected` — the session cannot
    /// recover without a fresh join.
    pub terminated: bool,
    pub last_error: Option<String>,
    pub participants: Vec<HostSessionParticipantView>,
}

struct DesktopHostSession {
    host_server: Arc<networking::HostServer>,
    config: domain::TournamentConfig,
    advertised_host: String,
    npc_runner: Option<crate::npc::runner::NpcRunnerGuard>,
}

struct DesktopClientSession {
    runtime: networking::ClientRuntime,
    join_payload: domain::JoinPayload,
    latest_snapshot: domain::SnapshotState,
    reconnecting: bool,
    /// Set when the background runtime emits `Disconnected` — the session
    /// cannot recover without rejoining.
    terminated: bool,
    last_error: Option<String>,
    event_feed: Vec<TableEventView>,
}

pub struct DesktopAppState {
    bootstrap: DesktopBootstrapState,
    app_data_dir: PathBuf,
    llm_provider: Arc<Mutex<Option<crate::npc::LlmProviderConfig>>>,
    /// Live provider config error — cleared when provider is successfully saved or cleared.
    /// Takes precedence over the startup-time snapshot in `bootstrap.provider_config_error`.
    live_provider_config_error: Mutex<Option<String>>,
    debug_table_runtime: Mutex<Option<DebugTableRuntime>>,
    host_session: Mutex<Option<DesktopHostSession>>,
    client_session: Mutex<Option<DesktopClientSession>>,
    launched_instances: Mutex<u32>,
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(1)
}

struct DebugTableRuntime {
    controller: TournamentController,
    protocol_log: Vec<TableEventView>,
    #[cfg(test)]
    now_ms: u64,
    next_sequence: u64,
    last_board_count: usize,
    last_hand_result_count: usize,
    last_action_window_id: Option<String>,
    last_hand_number: Option<u32>,
}

#[cfg(test)]
mod tests;
