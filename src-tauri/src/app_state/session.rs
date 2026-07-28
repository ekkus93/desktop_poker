use std::time::{Duration, Instant};

use crate::{domain, networking};

use super::*;

const CLIENT_EVENT_DRAIN_POLL_TIMEOUT: Duration = Duration::from_millis(1);
const CLIENT_EVENT_WAIT_POLL_TIMEOUT: Duration = Duration::from_millis(50);
const CLIENT_ACTION_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const CLIENT_LOBBY_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const READY_CHECK_TABLE_START_POLL_TIMEOUT: Duration = Duration::from_millis(250);

impl DesktopHostSession {
    pub(crate) fn status(&self) -> Result<HostSessionStatus, String> {
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

    pub(crate) fn table_view(
        &self,
        viewer_mode: TableViewerMode,
    ) -> Result<TableViewSnapshot, String> {
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
        let mut snapshot = build_table_view_snapshot(
            &authoritative_state,
            LOCAL_PLAYER_ID,
            viewer_mode,
            true,
            event_feed,
        )?;
        snapshot.session_connection = "normal".to_string();
        Ok(snapshot)
    }

    pub(crate) fn submit_table_action(
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

impl DesktopClientSession {
    pub(crate) fn status(&mut self) -> ClientSessionStatus {
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
            terminated: self.terminated,
            last_error: self.last_error.clone(),
            participants: build_session_participants(authoritative_state),
        }
    }

    pub(crate) fn table_view(
        &mut self,
        viewer_mode: TableViewerMode,
    ) -> Result<TableViewSnapshot, String> {
        self.refresh();

        if self.terminated {
            return Err(self
                .last_error
                .clone()
                .unwrap_or_else(|| "Disconnected from host".to_string()));
        }

        if self.latest_snapshot.state.phase == domain::TournamentPhase::ReadyCheck {
            // Best-effort: wait for the table to start or an error; timeout is not an error here.
            let _ = self.await_condition(READY_CHECK_TABLE_START_POLL_TIMEOUT, |session| {
                session.last_error.is_some()
                    || session.latest_snapshot.state.phase != domain::TournamentPhase::ReadyCheck
            });
        }

        if self.terminated {
            return Err(self
                .last_error
                .clone()
                .unwrap_or_else(|| "Disconnected from host".to_string()));
        }

        let session_connection = if self.reconnecting {
            "reconnecting".to_string()
        } else {
            "normal".to_string()
        };

        let mut snapshot = build_table_view_snapshot(
            &self.latest_snapshot.state,
            &self.latest_snapshot.local_player_id,
            viewer_mode,
            true,
            self.event_feed.clone(),
        )?;
        snapshot.session_connection = session_connection;
        Ok(snapshot)
    }

    pub(crate) fn submit_table_action(
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
    }

    pub(crate) fn refresh(&mut self) {
        loop {
            match self.runtime.poll_event(CLIENT_EVENT_DRAIN_POLL_TIMEOUT) {
                Ok(event) => self.apply_event(event),
                Err(networking::ClientRuntimePollError::Timeout) => break,
                Err(networking::ClientRuntimePollError::Disconnected) => {
                    self.mark_runtime_channel_disconnected();
                    break;
                }
            }
        }
    }

    pub(crate) fn apply_event(&mut self, event: networking::ClientRuntimeEvent) {
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
            networking::ClientRuntimeEvent::ProtocolWarning { .. } => {}
            networking::ClientRuntimeEvent::SafeError { message, .. } => {
                self.reconnecting = false;
                self.last_error = Some(message);
            }
            networking::ClientRuntimeEvent::Disconnected { .. } => {
                self.reconnecting = false;
                self.terminated = true;
                self.last_error = Some("Disconnected from host".to_string());
            }
        }
    }

    pub(crate) fn claim_lobby_seat(
        &mut self,
        request: ClaimLobbySeatRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.last_error = None;
        self.runtime
            .claim_seat(request.seat_index)
            .map_err(|error| error.to_string())?;
        let observed = self.await_condition(CLIENT_LOBBY_ACK_TIMEOUT, |session| {
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
        if !observed {
            return Err(format!(
                "seat claim timed out: host did not confirm within {} seconds",
                CLIENT_LOBBY_ACK_TIMEOUT.as_secs()
            ));
        }

        Ok(self.status())
    }

    pub(crate) fn set_lobby_ready_state(
        &mut self,
        request: SetLobbyReadyStateRequest,
    ) -> Result<ClientSessionStatus, String> {
        self.last_error = None;
        self.runtime
            .set_ready_state(request.is_ready)
            .map_err(|error| error.to_string())?;
        let observed = self.await_condition(CLIENT_LOBBY_ACK_TIMEOUT, |session| {
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
        if !observed {
            return Err(format!(
                "ready-state toggle timed out: host did not confirm within {} seconds",
                CLIENT_LOBBY_ACK_TIMEOUT.as_secs()
            ));
        }

        Ok(self.status())
    }

    /// Poll until `predicate` returns true or the timeout expires.
    ///
    /// Returns `true` if the condition was observed, `false` if the timeout
    /// elapsed without the predicate being satisfied. A disconnected runtime
    /// channel terminates the session and exits immediately.
    pub(crate) fn await_condition(
        &mut self,
        timeout: Duration,
        predicate: impl Fn(&Self) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            self.refresh();
            if predicate(self) {
                return true;
            }

            match self.runtime.poll_event(CLIENT_EVENT_WAIT_POLL_TIMEOUT) {
                Ok(event) => self.apply_event(event),
                Err(networking::ClientRuntimePollError::Timeout) => {}
                Err(networking::ClientRuntimePollError::Disconnected) => {
                    self.mark_runtime_channel_disconnected();
                    return predicate(self);
                }
            }

            if predicate(self) {
                return true;
            }
        }
        false
    }

    fn mark_runtime_channel_disconnected(&mut self) {
        self.reconnecting = false;
        self.terminated = true;
        if self.last_error.is_none() {
            self.last_error = Some("Client runtime stopped unexpectedly".to_string());
        }
    }
}
