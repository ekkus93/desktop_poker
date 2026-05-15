use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use dirs::data_local_dir;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::{
    crypto::{self, ProtocolCryptoProvider},
    domain, engine, interop, networking, protocol, storage, tournament,
    tournament::{RegisteredPlayer, TournamentController},
};

#[cfg(test)]
use crate::tournament::ActionRequest;

pub const INSTANCE_ID_ENV_VAR: &str = "DESKTOP_POKER_INSTANCE_ID";
pub const JOIN_PAYLOAD_ENV_VAR: &str = "DESKTOP_POKER_JOIN_PAYLOAD";
const INSTANCE_ID_ARG: &str = "--instance-id";
const JOIN_PAYLOAD_ARG: &str = "--join-payload";
const LOCAL_PLAYER_ID: &str = "local-player";
const RESERVED_PLAYER_ID: &str = "reserved-player";
const DEFAULT_INSTANCE_LABEL: &str = "default";

#[derive(Clone, Debug, PartialEq, Eq)]
struct InstanceProfile {
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
    pub tournament_name: String,
    pub table_name: String,
    pub table_id: String,
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

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebugInspectorState {
    pub protocol_log: Vec<TableEventView>,
    pub snapshot_json: String,
    pub current_sequence: u64,
    pub current_hand_number: Option<u32>,
    pub action_window_summary: Option<String>,
    pub launch_hint: String,
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
    pub last_error: Option<String>,
    pub participants: Vec<HostSessionParticipantView>,
}

struct DesktopHostSession {
    host_server: networking::HostServer,
    config: domain::TournamentConfig,
    advertised_host: String,
}

impl DesktopHostSession {
    fn status(&self) -> Result<HostSessionStatus, String> {
        let authoritative_state = self
            .host_server
            .authoritative_state()
            .map_err(|error| error.to_string())?;
        let active_seat_count = active_seat_count_for_state(&authoritative_state);
        let participants = build_session_participants(&authoritative_state);

        Ok(HostSessionStatus {
            tournament_name: self.config.tournament_name.clone(),
            table_name: self
                .config
                .table_name
                .clone()
                .unwrap_or_else(|| "Main Table".to_string()),
            table_id: authoritative_state.table_id,
            session_epoch: authoritative_state.session_epoch,
            advertised_host: self.advertised_host.clone(),
            host_port: self.host_server.listener_addr().port(),
            invite: self.host_server.encoded_join_payload().to_string(),
            phase: format_tournament_phase_value(authoritative_state.phase),
            active_seat_count,
            open_seat_count: self.config.max_players.saturating_sub(active_seat_count),
            participants,
        })
    }

    fn table_view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        let authoritative_state = self
            .host_server
            .authoritative_state()
            .map_err(|error| error.to_string())?;
        let event_feed = build_live_event_feed(
            &self
                .host_server
                .public_events()
                .map_err(|error| error.to_string())?,
            &authoritative_state,
        );
        build_table_view_snapshot(
            &authoritative_state,
            LOCAL_PLAYER_ID,
            viewer_mode,
            true,
            event_feed,
        )
    }

    fn submit_table_action(
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
    }
}

struct DesktopClientSession {
    runtime: networking::ClientRuntime,
    join_payload: domain::JoinPayload,
    latest_snapshot: domain::SnapshotState,
    reconnecting: bool,
    last_error: Option<String>,
    event_feed: Vec<TableEventView>,
}

impl DesktopClientSession {
    fn status(&mut self) -> ClientSessionStatus {
        self.refresh();

        let authoritative_state = &self.latest_snapshot.state;
        let active_seat_count = active_seat_count_for_state(authoritative_state);

        ClientSessionStatus {
            tournament_name: authoritative_state.config.tournament_name.clone(),
            table_name: authoritative_state
                .config
                .table_name
                .clone()
                .unwrap_or_else(|| "Main Table".to_string()),
            table_id: authoritative_state.table_id.clone(),
            session_epoch: authoritative_state.session_epoch,
            host_address: self.join_payload.host_address.clone(),
            host_port: self.join_payload.host_port,
            local_player_id: self.latest_snapshot.local_player_id.clone(),
            phase: format_tournament_phase_value(authoritative_state.phase),
            active_seat_count,
            open_seat_count: authoritative_state
                .config
                .max_players
                .saturating_sub(active_seat_count),
            reconnecting: self.reconnecting,
            last_error: self.last_error.clone(),
            participants: build_session_participants(authoritative_state),
        }
    }

    fn table_view(&mut self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        self.refresh();
        if self.latest_snapshot.state.phase == domain::TournamentPhase::ReadyCheck {
            self.await_condition(Duration::from_millis(250), |session| {
                session.last_error.is_some()
                    || session.latest_snapshot.state.phase != domain::TournamentPhase::ReadyCheck
            });
        }

        build_table_view_snapshot(
            &self.latest_snapshot.state,
            &self.latest_snapshot.local_player_id,
            viewer_mode,
            true,
            self.event_feed.clone(),
        )
    }

    fn submit_table_action(
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
        self.await_condition(Duration::from_secs(1), |session| {
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

        self.table_view(viewer_mode)
    }

    fn refresh(&mut self) {
        loop {
            let next_event = self.runtime.next_event(Duration::from_millis(1));
            let event = match next_event {
                Ok(event) => event,
                Err(_) => break,
            };

            self.apply_event(event);
        }
    }

    fn apply_event(&mut self, event: networking::ClientRuntimeEvent) {
        match event {
            networking::ClientRuntimeEvent::Snapshot(snapshot) => {
                self.latest_snapshot = client_snapshot_state_from_event(&snapshot);
                self.reconnecting = false;
                self.last_error = None;
            }
            networking::ClientRuntimeEvent::PublicEvent {
                message_type,
                server_sequence,
                payload,
            } => {
                apply_public_event_to_snapshot(
                    &mut self.latest_snapshot.state,
                    &self.latest_snapshot.local_player_id,
                    message_type,
                    &payload,
                );
                push_live_event(
                    &mut self.event_feed,
                    server_sequence,
                    message_type,
                    &payload,
                    &self.latest_snapshot.state,
                );
                self.reconnecting = false;
                self.last_error = None;
            }
            networking::ClientRuntimeEvent::PrivateHoleCards(private_hole_cards) => {
                apply_private_hole_cards_to_snapshot(
                    &mut self.latest_snapshot.state,
                    &private_hole_cards,
                );
                self.reconnecting = false;
                self.last_error = None;
            }
            networking::ClientRuntimeEvent::Reconnecting { .. } => {
                self.reconnecting = true;
            }
            networking::ClientRuntimeEvent::ResyncRequested { .. } => {
                self.reconnecting = true;
            }
            networking::ClientRuntimeEvent::SafeError { message, .. } => {
                self.reconnecting = false;
                self.last_error = Some(message);
            }
            networking::ClientRuntimeEvent::Disconnected { .. } => {
                self.reconnecting = false;
                self.last_error = Some("Disconnected from host".to_string());
            }
        }
    }

    fn claim_lobby_seat(
        &mut self,
        request: ClaimLobbySeatRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.last_error = None;
        self.runtime
            .claim_seat(request.seat_index)
            .map_err(|error| error.to_string())?;
        self.await_condition(Duration::from_secs(1), |session| {
            session.last_error.is_some()
                || session
                    .latest_snapshot
                    .state
                    .participants
                    .get(&session.latest_snapshot.local_player_id)
                    .and_then(|participant| participant.seat_index)
                    == Some(request.seat_index)
        });

        if let Some(error) = self.last_error.clone() {
            return Err(error);
        }

        Ok(self.status())
    }

    fn set_lobby_ready_state(
        &mut self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.last_error = None;
        self.runtime
            .set_ready_state(request.is_ready)
            .map_err(|error| error.to_string())?;
        self.await_condition(Duration::from_secs(1), |session| {
            session.last_error.is_some()
                || session
                    .latest_snapshot
                    .state
                    .participants
                    .get(&session.latest_snapshot.local_player_id)
                    .and_then(|participant| participant.seat_index)
                    .and_then(|seat_index| {
                        session
                            .latest_snapshot
                            .state
                            .seats
                            .get(seat_index as usize)
                            .map(|seat| seat.is_ready)
                    })
                    == Some(request.is_ready)
        });

        if let Some(error) = self.last_error.clone() {
            return Err(error);
        }

        Ok(self.status())
    }

    fn await_condition(&mut self, timeout: Duration, predicate: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            self.refresh();
            if predicate(self) {
                return;
            }

            if let Ok(event) = self.runtime.next_event(Duration::from_millis(50)) {
                self.apply_event(event);
            }

            if predicate(self) {
                return;
            }
        }
    }
}

pub struct DesktopAppState {
    bootstrap: DesktopBootstrapState,
    debug_table_runtime: Mutex<Option<DebugTableRuntime>>,
    host_session: Mutex<Option<DesktopHostSession>>,
    client_session: Mutex<Option<DesktopClientSession>>,
    launched_instances: Mutex<u32>,
}

impl DesktopAppState {
    #[must_use]
    pub fn detect() -> Self {
        let instance_profile = detect_instance_profile();
        let debug_tools_enabled = cfg!(debug_assertions);
        let launch_join_payload = detect_launch_join_payload();
        let (parsed_launch_join_payload, launch_join_payload_error) =
            parse_launch_join_payload(launch_join_payload.as_deref());

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
                instance_id: instance_profile.profile_id.clone(),
                instance_label: instance_profile.instance_label,
                storage_namespace: instance_profile.storage_namespace,
                session_identity: instance_profile.session_identity,
                reconnect_namespace: instance_profile.reconnect_namespace,
                profile_directory: instance_profile.profile_directory.display().to_string(),
                launch_join_payload,
                parsed_launch_join_payload,
                launch_join_payload_error,
                debug_tools_enabled,
                backend_modules: backend_modules(),
                screens: screen_catalog(debug_tools_enabled),
            },
            debug_table_runtime: Mutex::new(None),
            host_session: Mutex::new(None),
            client_session: Mutex::new(None),
            launched_instances: Mutex::new(0),
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

    pub fn table_view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        if let Some(table_view) = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(|session| session.table_view(viewer_mode))
            .transpose()?
        {
            return Ok(table_view);
        }

        if let Some(table_view) = self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(|session| session.table_view(viewer_mode))
            .transpose()?
        {
            return Ok(table_view);
        }

        let _ = viewer_mode;
        Err("no active live session is available for the table view".to_string())
    }

    pub fn submit_table_action(
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
    }

    pub fn debug_state(&self, viewer_mode: TableViewerMode) -> Result<DebugInspectorState, String> {
        if !self.bootstrap.debug_tools_enabled {
            return Err("debug tools are unavailable in release builds".to_string());
        }

        let mut debug_table_runtime = self
            .debug_table_runtime
            .lock()
            .map_err(|_| "debug table runtime lock poisoned".to_string())?;
        debug_table_runtime
            .get_or_insert_with(|| {
                DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .debug_state(viewer_mode)
    }

    pub fn start_host_session(
        &self,
        request: StartHostSessionRequest,
    ) -> Result<HostSessionStatus, String> {
        self.start_host_session_with_mode(request, networking::HostRuntimeMode::Production)
    }

    pub fn host_session_status(&self) -> Result<Option<HostSessionStatus>, String> {
        self.host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .as_ref()
            .map(DesktopHostSession::status)
            .transpose()
    }

    pub fn stop_host_session(&self) -> Result<(), String> {
        self.host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .take();
        Ok(())
    }

    pub fn host_claim_lobby_seat(
        &self,
        request: ClaimLobbySeatRequest,
    ) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .claim_seat(LOCAL_PLAYER_ID, request.seat_index)
            .map_err(|error| error.to_string())?;
        session.status()
    }

    pub fn host_set_lobby_ready_state(
        &self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .set_ready_state(LOCAL_PLAYER_ID, request.is_ready)
            .map_err(|error| error.to_string())?;
        session.status()
    }

    pub fn host_start_tournament(&self) -> Result<HostSessionStatus, String> {
        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        let session = host_session
            .as_mut()
            .ok_or_else(|| "no active host session".to_string())?;
        session
            .host_server
            .start_tournament()
            .map_err(|error| error.to_string())?;
        session.status()
    }

    pub fn join_host_session(
        &self,
        request: JoinHostSessionRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.join_host_session_with_player_id(
            request,
            format!("player-{}", self.bootstrap.instance_id),
        )
    }

    pub fn client_session_status(&self) -> Result<Option<ClientSessionStatus>, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .map(DesktopClientSession::status)
            .map(Ok)
            .transpose()
    }

