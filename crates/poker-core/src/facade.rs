use serde::{Deserialize, Serialize};

use crate::{
    domain::{ActionType, TournamentConfig, TournamentState},
    tournament::{ActionRequest, RegisteredPlayer, TournamentController},
};

#[derive(Debug, thiserror::Error)]
pub enum PokerCoreError {
    #[error("engine error: {0}")]
    Engine(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineCommand {
    SubmitAction {
        player_id: String,
        action_window_id: String,
        action_type: ActionType,
        raise_to_amount: Option<u32>,
        now_ms: u64,
    },
    AdvanceTime {
        now_ms: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineEvent {
    StateChanged,
    Noop,
}

pub struct PokerEngine {
    controller: TournamentController,
}

impl PokerEngine {
    pub fn new(
        table_id: impl Into<String>,
        session_epoch: u64,
        config: TournamentConfig,
        registered_players: Vec<RegisteredPlayer>,
    ) -> Result<Self, PokerCoreError> {
        Ok(Self {
            controller: TournamentController::new(table_id, session_epoch, config, registered_players)
                .map_err(|error| PokerCoreError::Engine(error.to_string()))?,
        })
    }

    pub fn state(&self) -> &TournamentState {
        self.controller.state()
    }

    pub fn submit_command(
        &mut self,
        command: EngineCommand,
    ) -> Result<Vec<EngineEvent>, PokerCoreError> {
        match command {
            EngineCommand::SubmitAction {
                player_id,
                action_window_id,
                action_type,
                raise_to_amount,
                now_ms,
            } => {
                self.controller
                    .submit_action(
                        ActionRequest {
                            player_id,
                            action_window_id,
                            action_type,
                            raise_to_amount,
                        },
                        now_ms,
                    )
                    .map_err(|error| PokerCoreError::Engine(error.to_string()))?;
                Ok(vec![EngineEvent::StateChanged])
            }
            EngineCommand::AdvanceTime { now_ms } => {
                self.controller
                    .advance_time(now_ms)
                    .map_err(|error| PokerCoreError::Engine(error.to_string()))?;
                Ok(vec![EngineEvent::StateChanged])
            }
        }
    }

    pub fn export_state_json(&self) -> Result<String, PokerCoreError> {
        serde_json::to_string(self.state())
            .map_err(|error| PokerCoreError::Serialization(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BlindLevel, BlindSchedule, ConnectionState, PlayerIdentity};

    fn test_config() -> TournamentConfig {
        TournamentConfig {
            tournament_name: "Test".to_string(),
            table_name: None,
            max_players: 2,
            starting_stack: 1000,
            turn_timer_seconds: 30,
            blind_schedule: BlindSchedule {
                levels: vec![BlindLevel {
                    level_index: 1,
                    label: "L1".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 300,
                }],
            },
        }
    }

    fn test_player(player_id: &str, seat_index: u8, is_host: bool) -> RegisteredPlayer {
        RegisteredPlayer {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: player_id.to_string(),
                signing_public_key: format!("sign-{player_id}"),
                encryption_public_key: format!("enc-{player_id}"),
                signing_key_fingerprint: format!("fp-{player_id}"),
            },
            seat_index,
            is_host,
            is_ready: true,
        }
    }

    fn two_player_engine() -> PokerEngine {
        PokerEngine::new(
            "table-1",
            1,
            test_config(),
            vec![
                test_player("host", 0, true),
                test_player("guest", 1, false),
            ],
        )
        .expect("engine should construct")
    }

    #[test]
    fn poker_engine_exports_state_json() {
        let engine = two_player_engine();
        let json = engine.export_state_json().unwrap();
        assert!(json.contains("phase"));
    }

    #[test]
    fn poker_engine_advance_time_command_is_deterministic() {
        let mut a = two_player_engine();
        let mut b = two_player_engine();

        let events_a = a
            .submit_command(EngineCommand::AdvanceTime { now_ms: 1234 })
            .unwrap();
        let events_b = b
            .submit_command(EngineCommand::AdvanceTime { now_ms: 1234 })
            .unwrap();

        assert_eq!(events_a, events_b);
        assert_eq!(a.export_state_json().unwrap(), b.export_state_json().unwrap());
    }
}
