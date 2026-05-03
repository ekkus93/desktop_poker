use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use dirs::data_local_dir;
use serde::{Deserialize, Serialize};

use crate::{
    crypto, domain, engine,
    engine::Deck,
    interop, networking, protocol, storage, tournament,
    tournament::{ActionRequest, RegisteredPlayer, TournamentController},
};

pub const INSTANCE_ID_ENV_VAR: &str = "DESKTOP_POKER_INSTANCE_ID";
pub const JOIN_PAYLOAD_ENV_VAR: &str = "DESKTOP_POKER_JOIN_PAYLOAD";
const INSTANCE_ID_ARG: &str = "--instance-id";
const JOIN_PAYLOAD_ARG: &str = "--join-payload";
const LOCAL_PLAYER_ID: &str = "local-player";
const OBSERVER_DISPLAY_NAME: &str = "Riley";
const OBSERVER_SEAT_INDEX: u8 = 5;

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

pub struct DesktopAppState {
    bootstrap: DesktopBootstrapState,
    table_runtime: Mutex<DesktopTableRuntime>,
    launched_instances: Mutex<u32>,
}

impl DesktopAppState {
    #[must_use]
    pub fn detect() -> Self {
        let instance_id = detect_instance_id();
        let profile_directory = detect_profile_directory(&instance_id);
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
                instance_id,
                profile_directory: profile_directory.display().to_string(),
                launch_join_payload,
                parsed_launch_join_payload,
                launch_join_payload_error,
                debug_tools_enabled,
                backend_modules: backend_modules(),
                screens: screen_catalog(debug_tools_enabled),
            },
            table_runtime: Mutex::new(
                DesktopTableRuntime::new().expect("desktop table runtime should initialize"),
            ),
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
        self.table_runtime
            .lock()
            .map_err(|_| "table runtime lock poisoned".to_string())?
            .view(viewer_mode)
    }

    pub fn submit_table_action(
        &self,
        viewer_mode: TableViewerMode,
        action_kind: DesktopTableActionKind,
        raise_to_amount: Option<u32>,
    ) -> Result<TableViewSnapshot, String> {
        self.table_runtime
            .lock()
            .map_err(|_| "table runtime lock poisoned".to_string())?
            .submit_action(viewer_mode, action_kind, raise_to_amount)
    }

    pub fn debug_state(&self, viewer_mode: TableViewerMode) -> Result<DebugInspectorState, String> {
        self.table_runtime
            .lock()
            .map_err(|_| "table runtime lock poisoned".to_string())?
            .debug_state(viewer_mode)
    }

    pub fn launch_additional_client_instance(&self) -> Result<String, String> {
        if !cfg!(debug_assertions) {
            return Err("debug launch helper is only available in debug builds".to_string());
        }

        let mut launch_counter = self
            .launched_instances
            .lock()
            .map_err(|_| "launch counter lock poisoned".to_string())?;
        *launch_counter += 1;
        let instance_id = format!("debug-client-{}", *launch_counter);
        let current_executable = env::current_exe().map_err(|error| error.to_string())?;
        let current_directory = env::current_dir().map_err(|error| error.to_string())?;

        Command::new(current_executable)
            .arg(INSTANCE_ID_ARG)
            .arg(&instance_id)
            .current_dir(current_directory)
            .spawn()
            .map_err(|error| error.to_string())?;

        Ok(instance_id)
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
            "Detects launch context, profile namespace, backend module map, screen catalog, and the desktop shell table runtime.",
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

fn detect_profile_directory(instance_id: &str) -> PathBuf {
    let base = data_local_dir()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    base.join("desktop-poker")
        .join("profiles")
        .join(instance_id)
}

struct DesktopTableRuntime {
    controller: TournamentController,
    protocol_log: Vec<TableEventView>,
    now_ms: u64,
    next_sequence: u64,
    last_board_count: usize,
    last_hand_result_count: usize,
    last_action_window_id: Option<String>,
    last_hand_number: Option<u32>,
}

impl DesktopTableRuntime {
    fn new() -> Result<Self, String> {
        let mut controller = demo_controller().map_err(|error| error.to_string())?;
        let demo_deck = stacked_demo_deck().map_err(|error| error.to_string())?;
        controller.set_next_deck(demo_deck);
        controller
            .start_tournament(1)
            .map_err(|error| error.to_string())?;

        let mut runtime = Self {
            controller,
            protocol_log: Vec::new(),
            now_ms: 1,
            next_sequence: 1,
            last_board_count: 0,
            last_hand_result_count: 0,
            last_action_window_id: None,
            last_hand_number: None,
        };
        runtime.log_event(
            "runtime",
            "Desktop table runtime initialized from the Rust tournament controller.",
        );
        runtime.drive_to_interesting_local_turn()?;
        runtime.sync_log_markers();
        Ok(runtime)
    }

    fn view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        let projection = domain::StateProjector::project(self.controller.state())
            .map_err(|error| error.to_string())?;
        let public_state = projection.public_state.clone();
        let private_projection = projection
            .private_states
            .get(LOCAL_PLAYER_ID)
            .cloned()
            .ok_or_else(|| "local player projection missing".to_string())?;
        let current_hand = self.controller.state().current_hand.as_ref();
        let betting_round = current_hand.as_ref().map(|hand| &hand.betting_round);
        let board_cards = public_state
            .board_cards
            .iter()
            .map(card_view)
            .collect::<Vec<_>>();
        let action_window = current_hand.and_then(|hand| hand.action_window.clone());
        let action_owner_label = public_state
            .action_window_player_id
            .as_deref()
            .and_then(|player_id| display_name_for_state(self.controller.state(), player_id))
            .unwrap_or_else(|| "Waiting for settlement".to_string());

        let seats = self.build_seats(viewer_mode, &private_projection)?;
        let standings = self.build_standings();
        let hand_history = self.build_history();
        let event_feed = self.protocol_log.iter().rev().take(10).cloned().collect();
        let pot_total = betting_round
            .map(|round| round.pot_size)
            .unwrap_or_default();
        let observer_banner = matches!(viewer_mode, TableViewerMode::Observer).then(|| {
            "Observer mode uses the public projector only: no private hole cards and no actions."
                .to_string()
        });
        let elimination_summary = format!(
            "{OBSERVER_DISPLAY_NAME} busted on hand 8 and now remains at the table as a public-only observer."
        );

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
            board_cards,
            pot_total,
            action_owner_label: action_owner_label.clone(),
            elimination_summary,
            observer_banner,
            seats,
            standings,
            hand_history,
            event_feed,
            action_tray: if matches!(viewer_mode, TableViewerMode::Local)
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
            launch_hint:
                "Spawn another debug client with its own storage namespace to test multi-instance flows."
                    .to_string(),
        })
    }

    fn drive_to_interesting_local_turn(&mut self) -> Result<(), String> {
        for _ in 0..24 {
            self.sync_log_markers();
            let current_hand = self
                .controller
                .state()
                .current_hand
                .as_ref()
                .ok_or_else(|| "current hand missing".to_string())?;
            if current_hand.board_cards.len() >= 3
                && current_hand
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

    fn build_seats(
        &self,
        viewer_mode: TableViewerMode,
        private_projection: &domain::PrivateState,
    ) -> Result<Vec<TableSeatView>, String> {
        let current_hand = self.controller.state().current_hand.as_ref();
        let contributions = current_hand
            .as_ref()
            .map(|hand| hand.betting_round.contributions_by_player_id.clone())
            .unwrap_or_default();

        let mut seats = self
            .controller
            .state()
            .seats
            .iter()
            .map(|seat| {
                if seat.occupancy == domain::SeatOccupancyState::Empty {
                    if seat.seat_index == OBSERVER_SEAT_INDEX {
                        return Ok(TableSeatView {
                            seat_index: seat.seat_index + 1,
                            display_name: OBSERVER_DISPLAY_NAME.to_string(),
                            chip_count: Some(0),
                            status_label: "Eliminated observer".to_string(),
                            marker_label: Some("Observer".to_string()),
                            contribution: 0,
                            is_local: false,
                            is_acting: false,
                            is_observer: true,
                            is_eliminated: true,
                            is_compact: true,
                            cards_hidden: true,
                            hole_cards: Vec::new(),
                            detail_lines: vec![
                                "Busted on hand 8 after finishing 4th.".to_string(),
                                "Observer seats remain public-only and never regain action authority."
                                    .to_string(),
                            ],
                        });
                    }

                    return Ok(TableSeatView {
                        seat_index: seat.seat_index + 1,
                        display_name: "Open seat".to_string(),
                        chip_count: None,
                        status_label: "Waiting for a LAN participant".to_string(),
                        marker_label: None,
                        contribution: 0,
                        is_local: false,
                        is_acting: false,
                        is_observer: false,
                        is_eliminated: false,
                        is_compact: true,
                        cards_hidden: true,
                        hole_cards: Vec::new(),
                        detail_lines: vec!["Available for a real join payload or reconnect path.".to_string()],
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
                let is_local = player_id == LOCAL_PLAYER_ID;
                let visible_cards = if is_local && matches!(viewer_mode, TableViewerMode::Local) {
                    private_projection
                        .private_hole_cards
                        .iter()
                        .map(card_view)
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let cards_hidden = !(is_local && matches!(viewer_mode, TableViewerMode::Local));
                let contribution = contributions.get(player_id).copied().unwrap_or_default();
                let participant = self
                    .controller
                    .state()
                    .participants
                    .get(player_id)
                    .ok_or_else(|| "participant missing from registry".to_string())?;

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
                    is_observer: false,
                    is_eliminated: false,
                    is_compact: !is_local,
                    cards_hidden,
                    hole_cards: visible_cards,
                    detail_lines: vec![
                        format!(
                            "Connection: {}",
                            format_connection_state(participant.connection_state)
                        ),
                        format!("Seat state: {}", format_tournament_seat_state(seat.tournament_state)),
                        format!("Contribution: {contribution}"),
                    ],
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        seats.sort_by_key(|seat| seat.seat_index);
        Ok(seats)
    }

    fn build_standings(&self) -> Vec<TableStandingView> {
        let mut standings = self
            .controller
            .state()
            .seats
            .iter()
            .filter(|seat| seat.occupancy == domain::SeatOccupancyState::Occupied)
            .map(|seat| TableStandingView {
                rank: 0,
                display_name: seat
                    .display_name
                    .clone()
                    .unwrap_or_else(|| "Player".to_string()),
                chip_count: seat.chip_count,
                status_label: format_tournament_seat_state(seat.tournament_state),
                note: seat.marker.map(format_marker),
                is_local: seat.participant_id.as_deref() == Some(LOCAL_PLAYER_ID),
                is_observer: false,
            })
            .collect::<Vec<_>>();
        standings.push(TableStandingView {
            rank: 0,
            display_name: OBSERVER_DISPLAY_NAME.to_string(),
            chip_count: Some(0),
            status_label: "Eliminated observer".to_string(),
            note: Some("Busted on hand 8".to_string()),
            is_local: false,
            is_observer: true,
        });
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

    fn build_history(&self) -> Vec<TableHistoryEntryView> {
        self.controller
            .state()
            .hand_results
            .iter()
            .rev()
            .map(|result| TableHistoryEntryView {
                hand_number: result.hand_number,
                summary: format!(
                    "{} won {} chip(s).",
                    display_names_for_state(self.controller.state(), &result.winning_player_ids)
                        .join(", "),
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
                winning_players: display_names_for_state(
                    self.controller.state(),
                    &result.winning_player_ids,
                ),
                eliminated_players: display_names_for_state(
                    self.controller.state(),
                    &result.eliminated_player_ids,
                ),
                board_cards: self
                    .controller
                    .state()
                    .current_hand
                    .as_ref()
                    .map(|hand| hand.board_cards.iter().map(card_view).collect())
                    .unwrap_or_default(),
            })
            .collect()
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

    fn bump_clock(&mut self, delta_ms: u64) -> u64 {
        self.now_ms += delta_ms;
        self.now_ms
    }
}

fn demo_controller() -> Result<TournamentController, tournament::TournamentError> {
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
            registered_player("host-player", "Host", 0, true),
            registered_player(LOCAL_PLAYER_ID, "You", 1, false),
            registered_player("guest-player", "Maya", 2, false),
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

fn stacked_demo_deck() -> Result<Deck, engine::EngineError> {
    let prefix = vec![
        card(domain::Rank::King, domain::Suit::Hearts),
        card(domain::Rank::Ace, domain::Suit::Spades),
        card(domain::Rank::Queen, domain::Suit::Clubs),
        card(domain::Rank::Ten, domain::Suit::Hearts),
        card(domain::Rank::Ace, domain::Suit::Diamonds),
        card(domain::Rank::Queen, domain::Suit::Diamonds),
        card(domain::Rank::Two, domain::Suit::Clubs),
        card(domain::Rank::Jack, domain::Suit::Spades),
        card(domain::Rank::Nine, domain::Suit::Hearts),
        card(domain::Rank::Three, domain::Suit::Clubs),
        card(domain::Rank::Four, domain::Suit::Spades),
    ];
    let mut cards = prefix.clone();
    cards.extend(
        Deck::standard_52()
            .cards()
            .iter()
            .copied()
            .filter(|card| !prefix.contains(card)),
    );
    Deck::from_cards(cards)
}

fn card(rank: domain::Rank, suit: domain::Suit) -> domain::Card {
    domain::Card { rank, suit }
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
    use super::{
        demo_controller, detect_profile_directory, screen_catalog, stacked_demo_deck,
        DesktopAppState, DesktopTableActionKind, TableViewerMode, INSTANCE_ID_ENV_VAR,
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

    #[test]
    fn demo_runtime_exposes_local_and_observer_views() {
        let state = DesktopAppState::detect();
        let local_view = state
            .table_view(TableViewerMode::Local)
            .expect("local view");
        let observer_view = state
            .table_view(TableViewerMode::Observer)
            .expect("observer view");

        assert!(local_view.action_tray.is_some());
        assert!(observer_view.action_tray.is_none());
        assert!(observer_view.observer_banner.is_some());
        assert!(local_view
            .seats
            .iter()
            .find(|seat| seat.is_local)
            .is_some_and(|seat| !seat.cards_hidden));
        assert!(observer_view
            .seats
            .iter()
            .find(|seat| seat.is_local)
            .is_some_and(|seat| seat.cards_hidden));
    }

    #[test]
    fn demo_runtime_actions_append_history() {
        let state = DesktopAppState::detect();
        let initial_history_len = state
            .table_view(TableViewerMode::Local)
            .expect("initial view")
            .hand_history
            .len();

        for _ in 0..3 {
            state
                .submit_table_action(
                    TableViewerMode::Local,
                    DesktopTableActionKind::CheckOrCall,
                    None,
                )
                .expect("check or call should succeed");
        }

        let updated_view = state
            .table_view(TableViewerMode::Local)
            .expect("updated view");
        assert!(updated_view.hand_history.len() > initial_history_len);
        assert!(!updated_view.event_feed.is_empty());
    }

    #[test]
    fn demo_controller_uses_deterministic_deck() {
        let mut controller = demo_controller().expect("demo controller");
        controller.set_next_deck(stacked_demo_deck().expect("stacked deck"));
        controller.start_tournament(1).expect("start tournament");
        let local_cards = controller
            .state()
            .current_hand
            .as_ref()
            .and_then(|hand| hand.hole_cards_by_player_id.get("local-player"))
            .expect("local cards");
        assert_eq!(local_cards.len(), 2);
    }
}