    pub fn leave_client_session(&self) -> Result<(), String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .take();
        Ok(())
    }

    pub fn client_claim_lobby_seat(
        &self,
        request: ClaimLobbySeatRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .ok_or_else(|| "no active client session".to_string())?
            .claim_lobby_seat(request)
    }

    pub fn client_set_lobby_ready_state(
        &self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .as_mut()
            .ok_or_else(|| "no active client session".to_string())?
            .set_lobby_ready_state(request)
    }

    pub fn launch_additional_client_instance(
        &self,
        join_payload: Option<String>,
    ) -> Result<String, String> {
        if !cfg!(debug_assertions) {
            return Err("debug launch helper is only available in debug builds".to_string());
        }

        let mut launch_counter = self
            .launched_instances
            .lock()
            .map_err(|_| "launch counter lock poisoned".to_string())?;
        *launch_counter += 1;
        let (instance_id, launch_args) = build_debug_child_launch_args(
            &self.bootstrap.instance_id,
            std::process::id(),
            *launch_counter,
            join_payload.as_deref(),
        );
        let current_executable = env::current_exe().map_err(|error| error.to_string())?;
        let current_directory = env::current_dir().map_err(|error| error.to_string())?;

        let mut command = Command::new(current_executable);
        command.current_dir(current_directory);
        for arg in launch_args {
            command.arg(arg);
        }
        command.spawn().map_err(|error| error.to_string())?;

        Ok(instance_id)
    }

    fn start_host_session_with_mode(
        &self,
        request: StartHostSessionRequest,
        runtime_mode: networking::HostRuntimeMode,
    ) -> Result<HostSessionStatus, String> {
        let host_address = request.host_address.trim();
        let tournament_name = request.tournament_name.trim();
        let display_name = request.display_name.trim();

        if host_address.is_empty() {
            return Err("hostAddress must be non-blank".to_string());
        }

        if self
            .client_session
            .lock()
            .map_err(|_| "client session lock poisoned".to_string())?
            .is_some()
        {
            return Err("leave the active client session before hosting".to_string());
        }

        if self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?
            .is_some()
        {
            return Err("stop the active host session before starting a new table".to_string());
        }

        if tournament_name.is_empty() {
            return Err("tournamentName must be non-blank".to_string());
        }

        if display_name.is_empty() {
            return Err("displayName must be non-blank".to_string());
        }

        if !(2..=10).contains(&request.max_players) {
            return Err("maxPlayers must be between 2 and 10".to_string());
        }

        if request.starting_stack == 0 {
            return Err("startingStack must be greater than zero".to_string());
        }

        if request.turn_timer_seconds == 0 {
            return Err("turnTimerSeconds must be greater than zero".to_string());
        }

        let provider = crypto::DefaultCryptoProvider;
        let host_signing_keys = Arc::new(provider.generate_signing_keypair());
        let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
        let session_epoch = now_epoch_ms().max(1);
        let table_id = format!("table-{}-{session_epoch}", self.bootstrap.instance_id);
        let config = build_host_tournament_config(&request)?;
        let snapshot_state = build_initial_host_state(
            &config,
            display_name,
            &table_id,
            session_epoch,
            &host_signing_keys,
            &host_encryption_keys,
        )?;
        let host_server = networking::HostServer::bind(networking::HostRuntimeConfig {
            bind_addr: format!("0.0.0.0:{}", request.host_port)
                .parse()
                .map_err(|error| format!("invalid host port: {error}"))?,
            advertised_host: host_address.to_string(),
            session_epoch,
            table_id,
            table_name: Some(config.tournament_name.clone()),
            join_token: issue_join_token(),
            host_signing_keys,
            host_encryption_keys,
            snapshot_state,
            runtime_mode,
        })
        .map_err(|error| error.to_string())?;

        let mut host_session = self
            .host_session
            .lock()
            .map_err(|_| "host session lock poisoned".to_string())?;
        *host_session = Some(DesktopHostSession {
            host_server,
            config,
            advertised_host: host_address.to_string(),
        });

        host_session
            .as_ref()
            .expect("host session was just inserted")
            .status()
    }

    fn join_host_session_with_player_id(
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

        let latest_snapshot = match runtime.next_event(Duration::from_secs(1)) {
            Ok(networking::ClientRuntimeEvent::Snapshot(snapshot)) => {
                client_snapshot_state_from_event(&snapshot)
            }
            Ok(other) => {
                return Err(format!(
                    "expected an initial snapshot event after join, got {other:?}"
                ));
            }
            Err(error) => return Err(error.to_string()),
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
            last_error: None,
            event_feed: Vec::new(),
        });

        Ok(client_session
            .as_mut()
            .expect("client session was just inserted")
            .status())
    }
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(1)
}

