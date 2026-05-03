use std::collections::BTreeMap;

use super::{
    validate_tournament_state, DomainError, ObserverProjection, ParticipantState, PrivateState,
    PublicState, TournamentState,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionBundle {
    pub public_state: PublicState,
    pub private_states: BTreeMap<String, PrivateState>,
    pub observer_projection: ObserverProjection,
}

pub struct StateProjector;

impl StateProjector {
    pub fn project(state: &TournamentState) -> Result<ProjectionBundle, DomainError> {
        validate_tournament_state(state)?;

        let public_state = project_public_state(state);
        let action_window_player_id = public_state.action_window_player_id.clone();

        let private_states = state
            .participants
            .iter()
            .filter(|(_, participant)| participant.state != ParticipantState::Removed)
            .map(|(player_id, participant)| {
                let is_observer = participant.state == ParticipantState::EliminatedObserver;
                let private_hole_cards = if is_observer {
                    Vec::new()
                } else {
                    state
                        .current_hand
                        .as_ref()
                        .and_then(|hand| hand.hole_cards_by_player_id.get(player_id))
                        .cloned()
                        .unwrap_or_default()
                };

                let can_act = !is_observer
                    && action_window_player_id
                        .as_ref()
                        .is_some_and(|window_player_id| window_player_id == player_id);

                (
                    player_id.clone(),
                    PrivateState {
                        public_state: public_state.clone(),
                        local_player_id: player_id.clone(),
                        private_hole_cards,
                        can_act,
                        is_observer,
                        action_window_player_id: can_act.then(|| player_id.clone()),
                    },
                )
            })
            .collect();

        Ok(ProjectionBundle {
            public_state: public_state.clone(),
            private_states,
            observer_projection: ObserverProjection {
                public_state,
                private_hole_cards: Vec::new(),
                can_act: false,
                is_observer: true,
                action_window_player_id: None,
            },
        })
    }
}

fn project_public_state(state: &TournamentState) -> PublicState {
    PublicState {
        tournament_name: state.config.tournament_name.clone(),
        table_name: state.config.table_name.clone(),
        table_id: state.table_id.clone(),
        session_epoch: state.session_epoch,
        phase: state.phase,
        seats: state.seats.clone(),
        board_cards: state
            .current_hand
            .as_ref()
            .map(|hand| hand.board_cards.clone())
            .unwrap_or_default(),
        blind_level_label: state
            .blind_schedule
            .levels
            .get(state.blind_level_index)
            .map(|level| level.label.clone()),
        current_hand_number: state.current_hand.as_ref().map(|hand| hand.hand_number),
        placements: state.placements.clone(),
        action_window_player_id: state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.as_ref())
            .map(|window| window.player_id.clone()),
    }
}
