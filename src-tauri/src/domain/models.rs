use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TournamentPhase {
    WaitingForPlayers,
    ReadyCheck,
    Running,
    Complete,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HandCyclePhase {
    Dealing,
    AwaitingAction,
    StreetReveal,
    Showdown,
    Settlement,
    BetweenHands,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreetPhase {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeatOccupancyState {
    Empty,
    Reserved,
    Occupied,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TournamentSeatState {
    Open,
    Lobby,
    Ready,
    Active,
    EliminatedObserver,
    Closed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HandParticipationState {
    Waiting,
    Active,
    Folded,
    AllIn,
    Out,
    EliminatedObserver,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParticipantState {
    Admitted,
    Seated,
    Active,
    Reconnecting,
    EliminatedObserver,
    Removed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    Fold,
    Check,
    Call,
    Bet,
    Raise,
    AllIn,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeatMarker {
    Dealer,
    SmallBlind,
    BigBlind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlindLevel {
    pub level_index: u8,
    pub label: String,
    pub small_blind: u32,
    pub big_blind: u32,
    pub ante: u32,
    pub duration_seconds: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlindSchedule {
    pub levels: Vec<BlindLevel>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TournamentConfig {
    pub tournament_name: String,
    pub table_name: Option<String>,
    pub max_players: u8,
    pub starting_stack: u32,
    pub turn_timer_seconds: u32,
    pub blind_schedule: BlindSchedule,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerIdentity {
    pub player_id: String,
    pub display_name: String,
    pub signing_public_key: String,
    pub encryption_public_key: String,
    pub signing_key_fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantRegistryEntry {
    pub identity: PlayerIdentity,
    pub state: ParticipantState,
    pub connection_state: ConnectionState,
    pub seat_index: Option<u8>,
    pub admitted_at_ms: u64,
    pub reconnect_token: Option<String>,
    pub reconnect_expiry_ms: Option<u64>,
    pub is_host: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeatState {
    pub seat_index: u8,
    pub occupancy: SeatOccupancyState,
    pub tournament_state: TournamentSeatState,
    pub participant_id: Option<String>,
    pub display_name: Option<String>,
    pub chip_count: Option<u32>,
    pub is_ready: bool,
    pub marker: Option<SeatMarker>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BettingRoundState {
    pub street: StreetPhase,
    pub current_bet: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub pot_size: u32,
    pub contributions_by_player_id: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionWindow {
    pub action_window_id: String,
    pub player_id: String,
    pub seat_index: u8,
    pub legal_actions: Vec<ActionType>,
    pub call_amount: u32,
    pub min_raise_to: Option<u32>,
    pub max_raise_to: Option<u32>,
    pub deadline_epoch_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PotSummary {
    pub pot_index: u8,
    pub amount: u32,
    pub eligible_player_ids: Vec<String>,
    pub winner_player_ids: Vec<String>,
    pub odd_chip_count: u32,
    pub odd_chip_awarded_to: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandResult {
    pub hand_number: u32,
    pub winning_player_ids: Vec<String>,
    pub pot_summaries: Vec<PotSummary>,
    pub board_cards: Vec<Card>,
    pub revealed_hands_by_player_id: BTreeMap<String, Vec<Card>>,
    pub eliminated_player_ids: Vec<String>,
    pub final_stack_by_player_id: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlacementEntry {
    pub player_id: String,
    pub place: u8,
    pub busted_at_hand_number: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandState {
    pub hand_number: u32,
    pub cycle_phase: HandCyclePhase,
    pub street: StreetPhase,
    pub dealer_seat_index: u8,
    pub small_blind_seat_index: u8,
    pub big_blind_seat_index: u8,
    pub board_cards: Vec<Card>,
    pub hole_cards_by_player_id: BTreeMap<String, Vec<Card>>,
    pub participation_by_player_id: BTreeMap<String, HandParticipationState>,
    pub betting_round: BettingRoundState,
    pub action_window: Option<ActionWindow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TournamentState {
    pub table_id: String,
    pub session_epoch: u64,
    pub phase: TournamentPhase,
    pub config: TournamentConfig,
    pub blind_schedule: BlindSchedule,
    pub blind_level_index: usize,
    pub participants: BTreeMap<String, ParticipantRegistryEntry>,
    pub seats: Vec<SeatState>,
    pub current_hand: Option<HandState>,
    pub hand_results: Vec<HandResult>,
    pub placements: Vec<PlacementEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JoinPayload {
    pub payload_version: u32,
    pub host_address: String,
    pub host_port: u16,
    pub table_id: String,
    pub session_epoch: u64,
    pub host_signing_public_key: String,
    pub join_token: String,
    pub generated_at_ms: u64,
    pub table_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotState {
    pub state: TournamentState,
    pub local_player_id: String,
    pub reconnect_token: Option<String>,
    pub host_signing_public_key: Option<String>,
    pub host_encryption_public_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicState {
    pub tournament_name: String,
    pub table_name: Option<String>,
    pub table_id: String,
    pub session_epoch: u64,
    pub phase: TournamentPhase,
    pub seats: Vec<SeatState>,
    pub board_cards: Vec<Card>,
    pub blind_level_label: Option<String>,
    pub current_hand_number: Option<u32>,
    pub placements: Vec<PlacementEntry>,
    pub action_window_player_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateState {
    pub public_state: PublicState,
    pub local_player_id: String,
    pub private_hole_cards: Vec<Card>,
    pub can_act: bool,
    pub is_observer: bool,
    pub action_window_player_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObserverProjection {
    pub public_state: PublicState,
    pub private_hole_cards: Vec<Card>,
    pub can_act: bool,
    pub is_observer: bool,
    pub action_window_player_id: Option<String>,
}