fn client_snapshot_state_from_event(event: &protocol::SnapshotEvent) -> domain::SnapshotState {
    let state = &event.state;
    let occupied_seat_links = state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == domain::SeatOccupancyState::Occupied)
        .filter_map(|seat| {
            seat.participant_id
                .as_ref()
                .map(|participant_id| (seat.seat_index, participant_id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let participants = state
        .participants
        .values()
        .map(|participant| {
            let normalized_seat_index = participant.seat_index.and_then(|seat_index| {
                (occupied_seat_links.get(&seat_index) == Some(&participant.player_id))
                    .then_some(seat_index)
            });
            (
                participant.player_id.clone(),
                domain::ParticipantRegistryEntry {
                    identity: domain::PlayerIdentity {
                        player_id: participant.player_id.clone(),
                        display_name: participant.display_name.clone(),
                        signing_public_key: format!("snapshot-sign-{}", participant.player_id),
                        encryption_public_key: format!("snapshot-enc-{}", participant.player_id),
                        signing_key_fingerprint: format!("snapshot-fp-{}", participant.player_id),
                    },
                    state: participant.participant_state,
                    connection_state: participant.connection_state,
                    seat_index: normalized_seat_index,
                    admitted_at_ms: state.session_epoch,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: participant.is_host,
                },
            )
        })
        .collect();
    let current_hand = state.current_hand.as_ref().map(|hand| {
        let mut hole_cards_by_player_id = hand.public_hole_cards_by_player_id.clone();
        if !event.private_hole_cards.is_empty() {
            hole_cards_by_player_id.insert(
                event.local_player_id.clone(),
                event.private_hole_cards.clone(),
            );
        }

        domain::HandState {
            hand_number: hand.hand_number,
            cycle_phase: hand.cycle_phase,
            street: hand.street,
            dealer_seat_index: hand.dealer_seat_index,
            small_blind_seat_index: hand.small_blind_seat_index,
            big_blind_seat_index: hand.big_blind_seat_index,
            board_cards: hand.board_cards.clone(),
            hole_cards_by_player_id,
            participation_by_player_id: hand.participation_by_player_id.clone(),
            betting_round: hand.betting_round.clone(),
            action_window: hand.action_window.clone(),
        }
    });

    domain::SnapshotState {
        state: domain::TournamentState {
            table_id: state.table_id.clone(),
            session_epoch: state.session_epoch,
            phase: state.phase,
            config: state.config.clone(),
            blind_schedule: state.blind_schedule.clone(),
            blind_level_index: state.blind_level_index,
            participants,
            seats: state.seats.clone(),
            current_hand,
            hand_results: state.hand_results.clone(),
            placements: state.placements.clone(),
        },
        local_player_id: event.local_player_id.clone(),
        reconnect_token: event.reconnect_token.clone(),
        host_signing_public_key: event.host_signing_public_key.clone(),
        host_encryption_public_key: event.host_encryption_public_key.clone(),
    }
}

fn build_host_tournament_config(
    request: &StartHostSessionRequest,
) -> Result<domain::TournamentConfig, String> {
    let blind_schedule = blind_schedule_for_preset(&request.blind_preset_id)?;

    Ok(domain::TournamentConfig {
        tournament_name: request.tournament_name.trim().to_string(),
        table_name: Some("Main Table".to_string()),
        max_players: request.max_players,
        starting_stack: request.starting_stack,
        turn_timer_seconds: request.turn_timer_seconds,
        blind_schedule,
    })
}

fn blind_schedule_for_preset(blind_preset_id: &str) -> Result<domain::BlindSchedule, String> {
    let duration_seconds = match blind_preset_id.trim() {
        "fast" => 180,
        "normal" => 300,
        "slow" => 480,
        "turbo" => 180,
        "standard" => 300,
        "deep-stack" => 480,
        other => {
            return Err(format!("unsupported blindPresetId: {other}"));
        }
    };

    Ok(domain::BlindSchedule {
        levels: [
            (10, 20),
            (15, 30),
            (25, 50),
            (50, 100),
            (75, 150),
            (100, 200),
            (150, 300),
            (200, 400),
            (300, 600),
            (400, 800),
            (600, 1200),
            (800, 1600),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (small_blind, big_blind))| domain::BlindLevel {
            level_index: (index + 1) as u8,
            label: format!("Level {}", index + 1),
            small_blind,
            big_blind,
            ante: 0,
            duration_seconds,
        })
        .collect(),
    })
}

fn issue_join_token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn build_initial_host_state(
    config: &domain::TournamentConfig,
    display_name: &str,
    table_id: &str,
    session_epoch: u64,
    host_signing_keys: &Arc<crypto::SigningKeyMaterial>,
    host_encryption_keys: &Arc<Mutex<crypto::EncryptionKeyMaterial>>,
) -> Result<domain::TournamentState, String> {
    let host_encryption_public_key = host_encryption_keys
        .lock()
        .map_err(|_| "host encryption key lock poisoned".to_string())?
        .public_key_base64();

    let mut participants = BTreeMap::new();
    participants.insert(
        LOCAL_PLAYER_ID.to_string(),
        domain::ParticipantRegistryEntry {
            identity: domain::PlayerIdentity {
                player_id: LOCAL_PLAYER_ID.to_string(),
                display_name: display_name.to_string(),
                signing_public_key: host_signing_keys.public_key_base64(),
                encryption_public_key: host_encryption_public_key,
                signing_key_fingerprint: host_signing_keys.key_id(),
            },
            state: domain::ParticipantState::Seated,
            connection_state: domain::ConnectionState::Connected,
            seat_index: Some(0),
            admitted_at_ms: session_epoch,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: true,
        },
    );

    let seats = (0..config.max_players)
        .map(|seat_index| {
            if seat_index == 0 {
                domain::SeatState {
                    seat_index,
                    occupancy: domain::SeatOccupancyState::Occupied,
                    tournament_state: domain::TournamentSeatState::Lobby,
                    participant_id: Some(LOCAL_PLAYER_ID.to_string()),
                    display_name: Some(display_name.to_string()),
                    chip_count: None,
                    is_ready: false,
                    marker: None,
                }
            } else {
                domain::SeatState {
                    seat_index,
                    occupancy: domain::SeatOccupancyState::Empty,
                    tournament_state: domain::TournamentSeatState::Open,
                    participant_id: None,
                    display_name: None,
                    chip_count: None,
                    is_ready: false,
                    marker: None,
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(domain::TournamentState {
        table_id: table_id.to_string(),
        session_epoch,
        phase: domain::TournamentPhase::WaitingForPlayers,
        config: config.clone(),
        blind_schedule: config.blind_schedule.clone(),
        blind_level_index: 0,
        participants,
        seats,
        current_hand: None,
        hand_results: Vec::new(),
        placements: Vec::new(),
    })
}

fn format_connection_state_value(connection_state: domain::ConnectionState) -> String {
    match connection_state {
        domain::ConnectionState::Connected => "connected".to_string(),
        domain::ConnectionState::Disconnected => "disconnected".to_string(),
        domain::ConnectionState::Reconnecting => "reconnecting".to_string(),
    }
}

fn format_participant_state_value(participant_state: domain::ParticipantState) -> String {
    match participant_state {
        domain::ParticipantState::Admitted => "admitted".to_string(),
        domain::ParticipantState::Seated => "seated".to_string(),
        domain::ParticipantState::Active => "active".to_string(),
        domain::ParticipantState::Reconnecting => "reconnecting".to_string(),
        domain::ParticipantState::EliminatedObserver => "eliminatedObserver".to_string(),
        domain::ParticipantState::Removed => "removed".to_string(),
    }
}

fn format_tournament_phase_value(phase: domain::TournamentPhase) -> String {
    match phase {
        domain::TournamentPhase::WaitingForPlayers => "waitingForPlayers".to_string(),
        domain::TournamentPhase::ReadyCheck => "readyCheck".to_string(),
        domain::TournamentPhase::Running => "running".to_string(),
        domain::TournamentPhase::Complete => "complete".to_string(),
        domain::TournamentPhase::Cancelled => "cancelled".to_string(),
    }
}

fn active_seat_count_for_state(authoritative_state: &domain::TournamentState) -> u8 {
    authoritative_state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == domain::SeatOccupancyState::Occupied)
        .count() as u8
}

fn build_session_participants(
    authoritative_state: &domain::TournamentState,
) -> Vec<HostSessionParticipantView> {
    authoritative_state
        .participants
        .values()
        .map(|participant| HostSessionParticipantView {
            player_id: participant.identity.player_id.clone(),
            display_name: participant.identity.display_name.clone(),
            seat_index: participant.seat_index,
            is_host: participant.is_host,
            is_ready: participant
                .seat_index
                .and_then(|seat_index| authoritative_state.seats.get(seat_index as usize))
                .map(|seat| seat.is_ready)
                .unwrap_or(false),
            connection_state: format_connection_state_value(participant.connection_state),
            participant_state: format_participant_state_value(participant.state),
        })
        .collect()
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
            "Detects launch context, profile namespace, backend module map, screen catalog, and the desktop shell table runtime.",
    }
}

fn detect_instance_profile() -> InstanceProfile {
    derive_instance_profile(
        parse_arg_value(INSTANCE_ID_ARG)
            .or_else(|| env::var(INSTANCE_ID_ENV_VAR).ok())
            .as_deref(),
    )
}

fn detect_launch_join_payload() -> Option<String> {
    parse_arg_value(JOIN_PAYLOAD_ARG)
        .or_else(|| env::var(JOIN_PAYLOAD_ENV_VAR).ok())
        .filter(|value| !value.trim().is_empty())
}

fn parse_launch_join_payload(
    raw_payload: Option<&str>,
) -> (Option<domain::JoinPayload>, Option<String>) {
    match raw_payload {
        Some(payload) => match protocol::decode_join_payload(payload) {
            Ok(join_payload) => (Some(join_payload), None),
            Err(error) => (None, Some(error.to_string())),
        },
        None => (None, None),
    }
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

fn derive_instance_profile(raw_instance_label: Option<&str>) -> InstanceProfile {
    let instance_label = raw_instance_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_INSTANCE_LABEL)
        .to_string();
    let profile_id = sanitize_profile_id(&instance_label);

    InstanceProfile {
        instance_label,
        storage_namespace: format!("desktop-poker:{profile_id}"),
        session_identity: format!("desktop-session:{profile_id}"),
        reconnect_namespace: format!("desktop-reconnect:{profile_id}"),
        profile_directory: detect_profile_directory(&profile_id),
        profile_id,
    }
}

fn sanitize_profile_id(raw_instance_label: &str) -> String {
    let mut profile_id = String::new();
    let mut previous_was_separator = false;

    for character in raw_instance_label.trim().chars() {
        if character.is_ascii_alphanumeric() {
            profile_id.push(character.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if !previous_was_separator && !profile_id.is_empty() {
            profile_id.push('-');
            previous_was_separator = true;
        }
    }

    while profile_id.ends_with('-') {
        profile_id.pop();
    }

    if profile_id.is_empty() {
        DEFAULT_INSTANCE_LABEL.to_string()
    } else {
        profile_id
    }
}

fn build_debug_child_instance_id(
    parent_profile_id: &str,
    process_id: u32,
    launch_counter: u32,
) -> String {
    format!("{parent_profile_id}-p{process_id}-client-{launch_counter}")
}

fn build_debug_child_launch_args(
    parent_profile_id: &str,
    process_id: u32,
    launch_counter: u32,
    join_payload: Option<&str>,
) -> (String, Vec<String>) {
    let instance_id = build_debug_child_instance_id(parent_profile_id, process_id, launch_counter);
    let mut args = vec![INSTANCE_ID_ARG.to_string(), instance_id.clone()];

    if let Some(payload) = join_payload
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        args.push(JOIN_PAYLOAD_ARG.to_string());
        args.push(payload.to_string());
    }

    (instance_id, args)
}

fn detect_profile_directory(instance_id: &str) -> PathBuf {
    let base = data_local_dir()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    base.join("desktop-poker")
        .join("profiles")
        .join(instance_id)
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

impl DebugTableRuntime {
    fn new() -> Result<Self, String> {
        let controller = debug_demo_controller().map_err(|error| error.to_string())?;

        let mut runtime = Self {
            controller,
            protocol_log: Vec::new(),
            #[cfg(test)]
            now_ms: 1,
            next_sequence: 1,
            last_board_count: 0,
            last_hand_result_count: 0,
            last_action_window_id: None,
            last_hand_number: None,
        };
        runtime.log_event(
            "runtime",
            "Debug table runtime initialized from the Rust tournament controller.",
        );
        runtime.sync_log_markers();
        Ok(runtime)
    }

    fn view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        build_table_view_snapshot(
            self.controller.state(),
            LOCAL_PLAYER_ID,
            viewer_mode,
            true,
            self.protocol_log.iter().rev().take(10).cloned().collect(),
        )
    }

    #[cfg(test)]
    fn submit_action(
        &mut self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        if matches!(viewer_mode, TableViewerMode::Observer) {
            return Err("observer mode cannot submit actions".to_string());
        }

        let current_window = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
            .ok_or_else(|| "no open action window".to_string())?;

        if current_window.player_id != LOCAL_PLAYER_ID {
            return Err("action tray is disabled until the local player owns the turn".to_string());
        }

        let (action_type, action_amount, action_summary) =
            resolve_action_request(&current_window, action_kind, raise_to_amount)?;
        let action_description = match action_amount {
            Some(amount) => format!("You selected {action_summary} to {amount} chips."),
            None => format!("You selected {action_summary}."),
        };
        self.log_event("action", action_description);
        let now_ms = self.bump_clock(1_000);
        self.controller
            .submit_action(
                ActionRequest {
                    player_id: LOCAL_PLAYER_ID.to_string(),
                    action_window_id: current_window.action_window_id,
                    action_type,
                    raise_to_amount: action_amount,
                },
                now_ms,
            )
            .map_err(|error| error.to_string())?;
        self.sync_log_markers();
        self.auto_play_opponents()?;
        self.view(TableViewerMode::Local)
    }

    fn debug_state(&self, viewer_mode: TableViewerMode) -> Result<DebugInspectorState, String> {
        let snapshot = self.view(viewer_mode)?;
        let snapshot_json =
            serde_json::to_string_pretty(&snapshot).map_err(|error| error.to_string())?;
        let action_window_summary = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| {
                format!(
                    "{} · call {} · min {:?} · max {:?} · legal {}",
                    display_name_for_state(self.controller.state(), &window.player_id)
                        .unwrap_or_else(|| window.player_id.clone()),
                    window.call_amount,
                    window.min_raise_to,
                    window.max_raise_to,
                    window
                        .legal_actions
                        .iter()
                        .map(|action| format_action(*action))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });

        Ok(DebugInspectorState {
            protocol_log: self.protocol_log.iter().rev().take(30).cloned().collect(),
            snapshot_json,
            current_sequence: self.next_sequence.saturating_sub(1),
            current_hand_number: self
                .controller
                .state()
                .current_hand
                .as_ref()
                .map(|hand| hand.hand_number),
            action_window_summary,
            launch_hint: "Spawn another debug client with its own storage namespace, or attach a copied pkr1_ payload to exercise local multi-instance join handoff."
                .to_string(),
        })
    }

    #[cfg(test)]
    fn auto_play_opponents(&mut self) -> Result<(), String> {
        for _ in 0..48 {
            self.sync_log_markers();
            let Some(current_hand) = self.controller.state().current_hand.as_ref() else {
                return Ok(());
            };
            if self.controller.state().phase == domain::TournamentPhase::Complete {
                return Ok(());
            }
            if current_hand
                .action_window
                .as_ref()
                .is_some_and(|window| window.player_id == LOCAL_PLAYER_ID)
            {
                return Ok(());
            }
            self.auto_play_single_actor()?;
        }

        Ok(())
    }

    #[cfg(test)]
    fn auto_play_single_actor(&mut self) -> Result<(), String> {
        let maybe_window = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone());
        if let Some(window) = maybe_window {
            let (action_type, raise_to_amount, action_summary) =
                scripted_action_for_window(&window)?;
            let actor_label = display_name_for_state(self.controller.state(), &window.player_id)
                .unwrap_or(window.player_id.clone());
            let log_message = match raise_to_amount {
                Some(amount) => {
                    format!("{actor_label} auto-played {action_summary} to {amount} chips.")
                }
                None => format!("{actor_label} auto-played {action_summary}."),
            };
            let now_ms = self.bump_clock(800);
            self.controller
                .submit_action(
                    ActionRequest {
                        player_id: window.player_id,
                        action_window_id: window.action_window_id,
                        action_type,
                        raise_to_amount,
                    },
                    now_ms,
                )
                .map_err(|error| error.to_string())?;
            self.log_event("auto-action", log_message);
            self.sync_log_markers();
            return Ok(());
        }

        let now_ms = self.bump_clock(2_000);
        self.controller
            .advance_time(now_ms)
            .map_err(|error| error.to_string())?;
        self.sync_log_markers();
        Ok(())
    }

    fn sync_log_markers(&mut self) {
        let hand_number = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .map(|hand| hand.hand_number);
        if hand_number != self.last_hand_number {
            if let Some(number) = hand_number {
                self.log_event("hand", format!("Hand {number} is now live on the table."));
            }
            self.last_hand_number = hand_number;
        }

        let board_count = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .map(|hand| hand.board_cards.len())
            .unwrap_or_default();
        if board_count > self.last_board_count {
            let stage = match board_count {
                3 => "flop",
                4 => "turn",
                5 => "river",
                _ => "board",
            };
            self.log_event(
                "public-event",
                format!("The {stage} was published to every seat and observer."),
            );
            self.last_board_count = board_count;
        }

        let hand_result_count = self.controller.state().hand_results.len();
        if hand_result_count > self.last_hand_result_count {
            let settlement_messages = self.controller.state().hand_results
                [self.last_hand_result_count..]
                .iter()
                .map(|result| {
                    format!(
                        "Hand {} settled: {} collected the pot.",
                        result.hand_number,
                        display_names_for_state(
                            self.controller.state(),
                            &result.winning_player_ids
                        )
                        .join(", ")
                    )
                })
                .collect::<Vec<_>>();
            for message in settlement_messages {
                self.log_event("settlement", message);
            }
            self.last_hand_result_count = hand_result_count;
        }

        let action_window_id = self
            .controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| window.action_window_id.clone());
        if action_window_id != self.last_action_window_id {
            if let Some(window) = self
                .controller
                .state()
                .current_hand
                .as_ref()
                .and_then(|hand| hand.action_window.as_ref())
            {
                let owner = display_name_for_state(self.controller.state(), &window.player_id)
                    .unwrap_or_else(|| window.player_id.clone());
                self.log_event(
                    "action-window",
                    format!(
                        "Action window {} opened for {owner} (call {}, min {:?}, max {:?}).",
                        window.action_window_id,
                        window.call_amount,
                        window.min_raise_to,
                        window.max_raise_to
                    ),
                );
            }
            self.last_action_window_id = action_window_id;
        }
    }

    fn log_event(&mut self, kind: impl Into<String>, message: impl Into<String>) {
        self.protocol_log.push(TableEventView {
            sequence: self.next_sequence,
            kind: kind.into(),
            message: message.into(),
        });
        self.next_sequence += 1;
    }

    #[cfg(test)]
    fn bump_clock(&mut self, delta_ms: u64) -> u64 {
        self.now_ms += delta_ms;
        self.now_ms
    }
}

fn debug_demo_controller() -> Result<TournamentController, tournament::TournamentError> {
    TournamentController::new(
        "desktop-shell-table",
        1,
        domain::TournamentConfig {
            tournament_name: "Desktop Sit 'n Go".to_string(),
            table_name: Some("Main Table".to_string()),
            max_players: 6,
            starting_stack: 1_500,
            turn_timer_seconds: 30,
            blind_schedule: domain::BlindSchedule {
                levels: vec![domain::BlindLevel {
                    level_index: 1,
                    label: "Level 1 · 10 / 20".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 300,
                }],
            },
        },
        vec![
            registered_player(LOCAL_PLAYER_ID, "You", 0, true),
            registered_player(RESERVED_PLAYER_ID, "Reserved seat", 1, false),
        ],
    )
}

fn registered_player(
    player_id: &str,
    display_name: &str,
    seat_index: u8,
    is_host: bool,
) -> RegisteredPlayer {
    RegisteredPlayer {
        identity: domain::PlayerIdentity {
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
            signing_public_key: format!("sign-{player_id}"),
            encryption_public_key: format!("enc-{player_id}"),
            signing_key_fingerprint: format!("fp-{player_id}"),
        },
        seat_index,
        is_host,
        is_ready: true,
    }
}

fn resolve_action_request(
    window: &domain::ActionWindow,
    action_kind: DesktopTableActionKind,
    raise_to_amount: Option<u32>,
) -> Result<(domain::ActionType, Option<u32>, &'static str), String> {
    match action_kind {
        DesktopTableActionKind::Fold => {
            ensure_legal(window, domain::ActionType::Fold)?;
            Ok((domain::ActionType::Fold, None, "Fold"))
        }
        DesktopTableActionKind::CheckOrCall => {
            if window.legal_actions.contains(&domain::ActionType::Check) {
                Ok((domain::ActionType::Check, None, "Check"))
            } else {
                ensure_legal(window, domain::ActionType::Call)?;
                Ok((domain::ActionType::Call, None, "Call"))
            }
        }
        DesktopTableActionKind::BetOrRaise => {
            let raise_to_amount = raise_to_amount
                .ok_or_else(|| "raise amount is required for bet / raise".to_string())?;
            if window.legal_actions.contains(&domain::ActionType::Bet) {
                Ok((domain::ActionType::Bet, Some(raise_to_amount), "Bet"))
            } else {
                ensure_legal(window, domain::ActionType::Raise)?;
                Ok((domain::ActionType::Raise, Some(raise_to_amount), "Raise"))
            }
        }
        DesktopTableActionKind::AllIn => {
            ensure_legal(window, domain::ActionType::AllIn)?;
            Ok((domain::ActionType::AllIn, None, "All-in"))
        }
    }
}

#[cfg(test)]
fn scripted_action_for_window(
    window: &domain::ActionWindow,
) -> Result<(domain::ActionType, Option<u32>, &'static str), String> {
    if window.legal_actions.contains(&domain::ActionType::Check) {
        return Ok((domain::ActionType::Check, None, "Check"));
    }
    if window.legal_actions.contains(&domain::ActionType::Call) {
        return Ok((domain::ActionType::Call, None, "Call"));
    }
    if window.legal_actions.contains(&domain::ActionType::Fold) {
        return Ok((domain::ActionType::Fold, None, "Fold"));
    }
    if window.legal_actions.contains(&domain::ActionType::Bet) {
        return Ok((domain::ActionType::Bet, window.min_raise_to, "Bet"));
    }
    if window.legal_actions.contains(&domain::ActionType::Raise) {
        return Ok((domain::ActionType::Raise, window.min_raise_to, "Raise"));
    }
    if window.legal_actions.contains(&domain::ActionType::AllIn) {
        return Ok((domain::ActionType::AllIn, None, "All-in"));
    }

    Err("no scripted action available".to_string())
}

fn ensure_legal(window: &domain::ActionWindow, action: domain::ActionType) -> Result<(), String> {
    if window.legal_actions.contains(&action) {
        Ok(())
    } else {
        Err(format!(
            "{} is not legal in the current action window",
            format_action(action)
        ))
    }
}

fn build_table_view_snapshot(
    state: &domain::TournamentState,
    local_player_id: &str,
    viewer_mode: TableViewerMode,
    include_action_tray: bool,
    event_feed: Vec<TableEventView>,
) -> Result<TableViewSnapshot, String> {
    let projection = domain::StateProjector::project(state).map_err(|error| error.to_string())?;
    let public_state = projection.public_state.clone();
    let private_projection = projection
        .private_states
        .get(local_player_id)
        .cloned()
        .ok_or_else(|| "local player projection missing".to_string())?;
    let current_hand = state.current_hand.as_ref();
    let action_window = current_hand.and_then(|hand| hand.action_window.clone());
    let action_owner_label = public_state
        .action_window_player_id
        .as_deref()
        .and_then(|player_id| display_name_for_state(state, player_id))
        .unwrap_or_else(|| "Waiting for settlement".to_string());
    let pot_total = current_hand
        .as_ref()
        .map(|hand| hand.betting_round.pot_size)
        .unwrap_or_default();

    Ok(TableViewSnapshot {
        viewer_mode,
        tournament_name: public_state.tournament_name,
        table_name: public_state
            .table_name
            .unwrap_or_else(|| "Main Table".to_string()),
        table_id: public_state.table_id,
        phase_label: format_phase(public_state.phase),
        street_label: current_hand
            .map(|hand| format_street(hand.street))
            .unwrap_or_else(|| "Waiting".to_string()),
        blind_level_label: public_state
            .blind_level_label
            .unwrap_or_else(|| "Blind level pending".to_string()),
        current_hand_number: public_state.current_hand_number,
        board_cards: public_state.board_cards.iter().map(card_view).collect(),
        pot_total,
        action_owner_label: action_owner_label.clone(),
        elimination_summary: build_elimination_summary(state, public_state.phase),
        observer_banner: matches!(viewer_mode, TableViewerMode::Observer).then(|| {
            "Observer mode uses the public projector only: no private hole cards and no actions."
                .to_string()
        }),
        seats: build_table_seats_for_state(
            state,
            viewer_mode,
            local_player_id,
            &private_projection,
        )?,
        standings: build_table_standings_for_state(state, local_player_id),
        hand_history: build_table_history_for_state(state),
        event_feed,
        action_tray: if include_action_tray
            && matches!(viewer_mode, TableViewerMode::Local)
            && private_projection.can_act
        {
            action_window.map(|window| TableActionTrayView {
                owner_label: action_owner_label,
                check_or_call_label: if window.call_amount == 0 {
                    "Check".to_string()
                } else {
                    format!("Call {}", window.call_amount)
                },
                bet_or_raise_label: if window.call_amount == 0 {
                    "Bet / Raise".to_string()
                } else {
                    format!(
                        "Raise to {}+",
                        window.min_raise_to.unwrap_or(window.call_amount)
                    )
                },
                call_amount: window.call_amount,
                current_bet: current_hand
                    .as_ref()
                    .map(|hand| hand.betting_round.current_bet)
                    .unwrap_or_default(),
                pot_total,
                min_raise_to: window.min_raise_to,
                max_raise_to: window.max_raise_to,
                deadline_epoch_ms: window.deadline_epoch_ms,
                legal_actions: window
                    .legal_actions
                    .iter()
                    .map(|action| format_action(*action))
                    .collect(),
            })
        } else {
            None
        },
    })
}

fn build_table_seats_for_state(
    state: &domain::TournamentState,
    viewer_mode: TableViewerMode,
    local_player_id: &str,
    private_projection: &domain::PrivateState,
) -> Result<Vec<TableSeatView>, String> {
    let current_hand = state.current_hand.as_ref();
    let contributions = current_hand
        .as_ref()
        .map(|hand| hand.betting_round.contributions_by_player_id.clone())
        .unwrap_or_default();

    let mut seats = state
        .seats
        .iter()
        .map(|seat| {
            if seat.occupancy == domain::SeatOccupancyState::Empty {
                return Ok(TableSeatView {
                    seat_index: seat.seat_index + 1,
                    display_name: "Open seat".to_string(),
                    chip_count: None,
                    status_label: "Open".to_string(),
                    marker_label: None,
                    contribution: 0,
                    is_local: false,
                    is_acting: false,
                    is_observer: false,
                    is_eliminated: false,
                    is_compact: true,
                    cards_hidden: true,
                    hole_cards: Vec::new(),
                    detail_lines: vec!["Available for another player to join.".to_string()],
                });
            }

            let player_id = seat
                .participant_id
                .as_deref()
                .ok_or_else(|| "occupied seat missing participant".to_string())?;
            let display_name = seat
                .display_name
                .clone()
                .unwrap_or_else(|| player_id.to_string());
            let participation = current_hand
                .as_ref()
                .and_then(|hand| hand.participation_by_player_id.get(player_id).copied());
            let is_local = player_id == local_player_id;
            let revealed_public_cards = current_hand
                .as_ref()
                .and_then(|hand| {
                    hand.hole_cards_by_player_id
                        .get(player_id)
                        .cloned()
                        .filter(|_| {
                            !is_local
                                && matches!(
                                    viewer_mode,
                                    TableViewerMode::Local | TableViewerMode::Observer
                                )
                                && matches!(
                                    hand.cycle_phase,
                                    domain::HandCyclePhase::Showdown
                                        | domain::HandCyclePhase::Settlement
                                )
                                && hand.participation_by_player_id.get(player_id).is_some_and(
                                    |participation| {
                                        !matches!(
                                            participation,
                                            domain::HandParticipationState::Folded
                                                | domain::HandParticipationState::Out
                                                | domain::HandParticipationState::EliminatedObserver
                                        )
                                    },
                                )
                        })
                })
                .unwrap_or_default();
            let visible_cards = if is_local && matches!(viewer_mode, TableViewerMode::Local) {
                private_projection
                    .private_hole_cards
                    .iter()
                    .map(card_view)
                    .collect::<Vec<_>>()
            } else if !revealed_public_cards.is_empty() {
                revealed_public_cards
                    .iter()
                    .map(card_view)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let cards_hidden = visible_cards.is_empty();
            let contribution = contributions.get(player_id).copied().unwrap_or_default();
            let participant = state
                .participants
                .get(player_id)
                .ok_or_else(|| "participant missing from registry".to_string())?;

            if player_id == RESERVED_PLAYER_ID {
                return Ok(TableSeatView {
                    seat_index: seat.seat_index + 1,
                    display_name: "Waiting for player".to_string(),
                    chip_count: None,
                    status_label: "Reserved".to_string(),
                    marker_label: None,
                    contribution: 0,
                    is_local: false,
                    is_acting: false,
                    is_observer: false,
                    is_eliminated: false,
                    is_compact: true,
                    cards_hidden: true,
                    hole_cards: Vec::new(),
                    detail_lines: vec![
                        "Placeholder seat until another real player joins.".to_string()
                    ],
                });
            }

            Ok(TableSeatView {
                seat_index: seat.seat_index + 1,
                display_name,
                chip_count: seat.chip_count,
                status_label: status_label_for_seat(seat, participation),
                marker_label: seat.marker.map(format_marker),
                contribution,
                is_local,
                is_acting: current_hand
                    .as_ref()
                    .and_then(|hand| hand.action_window.as_ref())
                    .is_some_and(|window| window.player_id == player_id),
                is_observer: participant.state == domain::ParticipantState::EliminatedObserver,
                is_eliminated: seat.tournament_state
                    == domain::TournamentSeatState::EliminatedObserver,
                is_compact: !is_local,
                cards_hidden,
                hole_cards: visible_cards,
                detail_lines: vec![
                    format!(
                        "Connection: {}",
                        format_connection_state(participant.connection_state)
                    ),
                    format!(
                        "Seat state: {}",
                        format_tournament_seat_state(seat.tournament_state)
                    ),
                    format!("Contribution: {contribution}"),
                ],
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    seats.sort_by_key(|seat| seat.seat_index);
    Ok(seats)
}

fn build_table_standings_for_state(
    state: &domain::TournamentState,
    local_player_id: &str,
) -> Vec<TableStandingView> {
    let mut standings = state
        .seats
        .iter()
        .filter(|seat| {
            seat.occupancy == domain::SeatOccupancyState::Occupied
                && seat.participant_id.as_deref() != Some(RESERVED_PLAYER_ID)
        })
        .map(|seat| TableStandingView {
            rank: 0,
            display_name: seat
                .display_name
                .clone()
                .unwrap_or_else(|| "Player".to_string()),
            chip_count: seat.chip_count,
            status_label: format_tournament_seat_state(seat.tournament_state),
            note: seat.marker.map(format_marker),
            is_local: seat.participant_id.as_deref() == Some(local_player_id),
            is_observer: seat.tournament_state == domain::TournamentSeatState::EliminatedObserver,
        })
        .collect::<Vec<_>>();
    standings.sort_by(|left, right| {
        right
            .chip_count
            .unwrap_or_default()
            .cmp(&left.chip_count.unwrap_or_default())
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    for (index, entry) in standings.iter_mut().enumerate() {
        entry.rank = (index + 1) as u8;
    }
    standings
}

fn build_table_history_for_state(state: &domain::TournamentState) -> Vec<TableHistoryEntryView> {
    state
        .hand_results
        .iter()
        .rev()
        .map(|result| TableHistoryEntryView {
            hand_number: result.hand_number,
            summary: format!(
                "{} won {} chip(s).",
                display_names_for_state(state, &result.winning_player_ids).join(", "),
                result
                    .pot_summaries
                    .iter()
                    .map(|summary| summary.amount)
                    .sum::<u32>()
            ),
            pot_total: result
                .pot_summaries
                .iter()
                .map(|summary| summary.amount)
                .sum(),
            winning_players: display_names_for_state(state, &result.winning_player_ids),
            eliminated_players: display_names_for_state(state, &result.eliminated_player_ids),
            board_cards: result.board_cards.iter().map(card_view).collect(),
        })
        .collect()
}

fn build_live_event_feed(
    events: &[networking::PublicEventLogEntry],
    state: &domain::TournamentState,
) -> Vec<TableEventView> {
    let mut feed = events
        .iter()
        .rev()
        .take(20)
        .map(|event| {
            let mut entry = TableEventView {
                sequence: event.sequence,
                kind: format!("{:?}", event.message_type),
                message: format!("{:?}", event.message_type),
            };
            update_live_event_entry(&mut entry, event.message_type, &event.payload, state);
            entry
        })
        .collect::<Vec<_>>();
    feed.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
    feed
}

fn push_live_event(
    event_feed: &mut Vec<TableEventView>,
    server_sequence: u64,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
    state: &domain::TournamentState,
) {
    let mut entry = TableEventView {
        sequence: server_sequence,
        kind: format!("{:?}", message_type),
        message: format!("{:?}", message_type),
    };
    update_live_event_entry(&mut entry, message_type, payload, state);
    event_feed.push(entry);
    event_feed.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
    event_feed.truncate(20);
}

fn update_live_event_entry(
    entry: &mut TableEventView,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
    state: &domain::TournamentState,
) {
    match message_type {
        protocol::ProtocolMessageType::TournamentStartedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::TournamentStartedEvent>(payload.clone())
            {
                entry.kind = "Tournament start".to_string();
                entry.message = format!(
                    "{} is live with {} seated players.",
                    event.tournament_name,
                    event.frozen_player_ids.len()
                );
            }
        }
        protocol::ProtocolMessageType::HandStartingEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::HandStartingEvent>(payload.clone())
            {
                entry.kind = "Hand start".to_string();
                entry.message = format!("Hand {} started.", event.hand_number);
            }
        }
        protocol::ProtocolMessageType::ActionWindowOpenedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::ActionWindowOpened>(payload.clone())
            {
                entry.kind = "Action window".to_string();
                entry.message = format!(
                    "{} to act.",
                    display_name_for_state(state, &event.player_id)
                        .unwrap_or_else(|| event.player_id.clone())
                );
            }
        }
        protocol::ProtocolMessageType::PlayerActionCommittedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::PlayerActionCommitted>(payload.clone())
            {
                entry.kind = "Player action".to_string();
                let actor = display_name_for_state(state, &event.player_id)
                    .unwrap_or_else(|| event.player_id.clone());
                entry.message = match event.raise_to_amount {
                    Some(amount) => format!(
                        "{actor} {} to {}.",
                        format_action(event.action_type),
                        amount
                    ),
                    None => format!("{actor} {}.", format_action(event.action_type)),
                };
            }
        }
        protocol::ProtocolMessageType::StreetRevealedEvent => {
            if let Ok(event) = serde_json::from_value::<protocol::StreetRevealed>(payload.clone()) {
                entry.kind = "Street reveal".to_string();
                entry.message = format!("{} revealed.", event.street);
            }
        }
        protocol::ProtocolMessageType::HandResultCommittedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::HandResultCommitted>(payload.clone())
            {
                entry.kind = "Hand result".to_string();
                entry.message = format!(
                    "Hand {} settled: {} collected the pot.",
                    event.hand_number,
                    display_names_for_state(state, &event.result.winning_player_ids).join(", ")
                );
            }
        }
        protocol::ProtocolMessageType::EliminationEvent => {
            if let Ok(event) = serde_json::from_value::<protocol::EliminationEvent>(payload.clone())
            {
                entry.kind = "Elimination".to_string();
                entry.message = format!(
                    "{} finished in place {}.",
                    display_name_for_state(state, &event.player_id)
                        .unwrap_or_else(|| event.player_id.clone()),
                    event.place
                );
            }
        }
        protocol::ProtocolMessageType::TournamentCompleteEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::TournamentCompleteEvent>(payload.clone())
            {
                entry.kind = "Tournament complete".to_string();
                entry.message = format!(
                    "{} won the tournament.",
                    display_name_for_state(state, &event.winner_player_id)
                        .unwrap_or_else(|| event.winner_player_id.clone())
                );
            }
        }
        _ => {}
    }
}

fn parse_protocol_street(value: &str) -> Option<domain::StreetPhase> {
    match value.trim().to_ascii_uppercase().as_str() {
        "PREFLOP" => Some(domain::StreetPhase::Preflop),
        "FLOP" => Some(domain::StreetPhase::Flop),
        "TURN" => Some(domain::StreetPhase::Turn),
        "RIVER" => Some(domain::StreetPhase::River),
        "SHOWDOWN" => Some(domain::StreetPhase::Showdown),
        _ => None,
    }
}

fn apply_public_event_to_snapshot(
    state: &mut domain::TournamentState,
    local_player_id: &str,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
) {
    match message_type {
        protocol::ProtocolMessageType::TournamentStartedEvent => {
            state.phase = domain::TournamentPhase::Running;
            for seat in &mut state.seats {
                if seat.occupancy == domain::SeatOccupancyState::Occupied {
                    seat.tournament_state = domain::TournamentSeatState::Active;
                    seat.is_ready = false;
                    seat.chip_count.get_or_insert(state.config.starting_stack);
                }
            }
            for participant in state.participants.values_mut() {
                if participant.state == domain::ParticipantState::Seated {
                    participant.state = domain::ParticipantState::Active;
                }
            }
        }
        protocol::ProtocolMessageType::HandStartingEvent => {
            let Ok(event) = serde_json::from_value::<protocol::HandStartingEvent>(payload.clone())
            else {
                return;
            };
            let blind_level = state
                .blind_schedule
                .levels
                .get(state.blind_level_index)
                .cloned();
            let small_blind = blind_level
                .as_ref()
                .map(|level| level.small_blind)
                .unwrap_or(0);
            let big_blind = blind_level
                .as_ref()
                .map(|level| level.big_blind)
                .unwrap_or(0);
            let mut contributions = std::collections::BTreeMap::new();
            let mut participation = std::collections::BTreeMap::new();
            for seat in &mut state.seats {
                seat.marker = None;
                if seat.occupancy != domain::SeatOccupancyState::Occupied {
                    continue;
                }
                seat.tournament_state =
                    if seat.tournament_state == domain::TournamentSeatState::EliminatedObserver {
                        domain::TournamentSeatState::EliminatedObserver
                    } else {
                        domain::TournamentSeatState::Active
                    };
                let Some(player_id) = seat.participant_id.clone() else {
                    continue;
                };
                if seat.tournament_state != domain::TournamentSeatState::EliminatedObserver {
                    participation.insert(player_id.clone(), domain::HandParticipationState::Active);
                } else {
                    participation.insert(
                        player_id.clone(),
                        domain::HandParticipationState::EliminatedObserver,
                    );
                }
                let contribution = if seat.seat_index == event.small_blind_seat_index {
                    seat.marker = Some(domain::SeatMarker::SmallBlind);
                    small_blind
                } else if seat.seat_index == event.big_blind_seat_index {
                    seat.marker = Some(domain::SeatMarker::BigBlind);
                    big_blind
                } else {
                    0
                };
                if seat.seat_index == event.dealer_seat_index {
                    seat.marker = Some(domain::SeatMarker::Dealer);
                }
                if contribution > 0 {
                    let stack = seat.chip_count.get_or_insert(state.config.starting_stack);
                    *stack = stack.saturating_sub(contribution);
                } else {
                    seat.chip_count.get_or_insert(state.config.starting_stack);
                }
                contributions.insert(player_id, contribution);
            }
            state.current_hand = Some(domain::HandState {
                hand_number: event.hand_number,
                cycle_phase: domain::HandCyclePhase::AwaitingAction,
                street: domain::StreetPhase::Preflop,
                dealer_seat_index: event.dealer_seat_index,
                small_blind_seat_index: event.small_blind_seat_index,
                big_blind_seat_index: event.big_blind_seat_index,
                board_cards: event.board_cards,
                hole_cards_by_player_id: std::collections::BTreeMap::new(),
                participation_by_player_id: participation,
                betting_round: domain::BettingRoundState {
                    street: domain::StreetPhase::Preflop,
                    current_bet: big_blind,
                    min_raise_to: Some(big_blind.saturating_mul(2)),
                    max_raise_to: None,
                    pot_size: small_blind.saturating_add(big_blind),
                    contributions_by_player_id: contributions,
                },
                action_window: None,
            });
        }
        protocol::ProtocolMessageType::ActionWindowOpenedEvent => {
            let Ok(event) = serde_json::from_value::<protocol::ActionWindowOpened>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            let contribution = hand
                .betting_round
                .contributions_by_player_id
                .get(&event.player_id)
                .copied()
                .unwrap_or_default();
            hand.cycle_phase = domain::HandCyclePhase::AwaitingAction;
            hand.action_window = Some(domain::ActionWindow {
                action_window_id: event.action_window_id,
                player_id: event.player_id,
                seat_index: event.seat_index,
                legal_actions: event.legal_actions,
                call_amount: event.call_amount,
                min_raise_to: event.min_raise_to,
                max_raise_to: event.max_raise_to,
                deadline_epoch_ms: event.deadline_epoch_ms,
            });
            hand.betting_round.current_bet = hand
                .betting_round
                .current_bet
                .max(contribution.saturating_add(event.call_amount));
            hand.betting_round.min_raise_to = event.min_raise_to;
            hand.betting_round.max_raise_to = event.max_raise_to;
        }
        protocol::ProtocolMessageType::PlayerActionCommittedEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::PlayerActionCommitted>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            let previous_call_amount = hand
                .action_window
                .as_ref()
                .filter(|window| window.player_id == event.player_id)
                .map(|window| window.call_amount)
                .unwrap_or_default();
            let previous_contribution = hand
                .betting_round
                .contributions_by_player_id
                .get(&event.player_id)
                .copied()
                .unwrap_or_default();
            let next_contribution = match event.action_type {
                domain::ActionType::Fold | domain::ActionType::Check => previous_contribution,
                domain::ActionType::Call => {
                    previous_contribution.saturating_add(previous_call_amount)
                }
                domain::ActionType::Bet | domain::ActionType::Raise | domain::ActionType::AllIn => {
                    event
                        .raise_to_amount
                        .unwrap_or(previous_contribution.saturating_add(previous_call_amount))
                }
            };
            let additional = next_contribution.saturating_sub(previous_contribution);
            hand.betting_round
                .contributions_by_player_id
                .insert(event.player_id.clone(), next_contribution);
            hand.betting_round.pot_size = hand.betting_round.pot_size.saturating_add(additional);
            hand.betting_round.current_bet = hand.betting_round.current_bet.max(next_contribution);
            if let Some(participation) = hand.participation_by_player_id.get_mut(&event.player_id) {
                *participation = match event.action_type {
                    domain::ActionType::Fold => domain::HandParticipationState::Folded,
                    domain::ActionType::AllIn => domain::HandParticipationState::AllIn,
                    _ => domain::HandParticipationState::Active,
                };
            }
            if let Some(seat) = state
                .seats
                .iter_mut()
                .find(|seat| seat.participant_id.as_deref() == Some(event.player_id.as_str()))
            {
                if let Some(chips) = seat.chip_count.as_mut() {
                    *chips = chips.saturating_sub(additional);
                    if *chips == 0 && event.action_type != domain::ActionType::Fold {
                        if let Some(participation) =
                            hand.participation_by_player_id.get_mut(&event.player_id)
                        {
                            *participation = domain::HandParticipationState::AllIn;
                        }
                    }
                }
            }
            hand.action_window = None;
        }
        protocol::ProtocolMessageType::StreetRevealedEvent => {
            let Ok(event) = serde_json::from_value::<protocol::StreetRevealed>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            hand.board_cards = event.board_cards;
            if let Some(street) = parse_protocol_street(&event.street) {
                hand.street = street;
                hand.betting_round.street = street;
            }
            hand.cycle_phase = domain::HandCyclePhase::AwaitingAction;
            hand.betting_round.current_bet = 0;
            hand.betting_round.min_raise_to = None;
            hand.betting_round.max_raise_to = None;
            hand.action_window = None;
        }
        protocol::ProtocolMessageType::HandResultCommittedEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::HandResultCommitted>(payload.clone())
            else {
                return;
            };
            if !state
                .hand_results
                .iter()
                .any(|result| result.hand_number == event.hand_number)
            {
                state.hand_results.push(event.result.clone());
            }
            if event.result.final_stack_by_player_id.is_empty() {
                let mut payouts = std::collections::BTreeMap::<String, u32>::new();
                for pot in &event.result.pot_summaries {
                    let Some(split_winner_count) = (!pot.winner_player_ids.is_empty())
                        .then_some(pot.winner_player_ids.len() as u32)
                    else {
                        continue;
                    };
                    let split_amount = pot.amount / split_winner_count;
                    for winner in &pot.winner_player_ids {
                        *payouts.entry(winner.clone()).or_default() += split_amount;
                    }
                    if let Some(odd_chip_player_id) = pot.odd_chip_awarded_to.as_ref() {
                        *payouts.entry(odd_chip_player_id.clone()).or_default() +=
                            pot.odd_chip_count;
                    }
                }
                for seat in &mut state.seats {
                    let Some(player_id) = seat.participant_id.as_ref() else {
                        continue;
                    };
                    if let Some(payout) = payouts.get(player_id) {
                        let chips = seat.chip_count.get_or_insert(0);
                        *chips = chips.saturating_add(*payout);
                    }
                }
            } else {
                for seat in &mut state.seats {
                    let Some(player_id) = seat.participant_id.as_ref() else {
                        continue;
                    };
                    if let Some(final_stack) = event.result.final_stack_by_player_id.get(player_id)
                    {
                        seat.chip_count = Some(*final_stack);
                    }
                }
            }
            if let Some(hand) = state.current_hand.as_mut() {
                if hand.hand_number == event.hand_number {
                    hand.board_cards = event.result.board_cards.clone();
                    for (player_id, cards) in &event.result.revealed_hands_by_player_id {
                        hand.hole_cards_by_player_id
                            .insert(player_id.clone(), cards.clone());
                    }
                    hand.cycle_phase = domain::HandCyclePhase::Settlement;
                    hand.action_window = None;
                }
            }
        }
        protocol::ProtocolMessageType::EliminationEvent => {
            let Ok(event) = serde_json::from_value::<protocol::EliminationEvent>(payload.clone())
            else {
                return;
            };
            if !state
                .placements
                .iter()
                .any(|entry| entry.player_id == event.player_id)
            {
                state.placements.push(domain::PlacementEntry {
                    player_id: event.player_id.clone(),
                    place: event.place,
                    busted_at_hand_number: state.current_hand.as_ref().map(|hand| hand.hand_number),
                });
            }
            if let Some(participant) = state.participants.get_mut(&event.player_id) {
                participant.state = domain::ParticipantState::EliminatedObserver;
            }
            if let Some(seat) = state
                .seats
                .iter_mut()
                .find(|seat| seat.participant_id.as_deref() == Some(event.player_id.as_str()))
            {
                seat.tournament_state = domain::TournamentSeatState::EliminatedObserver;
                seat.chip_count = Some(0);
            }
        }
        protocol::ProtocolMessageType::TournamentCompleteEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::TournamentCompleteEvent>(payload.clone())
            else {
                return;
            };
            state.phase = domain::TournamentPhase::Complete;
            state.placements = event.placements;
            if let Some(hand) = state.current_hand.as_mut() {
                hand.action_window = None;
                hand.cycle_phase = domain::HandCyclePhase::Settlement;
            }
        }
        _ => {
            let _ = local_player_id;
        }
    }
}

fn apply_private_hole_cards_to_snapshot(
    state: &mut domain::TournamentState,
    event: &protocol::PrivateHoleCardsEvent,
) {
    let Some(hand) = state.current_hand.as_mut() else {
        return;
    };
    hand.hole_cards_by_player_id
        .insert(event.recipient_player_id.clone(), event.hole_cards.clone());
}

fn build_elimination_summary(
    state: &domain::TournamentState,
    phase: domain::TournamentPhase,
) -> String {
    if phase == domain::TournamentPhase::WaitingForPlayers {
        return "Waiting for the first real hand to start.".to_string();
    }
    if phase == domain::TournamentPhase::ReadyCheck {
        return "Waiting for every seated player to be ready.".to_string();
    }

    state
        .hand_results
        .last()
        .map(|result| {
            format!(
                "{} won {} chip(s).",
                display_names_for_state(state, &result.winning_player_ids).join(", "),
                result
                    .pot_summaries
                    .iter()
                    .map(|summary| summary.amount)
                    .sum::<u32>()
            )
        })
        .unwrap_or_else(|| "Table state is live.".to_string())
}

fn card_view(card: &domain::Card) -> TableCardView {
    let (rank_label, compact_label) = match card.rank {
        domain::Rank::Two => ("Two", "2"),
        domain::Rank::Three => ("Three", "3"),
        domain::Rank::Four => ("Four", "4"),
        domain::Rank::Five => ("Five", "5"),
        domain::Rank::Six => ("Six", "6"),
        domain::Rank::Seven => ("Seven", "7"),
        domain::Rank::Eight => ("Eight", "8"),
        domain::Rank::Nine => ("Nine", "9"),
        domain::Rank::Ten => ("Ten", "10"),
        domain::Rank::Jack => ("Jack", "J"),
        domain::Rank::Queen => ("Queen", "Q"),
        domain::Rank::King => ("King", "K"),
        domain::Rank::Ace => ("Ace", "A"),
    };
    let (suit_label, suit_symbol, tone) = match card.suit {
        domain::Suit::Clubs => ("Clubs", "♣", "dark"),
        domain::Suit::Diamonds => ("Diamonds", "♦", "red"),
        domain::Suit::Hearts => ("Hearts", "♥", "red"),
        domain::Suit::Spades => ("Spades", "♠", "dark"),
    };
    TableCardView {
        label: format!("{rank_label} of {suit_label}"),
        compact_label: format!("{compact_label}{suit_symbol}"),
        suit_symbol,
        tone,
    }
}

fn display_name_for_state(state: &domain::TournamentState, player_id: &str) -> Option<String> {
    state
        .participants
        .get(player_id)
        .map(|participant| participant.identity.display_name.clone())
}

fn display_names_for_state(state: &domain::TournamentState, player_ids: &[String]) -> Vec<String> {
    player_ids
        .iter()
        .map(|player_id| {
            display_name_for_state(state, player_id).unwrap_or_else(|| player_id.clone())
        })
        .collect()
}

fn status_label_for_seat(
    seat: &domain::SeatState,
    participation: Option<domain::HandParticipationState>,
) -> String {
    match participation {
        Some(domain::HandParticipationState::Folded) => "Folded this hand".to_string(),
        Some(domain::HandParticipationState::AllIn) => "All-in".to_string(),
        Some(domain::HandParticipationState::Active) => {
            format_tournament_seat_state(seat.tournament_state)
        }
        Some(domain::HandParticipationState::EliminatedObserver) => {
            "Eliminated observer".to_string()
        }
        Some(domain::HandParticipationState::Out) => "Out of this hand".to_string(),
        Some(domain::HandParticipationState::Waiting) | None => {
            format_tournament_seat_state(seat.tournament_state)
        }
    }
}

fn format_phase(phase: domain::TournamentPhase) -> String {
    match phase {
        domain::TournamentPhase::WaitingForPlayers => "Waiting for players".to_string(),
        domain::TournamentPhase::ReadyCheck => "Ready check".to_string(),
        domain::TournamentPhase::Running => "Running".to_string(),
        domain::TournamentPhase::Complete => "Complete".to_string(),
        domain::TournamentPhase::Cancelled => "Cancelled".to_string(),
    }
}

fn format_street(street: domain::StreetPhase) -> String {
    match street {
        domain::StreetPhase::Preflop => "Preflop".to_string(),
        domain::StreetPhase::Flop => "Flop".to_string(),
        domain::StreetPhase::Turn => "Turn".to_string(),
        domain::StreetPhase::River => "River".to_string(),
        domain::StreetPhase::Showdown => "Showdown".to_string(),
    }
}

fn format_marker(marker: domain::SeatMarker) -> String {
    match marker {
        domain::SeatMarker::Dealer => "Dealer".to_string(),
        domain::SeatMarker::SmallBlind => "Small blind".to_string(),
        domain::SeatMarker::BigBlind => "Big blind".to_string(),
    }
}

fn format_connection_state(state: domain::ConnectionState) -> &'static str {
    match state {
        domain::ConnectionState::Connected => "Connected",
        domain::ConnectionState::Disconnected => "Disconnected",
        domain::ConnectionState::Reconnecting => "Reconnecting",
    }
}

