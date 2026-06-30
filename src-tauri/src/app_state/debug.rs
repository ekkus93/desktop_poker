use crate::{
    domain, tournament,
    tournament::{RegisteredPlayer, TournamentController},
};

use super::*;

impl DebugTableRuntime {
    pub(crate) fn new() -> Result<Self, String> {
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

    pub(crate) fn view(&self, viewer_mode: TableViewerMode) -> Result<TableViewSnapshot, String> {
        build_table_view_snapshot(
            self.controller.state(),
            LOCAL_PLAYER_ID,
            viewer_mode,
            true,
            self.protocol_log.iter().rev().take(10).cloned().collect(),
        )
    }

    #[cfg(test)]
    pub(crate) fn submit_action(
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

    pub(crate) fn debug_state(
        &self,
        viewer_mode: TableViewerMode,
    ) -> Result<DebugInspectorState, String> {
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
            npc_tilt_levels: std::collections::BTreeMap::new(),
            last_llm_fallback: None,
            last_npc_action_error: None,
            host_runtime_health: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn auto_play_opponents(&mut self) -> Result<(), String> {
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
    pub(crate) fn auto_play_single_actor(&mut self) -> Result<(), String> {
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

    pub(crate) fn sync_log_markers(&mut self) {
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

    pub(crate) fn log_event(&mut self, kind: impl Into<String>, message: impl Into<String>) {
        self.protocol_log.push(TableEventView {
            sequence: self.next_sequence,
            kind: kind.into(),
            message: message.into(),
        });
        self.next_sequence += 1;
    }

    #[cfg(test)]
    pub(crate) fn bump_clock(&mut self, delta_ms: u64) -> u64 {
        self.now_ms += delta_ms;
        self.now_ms
    }
}

pub(crate) fn debug_demo_controller() -> Result<TournamentController, tournament::TournamentError> {
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

pub(crate) fn registered_player(
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

pub(crate) fn resolve_action_request(
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
pub(crate) fn scripted_action_for_window(
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

pub(crate) fn ensure_legal(
    window: &domain::ActionWindow,
    action: domain::ActionType,
) -> Result<(), String> {
    if window.legal_actions.contains(&action) {
        Ok(())
    } else {
        Err(format!(
            "{} is not legal in the current action window",
            format_action(action)
        ))
    }
}