fn format_tournament_seat_state(state: domain::TournamentSeatState) -> String {
    match state {
        domain::TournamentSeatState::Open => "Open".to_string(),
        domain::TournamentSeatState::Lobby => "Lobby".to_string(),
        domain::TournamentSeatState::Ready => "Ready".to_string(),
        domain::TournamentSeatState::Active => "Active".to_string(),
        domain::TournamentSeatState::EliminatedObserver => "Eliminated observer".to_string(),
        domain::TournamentSeatState::Closed => "Closed".to_string(),
    }
}

fn format_action(action: domain::ActionType) -> String {
    match action {
        domain::ActionType::Fold => "Fold".to_string(),
        domain::ActionType::Check => "Check".to_string(),
        domain::ActionType::Call => "Call".to_string(),
        domain::ActionType::Bet => "Bet".to_string(),
        domain::ActionType::Raise => "Raise".to_string(),
        domain::ActionType::AllIn => "All-in".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        apply_public_event_to_snapshot, blind_schedule_for_preset, build_debug_child_instance_id,
        build_debug_child_launch_args, build_table_history_for_state, build_table_view_snapshot,
        derive_instance_profile, detect_profile_directory, ensure_legal, format_marker,
        format_phase, format_street, issue_join_token, resolve_action_request, screen_catalog,
        ClaimLobbySeatRequest, DesktopAppState, DesktopTableActionKind, JoinHostSessionRequest,
        SetLobbyReadyStateRequest, StartHostSessionRequest, TableViewerMode, INSTANCE_ID_ENV_VAR,
        JOIN_PAYLOAD_ENV_VAR, LOCAL_PLAYER_ID,
    };
    use crate::{
        domain::{
            ActionType, ActionWindow, BettingRoundState, Card, ConnectionState, HandCyclePhase,
            HandParticipationState, HandResult, ParticipantRegistryEntry, ParticipantState,
            PlayerIdentity, PotSummary, Rank, SeatMarker, SeatOccupancyState, SeatState,
            StreetPhase, Suit, TournamentConfig, TournamentPhase, TournamentSeatState,
            TournamentState,
        },
        networking::HostRuntimeMode,
        protocol::{self, decode_join_payload},
    };

    fn sample_host_session_request(host_address: &str) -> StartHostSessionRequest {
        StartHostSessionRequest {
            host_address: host_address.to_string(),
            host_port: 0,
            tournament_name: "Friday Finals".to_string(),
            max_players: 6,
            starting_stack: 1_500,
            blind_preset_id: "normal".to_string(),
            turn_timer_seconds: 30,
            display_name: "Host Alpha".to_string(),
        }
    }

    fn sample_join_host_session_request(join_payload: &str) -> JoinHostSessionRequest {
        JoinHostSessionRequest {
            join_payload: join_payload.to_string(),
            display_name: "Client Bravo".to_string(),
        }
    }

    fn sample_action_window(legal_actions: Vec<ActionType>) -> ActionWindow {
        ActionWindow {
            action_window_id: "window-1".to_string(),
            player_id: super::LOCAL_PLAYER_ID.to_string(),
            seat_index: 0,
            legal_actions,
            call_amount: 40,
            min_raise_to: Some(80),
            max_raise_to: Some(200),
            deadline_epoch_ms: 123_456,
        }
    }

    fn start_live_table_state() -> DesktopAppState {
        let state = DesktopAppState::detect();
        state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .controller
            .start_tournament(1)
            .expect("start tournament");
        state
    }

    #[test]
    fn detect_uses_android_compatible_defaults() {
        std::env::remove_var(INSTANCE_ID_ENV_VAR);
        std::env::remove_var(JOIN_PAYLOAD_ENV_VAR);

        let state = DesktopAppState::detect().bootstrap();

        assert_eq!(state.protocol_version, 1);
        assert_eq!(state.default_host_port, 43_818);
        assert_eq!(state.instance_id, "default");
        assert_eq!(state.instance_label, "default");
        assert_eq!(state.storage_namespace, "desktop-poker:default");
        assert_eq!(state.session_identity, "desktop-session:default");
        assert_eq!(state.reconnect_namespace, "desktop-reconnect:default");
        assert!(state
            .profile_directory
            .ends_with("desktop-poker/profiles/default"));
    }

    #[test]
    fn detect_does_not_boot_debug_table_runtime_until_debug_state_is_requested() {
        let state = DesktopAppState::detect();

        assert!(state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .is_none());

        let debug_state = state
            .debug_state(TableViewerMode::Local)
            .expect("debug state should lazily initialize the debug runtime");

        assert_eq!(debug_state.current_hand_number, None);
        assert!(state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .is_some());
    }

    #[test]
    fn instance_profile_sanitizes_namespace_and_identity_fields() {
        let profile = derive_instance_profile(Some("Host A / QA"));

        assert_eq!(profile.instance_label, "Host A / QA");
        assert_eq!(profile.profile_id, "host-a-qa");
        assert_eq!(profile.storage_namespace, "desktop-poker:host-a-qa");
        assert_eq!(profile.session_identity, "desktop-session:host-a-qa");
        assert_eq!(profile.reconnect_namespace, "desktop-reconnect:host-a-qa");
        assert!(profile
            .profile_directory
            .ends_with("desktop-poker/profiles/host-a-qa"));
    }

    #[test]
    fn debug_child_instance_ids_are_scoped_to_the_parent_profile() {
        let first_child = build_debug_child_instance_id("host-a", 9001, 1);
        let second_child = build_debug_child_instance_id("host-a", 9001, 2);
        let other_parent_child = build_debug_child_instance_id("host-b", 9001, 1);

        assert_eq!(first_child, "host-a-p9001-client-1");
        assert_eq!(second_child, "host-a-p9001-client-2");
        assert_eq!(other_parent_child, "host-b-p9001-client-1");
        assert_ne!(first_child, other_parent_child);
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

    #[test]
    fn table_view_requires_an_active_live_session() {
        std::env::remove_var(INSTANCE_ID_ENV_VAR);
        std::env::remove_var(JOIN_PAYLOAD_ENV_VAR);

        assert_eq!(
            DesktopAppState::detect()
                .table_view(TableViewerMode::Local)
                .expect_err("table view should require an active live session"),
            "no active live session is available for the table view"
        );
    }

    #[test]
    fn table_actions_require_an_active_live_session() {
        assert_eq!(
            DesktopAppState::detect()
                .submit_table_action(TableViewerMode::Local, DesktopTableActionKind::Fold, None)
                .expect_err("table actions should require an active live session"),
            "no active live session is available for table actions"
        );
    }

    #[test]
    fn local_actions_advance_runtime_and_invalid_paths_fail_cleanly() {
        let state = start_live_table_state();
        let before_action = state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .view(TableViewerMode::Local)
            .expect("table view before action");

        assert_eq!(before_action.current_hand_number, Some(1));
        assert!(before_action.action_tray.is_some());

        assert_eq!(
            state
                .debug_table_runtime
                .lock()
                .expect("debug table runtime")
                .get_or_insert_with(|| {
                    super::DebugTableRuntime::new().expect("debug table runtime should initialize")
                })
                .submit_action(
                    TableViewerMode::Observer,
                    DesktopTableActionKind::Fold,
                    None,
                )
                .expect_err("observer action should fail"),
            "observer mode cannot submit actions"
        );

        let invalid_raise_state = start_live_table_state();
        assert!(invalid_raise_state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .submit_action(
                TableViewerMode::Local,
                DesktopTableActionKind::BetOrRaise,
                Some(1),
            )
            .expect_err("invalid raise should fail")
            .contains("minimum full raise sizing"));

        let after_action = state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .submit_action(
                TableViewerMode::Local,
                DesktopTableActionKind::CheckOrCall,
                None,
            )
            .expect("check/call should succeed");

        assert_eq!(after_action.current_hand_number, Some(1));
        assert!(!after_action.event_feed.is_empty());
        assert!(after_action
            .event_feed
            .iter()
            .any(|entry| entry.message.contains("You selected")));
    }

    #[test]
    fn debug_state_tracks_runtime_sequence_and_action_window_presence() {
        let idle_debug_state = DesktopAppState::detect()
            .debug_state(TableViewerMode::Local)
            .expect("idle debug state");
        assert_eq!(idle_debug_state.current_hand_number, None);
        assert!(idle_debug_state.action_window_summary.is_none());

        let state = start_live_table_state();
        let running_debug_state = state
            .debug_state(TableViewerMode::Local)
            .expect("running debug state");
        assert_eq!(running_debug_state.current_hand_number, Some(1));
        assert!(running_debug_state.current_sequence >= 1);
        assert!(running_debug_state
            .action_window_summary
            .as_deref()
            .is_some_and(|summary| summary.contains("You")));

        state
            .debug_table_runtime
            .lock()
            .expect("debug table runtime")
            .get_or_insert_with(|| {
                super::DebugTableRuntime::new().expect("debug table runtime should initialize")
            })
            .submit_action(
                TableViewerMode::Local,
                DesktopTableActionKind::CheckOrCall,
                None,
            )
            .expect("local action");
        let updated_debug_state = state
            .debug_state(TableViewerMode::Local)
            .expect("updated debug state");
        assert!(updated_debug_state.current_sequence > running_debug_state.current_sequence);
        assert!(!updated_debug_state.protocol_log.is_empty());
    }

    #[test]
    fn debug_child_launch_args_keep_instance_scope_and_optional_join_payload() {
        let (instance_id, args) =
            build_debug_child_launch_args("host-a", 9001, 2, Some("  pkr1_join  "));
        let (instance_id_without_payload, args_without_payload) =
            build_debug_child_launch_args("host-a", 9001, 3, Some("   "));

        assert_eq!(instance_id, "host-a-p9001-client-2");
        assert_eq!(
            args,
            vec![
                "--instance-id".to_string(),
                "host-a-p9001-client-2".to_string(),
                "--join-payload".to_string(),
                "pkr1_join".to_string(),
            ],
        );
        assert_eq!(instance_id_without_payload, "host-a-p9001-client-3");
        assert_eq!(
            args_without_payload,
            vec![
                "--instance-id".to_string(),
                "host-a-p9001-client-3".to_string(),
            ],
        );
    }

    #[test]
    fn start_host_session_returns_a_live_invite_from_the_running_host() {
        let state = DesktopAppState::detect();

        let status = state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");

        assert_eq!(status.tournament_name, "Friday Finals");
        assert_eq!(status.table_name, "Main Table");
        assert_eq!(status.advertised_host, "127.0.0.1");
        assert_eq!(status.phase, "waitingForPlayers");
        assert_eq!(status.active_seat_count, 1);
        assert_eq!(status.open_seat_count, 5);
        assert_eq!(status.participants.len(), 1);
        assert_eq!(status.participants[0].display_name, "Host Alpha");
        assert_eq!(status.participants[0].connection_state, "connected");
        assert!(status.invite.starts_with("pkr1_"));

        let decoded = decode_join_payload(&status.invite).expect("invite should decode");
        assert_eq!(decoded.host_address, "127.0.0.1");
        assert_eq!(decoded.host_port, status.host_port);
        assert_eq!(decoded.table_name.as_deref(), Some("Friday Finals"));
        assert_eq!(decoded.table_id, status.table_id);
        assert_eq!(decoded.session_epoch, status.session_epoch);

        let active_status = state
            .host_session_status()
            .expect("host session status should resolve")
            .expect("host session should remain active");
        assert_eq!(active_status.invite, status.invite);
    }

    #[test]
    fn stop_host_session_clears_active_host_status() {
        let state = DesktopAppState::detect();

        state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");
        state.stop_host_session().expect("host session should stop");

        assert!(state
            .host_session_status()
            .expect("host session status should resolve")
            .is_none());
    }

    #[test]
    fn detect_starts_a_fresh_host_state_after_a_prior_instance_was_hosting() {
        let prior_state = DesktopAppState::detect();

        prior_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("prior host session should start");

        let restarted_state = DesktopAppState::detect();

        assert!(restarted_state
            .host_session_status()
            .expect("restarted host status should resolve")
            .is_none());
        assert!(restarted_state
            .client_session_status()
            .expect("restarted client status should resolve")
            .is_none());
    }

    #[test]
    fn start_host_session_rejects_replacing_an_active_host_session() {
        let state = DesktopAppState::detect();

        let first_status = state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("first host session should start");

        let error = state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect_err("second host session should be rejected");

        assert_eq!(
            error,
            "stop the active host session before starting a new table"
        );
        assert_eq!(
            state
                .host_session_status()
                .expect("host session status should resolve")
                .expect("original host session should remain active")
                .invite,
            first_status.invite,
        );
    }

    #[test]
    fn join_host_session_returns_the_initial_live_snapshot() {
        let host_state = DesktopAppState::detect();
        let host_status = host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");

        let client_state = DesktopAppState::detect();
        let client_status = client_state
            .join_host_session(sample_join_host_session_request(&host_status.invite))
            .expect("client should join live host");

        assert_eq!(client_status.host_address, "127.0.0.1");
        assert_eq!(client_status.host_port, host_status.host_port);
        assert_eq!(client_status.phase, "waitingForPlayers");
        assert_eq!(client_status.active_seat_count, 1);
        assert_eq!(client_status.open_seat_count, 5);
        assert_eq!(client_status.tournament_name, "Friday Finals");
        assert!(client_status
            .participants
            .iter()
            .any(|participant| participant.is_host && participant.display_name == "Host Alpha"));
        assert!(client_status
            .participants
            .iter()
            .any(|participant| participant.display_name == "Client Bravo"));

        let refreshed_host_status = host_state
            .host_session_status()
            .expect("host session status should resolve")
            .expect("host session should remain active");
        assert!(refreshed_host_status
            .participants
            .iter()
            .any(|participant| participant.display_name == "Client Bravo"));
    }

    #[test]
    fn join_host_session_rejects_replacing_an_active_client_session() {
        let first_host_state = DesktopAppState::detect();
        let first_host_status = first_host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("first host session should start");
        let second_host_state = DesktopAppState::detect();
        let second_host_status = second_host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("second host session should start");

        let client_state = DesktopAppState::detect();
        let first_client_status = client_state
            .join_host_session(sample_join_host_session_request(&first_host_status.invite))
            .expect("client should join the first host");

        let error = client_state
            .join_host_session(sample_join_host_session_request(&second_host_status.invite))
            .expect_err("second client join should be rejected");

        assert_eq!(
            error,
            "leave the active client session before joining another table"
        );

        let active_client_status = client_state
            .client_session_status()
            .expect("client session status should resolve")
            .expect("original client session should remain active");
        assert_eq!(active_client_status.table_id, first_client_status.table_id);
        assert_eq!(
            active_client_status.host_port,
            first_client_status.host_port
        );

        assert_eq!(
            second_host_state
                .host_session_status()
                .expect("second host session status should resolve")
                .expect("second host session should remain active")
                .participants
                .len(),
            1,
        );
    }

    #[test]
    fn host_mutations_reject_missing_host_sessions_clearly() {
        let state = DesktopAppState::detect();

        assert_eq!(
            state
                .host_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 0 })
                .expect_err("claiming a host seat without a host session should fail"),
            "no active host session",
        );
        assert_eq!(
            state
                .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
                .expect_err("setting host ready state without a host session should fail"),
            "no active host session",
        );
        assert_eq!(
            state
                .host_start_tournament()
                .expect_err("starting a tournament without a host session should fail"),
            "no active host session",
        );
    }

    #[test]
    fn client_mutations_reject_missing_client_sessions_clearly() {
        let state = DesktopAppState::detect();

        assert_eq!(
            state
                .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
                .expect_err("claiming a client seat without a client session should fail"),
            "no active client session",
        );
        assert_eq!(
            state
                .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
                .expect_err("setting client ready state without a client session should fail"),
            "no active client session",
        );
    }

    #[test]
    fn client_ready_state_requires_a_claimed_seat() {
        let host_state = DesktopAppState::detect();
        let host_status = host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");
        let client_state = DesktopAppState::detect();

        client_state
            .join_host_session(sample_join_host_session_request(&host_status.invite))
            .expect("client should join the host before claiming a seat");

        assert_eq!(
            client_state
                .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
                .expect_err("readying before claiming a seat should fail"),
            "ready state requires a claimed seat",
        );

        let refreshed_client_status = client_state
            .client_session_status()
            .expect("client session status should resolve")
            .expect("client session should remain active");
        let local_participant = refreshed_client_status
            .participants
            .iter()
            .find(|participant| participant.display_name == "Client Bravo")
            .expect("client participant should remain visible");
        assert_eq!(local_participant.seat_index, None);
        assert!(!local_participant.is_ready);
    }

    #[test]
    fn live_table_view_prefers_the_authoritative_session_snapshot() {
        let host_state = DesktopAppState::detect();
        let host_status = host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");

        let client_state = DesktopAppState::detect();
        client_state
            .join_host_session(sample_join_host_session_request(&host_status.invite))
            .expect("client should join live host");

        host_state
            .host_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 0 })
            .expect("host seat claim should succeed");
        client_state
            .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
            .expect("client seat claim should succeed");
        host_state
            .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect("host ready should succeed");
        client_state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect("client ready should succeed");
        host_state
            .host_start_tournament()
            .expect("host should start the live tournament");

        let host_table_view = (0..20)
            .find_map(|_| {
                let next_view = host_state
                    .table_view(TableViewerMode::Local)
                    .expect("host live table view");
                if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
                    Some(next_view)
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    None
                }
            })
            .expect("host live table view should expose the running snapshot");
        assert_eq!(host_table_view.tournament_name, "Friday Finals");
        assert_eq!(host_table_view.table_id, host_status.table_id);
        assert_eq!(host_table_view.phase_label, "Running");
        assert_eq!(host_table_view.current_hand_number, Some(1));
        assert!(host_table_view
            .seats
            .iter()
            .all(|seat| seat.display_name != "Waiting for player"));

        let client_table_view = (0..20)
            .find_map(|_| {
                let next_view = client_state
                    .table_view(TableViewerMode::Local)
                    .expect("client live table view");
                if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
                    Some(next_view)
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    None
                }
            })
            .expect("client should observe the running live table view");
        assert_eq!(client_table_view.phase_label, "Running");
        let local_client_seat = client_table_view
            .seats
            .iter()
            .find(|seat| seat.is_local)
            .expect("client local seat should exist");
        assert_eq!(local_client_seat.seat_index, 2);
        assert_eq!(local_client_seat.hole_cards.len(), 2);
        assert!(client_table_view
            .seats
            .iter()
            .filter(|seat| !seat.is_local && seat.display_name != "Open seat")
            .all(|seat| seat.hole_cards.is_empty()));
    }

    #[test]
    fn live_table_actions_route_through_the_real_session_runtime() {
        let host_state = DesktopAppState::detect();
        let host_status = host_state
            .start_host_session_with_mode(
                sample_host_session_request("127.0.0.1"),
                HostRuntimeMode::Test,
            )
            .expect("host session should start");

        let client_state = DesktopAppState::detect();
        client_state
            .join_host_session(sample_join_host_session_request(&host_status.invite))
            .expect("client should join live host");

        client_state
            .client_claim_lobby_seat(ClaimLobbySeatRequest { seat_index: 1 })
            .expect("client seat claim should succeed");
        host_state
            .host_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect("host ready should succeed");
        client_state
            .client_set_lobby_ready_state(SetLobbyReadyStateRequest { is_ready: true })
            .expect("client ready should succeed");
        host_state
            .host_start_tournament()
            .expect("host should start the live tournament");

        let running_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let next_view = host_state
                .table_view(TableViewerMode::Local)
                .expect("host running table view before live action");
            if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
                break;
            }

            assert!(
                std::time::Instant::now() < running_deadline,
                "host should expose the running table before live action"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        loop {
            let next_view = client_state
                .table_view(TableViewerMode::Local)
                .expect("client running table view before live action");
            if next_view.phase_label == "Running" && next_view.current_hand_number == Some(1) {
                break;
            }

            assert!(
                std::time::Instant::now() < running_deadline,
                "client should expose the running table before live action"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let (acting_state, observing_state, acting_view_before, observing_view_before) = loop {
            let next_host_view = host_state
                .table_view(TableViewerMode::Local)
                .expect("host table view before live action retry");
            let next_client_view = client_state
                .table_view(TableViewerMode::Local)
                .expect("client table view before live action retry");

            let host_can_act = host_state
                .host_session
                .lock()
                .expect("host session lock")
                .as_ref()
                .and_then(|session| {
                    session
                        .host_server
                        .authoritative_state()
                        .ok()
                        .and_then(|state| state.current_hand)
                        .and_then(|hand| hand.action_window)
                        .map(|window| window.player_id == LOCAL_PLAYER_ID)
                })
                .unwrap_or(false);
            if host_can_act {
                break (
                    &host_state,
                    &client_state,
                    next_host_view.clone(),
                    next_client_view.clone(),
                );
            }

            let client_can_act = client_state
                .client_session
                .lock()
                .expect("client session lock")
                .as_ref()
                .and_then(|session| {
                    session
                        .latest_snapshot
                        .state
                        .current_hand
                        .as_ref()
                        .and_then(|hand| {
                            hand.action_window.as_ref().map(|window| {
                                window.player_id == session.latest_snapshot.local_player_id
                            })
                        })
                })
                .unwrap_or(false);
            if client_can_act {
                break (
                    &client_state,
                    &host_state,
                    next_client_view.clone(),
                    next_host_view.clone(),
                );
            }

            assert!(
                std::time::Instant::now() < deadline,
                "one live session should observe an open action window"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert_eq!(acting_view_before.phase_label, "Running");

        let acting_after_action = acting_state
            .submit_table_action(
                TableViewerMode::Local,
                DesktopTableActionKind::CheckOrCall,
                None,
            )
            .expect("live action should succeed");
        assert_eq!(acting_after_action.phase_label, "Running");
        assert!(
            acting_after_action.action_owner_label != acting_view_before.action_owner_label
                || acting_after_action.current_hand_number != acting_view_before.current_hand_number
                || acting_after_action.event_feed.len() > acting_view_before.event_feed.len()
                || acting_after_action.action_tray.is_none(),
            "live action should change the acting view"
        );

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let observer_after_action = loop {
            let next_view = observing_state
                .table_view(TableViewerMode::Local)
                .expect("observer table view after live action");
            if next_view.action_owner_label != observing_view_before.action_owner_label
                || next_view.current_hand_number != observing_view_before.current_hand_number
                || next_view.event_feed.len() > observing_view_before.event_feed.len()
            {
                break next_view;
            }

            assert!(
                std::time::Instant::now() < deadline,
                "observer should observe the live action update"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert_eq!(observer_after_action.phase_label, "Running");
        assert!(
            observer_after_action.action_owner_label != observing_view_before.action_owner_label
                || observer_after_action.current_hand_number != observing_view_before.current_hand_number
                || observer_after_action.event_feed.len() > observing_view_before.event_feed.len(),
            "observer should observe the live action update"
        );
    }

    #[test]
    fn resolve_action_request_covers_fold_check_call_bet_raise_and_all_in_paths() {
        let check_window = sample_action_window(vec![ActionType::Check, ActionType::Bet]);
        let call_window = sample_action_window(vec![ActionType::Call, ActionType::Raise]);
        let all_in_window = sample_action_window(vec![ActionType::Fold, ActionType::AllIn]);

        assert_eq!(
            resolve_action_request(&all_in_window, DesktopTableActionKind::Fold, None)
                .expect("fold should resolve"),
            (ActionType::Fold, None, "Fold"),
        );
        assert_eq!(
            resolve_action_request(&check_window, DesktopTableActionKind::CheckOrCall, None)
                .expect("check should resolve"),
            (ActionType::Check, None, "Check"),
        );
        assert_eq!(
            resolve_action_request(&call_window, DesktopTableActionKind::CheckOrCall, None)
                .expect("call should resolve"),
            (ActionType::Call, None, "Call"),
        );
        assert_eq!(
            resolve_action_request(&check_window, DesktopTableActionKind::BetOrRaise, Some(120))
                .expect("bet should resolve"),
            (ActionType::Bet, Some(120), "Bet"),
        );
        assert_eq!(
            resolve_action_request(&call_window, DesktopTableActionKind::BetOrRaise, Some(120))
                .expect("raise should resolve"),
            (ActionType::Raise, Some(120), "Raise"),
        );
        assert_eq!(
            resolve_action_request(&all_in_window, DesktopTableActionKind::AllIn, None)
                .expect("all-in should resolve"),
            (ActionType::AllIn, None, "All-in"),
        );
        assert_eq!(
            resolve_action_request(&call_window, DesktopTableActionKind::BetOrRaise, None)
                .expect_err("missing raise amount should fail"),
            "raise amount is required for bet / raise"
        );
    }

    #[test]
    fn ensure_legal_and_format_helpers_return_expected_contract_strings() {
        let window = sample_action_window(vec![ActionType::Call, ActionType::Raise]);

        assert!(ensure_legal(&window, ActionType::Raise).is_ok());
        assert_eq!(
            ensure_legal(&window, ActionType::Fold).expect_err("fold should be illegal"),
            "Fold is not legal in the current action window"
        );
        assert_eq!(
            format_phase(TournamentPhase::WaitingForPlayers),
            "Waiting for players"
        );
        assert_eq!(format_phase(TournamentPhase::Running), "Running");
        assert_eq!(format_street(StreetPhase::Preflop), "Preflop");
        assert_eq!(format_street(StreetPhase::River), "River");
        assert_eq!(format_marker(SeatMarker::Dealer), "Dealer");
        assert_eq!(format_marker(SeatMarker::BigBlind), "Big blind");
    }

    #[test]
    fn blind_schedule_presets_match_the_canonical_v1_structure() {
        let fast = blind_schedule_for_preset("fast").expect("fast blind schedule");
        let normal = blind_schedule_for_preset("normal").expect("normal blind schedule");
        let slow = blind_schedule_for_preset("slow").expect("slow blind schedule");

        for (schedule, expected_duration) in [(&fast, 180), (&normal, 300), (&slow, 480)] {
            assert_eq!(schedule.levels.len(), 12);
            assert_eq!(
                schedule.levels.first().map(|level| level.small_blind),
                Some(10)
            );
            assert_eq!(
                schedule.levels.first().map(|level| level.big_blind),
                Some(20)
            );
            assert_eq!(
                schedule.levels.last().map(|level| level.small_blind),
                Some(800)
            );
            assert_eq!(
                schedule.levels.last().map(|level| level.big_blind),
                Some(1600)
            );
            assert!(schedule
                .levels
                .iter()
                .all(|level| level.duration_seconds == expected_duration));
        }
    }

    #[test]
    fn join_tokens_are_random_and_url_safe() {
        let first = issue_join_token();
        let second = issue_join_token();

        assert!(!first.is_empty());
        assert!(!second.is_empty());
        assert_ne!(first, second);
        assert!(!first.contains(':'));
        assert!(first
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')));
    }

    #[test]
    fn debug_state_is_blocked_when_debug_tools_are_disabled() {
        let mut state = DesktopAppState::detect();
        state.bootstrap.debug_tools_enabled = false;
        state.bootstrap.screens = screen_catalog(false);

        assert_eq!(
            state
                .debug_state(TableViewerMode::Local)
                .expect_err("release mode should block debug state"),
            "debug tools are unavailable in release builds"
        );
    }

    fn snapshot_test_state() -> TournamentState {
        let blind_schedule = blind_schedule_for_preset("normal").expect("normal blind schedule");
        TournamentState {
            table_id: "table-test".to_string(),
            session_epoch: 7,
            phase: TournamentPhase::Running,
            config: TournamentConfig {
                tournament_name: "Observer Table".to_string(),
                table_name: Some("Main Table".to_string()),
                max_players: 3,
                starting_stack: 1_500,
                turn_timer_seconds: 30,
                blind_schedule: blind_schedule.clone(),
            },
            blind_schedule,
            blind_level_index: 0,
            participants: [
                (
                    "observer".to_string(),
                    ParticipantRegistryEntry {
                        identity: PlayerIdentity {
                            player_id: "observer".to_string(),
                            display_name: "Observer".to_string(),
                            signing_public_key: "sign-observer".to_string(),
                            encryption_public_key: "enc-observer".to_string(),
                            signing_key_fingerprint: "fp-observer".to_string(),
                        },
                        state: ParticipantState::EliminatedObserver,
                        connection_state: ConnectionState::Connected,
                        seat_index: Some(0),
                        admitted_at_ms: 1,
                        reconnect_token: None,
                        reconnect_expiry_ms: None,
                        is_host: false,
                    },
                ),
                (
                    "showdown".to_string(),
                    ParticipantRegistryEntry {
                        identity: PlayerIdentity {
                            player_id: "showdown".to_string(),
                            display_name: "Showdown".to_string(),
                            signing_public_key: "sign-showdown".to_string(),
                            encryption_public_key: "enc-showdown".to_string(),
                            signing_key_fingerprint: "fp-showdown".to_string(),
                        },
                        state: ParticipantState::Active,
                        connection_state: ConnectionState::Connected,
                        seat_index: Some(1),
                        admitted_at_ms: 1,
                        reconnect_token: None,
                        reconnect_expiry_ms: None,
                        is_host: false,
                    },
                ),
                (
                    "folded".to_string(),
                    ParticipantRegistryEntry {
                        identity: PlayerIdentity {
                            player_id: "folded".to_string(),
                            display_name: "Folded".to_string(),
                            signing_public_key: "sign-folded".to_string(),
                            encryption_public_key: "enc-folded".to_string(),
                            signing_key_fingerprint: "fp-folded".to_string(),
                        },
                        state: ParticipantState::Active,
                        connection_state: ConnectionState::Connected,
                        seat_index: Some(2),
                        admitted_at_ms: 1,
                        reconnect_token: None,
                        reconnect_expiry_ms: None,
                        is_host: false,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            seats: vec![
                SeatState {
                    seat_index: 0,
                    occupancy: SeatOccupancyState::Occupied,
                    tournament_state: TournamentSeatState::EliminatedObserver,
                    participant_id: Some("observer".to_string()),
                    display_name: Some("Observer".to_string()),
                    chip_count: Some(0),
                    is_ready: true,
                    marker: None,
                },
                SeatState {
                    seat_index: 1,
                    occupancy: SeatOccupancyState::Occupied,
                    tournament_state: TournamentSeatState::Active,
                    participant_id: Some("showdown".to_string()),
                    display_name: Some("Showdown".to_string()),
                    chip_count: Some(1_250),
                    is_ready: false,
                    marker: None,
                },
                SeatState {
                    seat_index: 2,
                    occupancy: SeatOccupancyState::Occupied,
                    tournament_state: TournamentSeatState::Active,
                    participant_id: Some("folded".to_string()),
                    display_name: Some("Folded".to_string()),
                    chip_count: Some(250),
                    is_ready: false,
                    marker: None,
                },
            ],
            current_hand: Some(crate::domain::HandState {
                hand_number: 9,
                cycle_phase: HandCyclePhase::Settlement,
                street: StreetPhase::Showdown,
                dealer_seat_index: 0,
                small_blind_seat_index: 1,
                big_blind_seat_index: 2,
                board_cards: vec![Card {
                    rank: Rank::Ace,
                    suit: Suit::Clubs,
                }],
                hole_cards_by_player_id: [
                    (
                        "showdown".to_string(),
                        vec![
                            Card {
                                rank: Rank::King,
                                suit: Suit::Hearts,
                            },
                            Card {
                                rank: Rank::King,
                                suit: Suit::Diamonds,
                            },
                        ],
                    ),
                    (
                        "folded".to_string(),
                        vec![
                            Card {
                                rank: Rank::Two,
                                suit: Suit::Spades,
                            },
                            Card {
                                rank: Rank::Three,
                                suit: Suit::Spades,
                            },
                        ],
                    ),
                ]
                .into_iter()
                .collect(),
                participation_by_player_id: [
                    ("showdown".to_string(), HandParticipationState::AllIn),
                    ("folded".to_string(), HandParticipationState::Folded),
                ]
                .into_iter()
                .collect(),
                betting_round: BettingRoundState {
                    street: StreetPhase::Showdown,
                    current_bet: 0,
                    min_raise_to: None,
                    max_raise_to: None,
                    pot_size: 200,
                    contributions_by_player_id: BTreeMap::new(),
                },
                action_window: None,
            }),
            hand_results: vec![HandResult {
                hand_number: 8,
                winning_player_ids: vec!["showdown".to_string()],
                pot_summaries: vec![PotSummary {
                    pot_index: 0,
                    amount: 120,
                    eligible_player_ids: vec!["showdown".to_string()],
                    winner_player_ids: vec!["showdown".to_string()],
                    odd_chip_count: 0,
                    odd_chip_awarded_to: None,
                }],
                board_cards: vec![Card {
                    rank: Rank::Queen,
                    suit: Suit::Clubs,
                }],
                revealed_hands_by_player_id: BTreeMap::new(),
                eliminated_player_ids: Vec::new(),
                final_stack_by_player_id: [("showdown".to_string(), 1_620)].into_iter().collect(),
            }],
            placements: Vec::new(),
        }
    }

    #[test]
    fn observer_table_view_shows_public_showdown_cards_but_not_folded_private_cards() {
        let state = snapshot_test_state();
        let view = build_table_view_snapshot(
            &state,
            "observer",
            TableViewerMode::Observer,
            false,
            Vec::new(),
        )
        .expect("observer table view");

        let showdown_seat = view
            .seats
            .iter()
            .find(|seat| seat.display_name == "Showdown")
            .expect("showdown seat");
        let folded_seat = view
            .seats
            .iter()
            .find(|seat| seat.display_name == "Folded")
            .expect("folded seat");

        assert!(!showdown_seat.cards_hidden);
        assert_eq!(showdown_seat.hole_cards.len(), 2);
        assert!(folded_seat.cards_hidden);
        assert!(folded_seat.hole_cards.is_empty());
    }

    #[test]
    fn hand_history_uses_stored_completed_hand_boards() {
        let mut state = snapshot_test_state();
        state
            .current_hand
            .as_mut()
            .expect("current hand")
            .board_cards = vec![Card {
            rank: Rank::Ten,
            suit: Suit::Hearts,
        }];

        let history = build_table_history_for_state(&state);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].board_cards[0].label, "Queen of Clubs");
    }

    #[test]
    fn hand_result_application_uses_final_stack_values_when_present() {
        let mut state = snapshot_test_state();
        let event = serde_json::to_value(protocol::HandResultCommitted {
            hand_number: 9,
            result: HandResult {
                hand_number: 9,
                winning_player_ids: vec!["showdown".to_string()],
                pot_summaries: vec![PotSummary {
                    pot_index: 0,
                    amount: 101,
                    eligible_player_ids: vec!["showdown".to_string()],
                    winner_player_ids: vec!["showdown".to_string()],
                    odd_chip_count: 0,
                    odd_chip_awarded_to: None,
                }],
                board_cards: vec![Card {
                    rank: Rank::Ace,
                    suit: Suit::Clubs,
                }],
                revealed_hands_by_player_id: BTreeMap::new(),
                eliminated_player_ids: Vec::new(),
                final_stack_by_player_id: [
                    ("showdown".to_string(), 1_351),
                    ("folded".to_string(), 249),
                ]
                .into_iter()
                .collect(),
            },
        })
        .expect("hand result payload");

        apply_public_event_to_snapshot(
            &mut state,
            "observer",
            protocol::ProtocolMessageType::HandResultCommittedEvent,
            &event,
        );

        assert_eq!(state.seats[1].chip_count, Some(1_351));
        assert_eq!(state.seats[2].chip_count, Some(249));
    }

    #[test]
    fn odd_chip_fallback_awards_the_extra_chip_to_the_declared_recipient() {
        let mut state = snapshot_test_state();
        state.seats[1].chip_count = Some(10);
        state.seats[2].chip_count = Some(10);
        let event = serde_json::to_value(protocol::HandResultCommitted {
            hand_number: 9,
            result: HandResult {
                hand_number: 9,
                winning_player_ids: vec!["showdown".to_string(), "folded".to_string()],
                pot_summaries: vec![PotSummary {
                    pot_index: 0,
                    amount: 5,
                    eligible_player_ids: vec!["showdown".to_string(), "folded".to_string()],
                    winner_player_ids: vec!["showdown".to_string(), "folded".to_string()],
                    odd_chip_count: 1,
                    odd_chip_awarded_to: Some("showdown".to_string()),
                }],
                board_cards: vec![Card {
                    rank: Rank::Ace,
                    suit: Suit::Clubs,
                }],
                revealed_hands_by_player_id: BTreeMap::new(),
                eliminated_player_ids: Vec::new(),
                final_stack_by_player_id: BTreeMap::new(),
            },
        })
        .expect("hand result payload");

        apply_public_event_to_snapshot(
            &mut state,
            "observer",
            protocol::ProtocolMessageType::HandResultCommittedEvent,
            &event,
        );

        assert_eq!(state.seats[1].chip_count, Some(13));
        assert_eq!(state.seats[2].chip_count, Some(12));
    }
}
