use desktop_poker_lib::domain::{ActionType, BlindLevel, Card, Rank, StreetPhase, Suit};
use desktop_poker_lib::npc::llm_client::LlmClient;
use desktop_poker_lib::npc::llm_strategy::choose_embedded_action;
use desktop_poker_lib::npc::profile::NpcProfile;
use desktop_poker_lib::npc::prompt::GameStateSnapshot;
use desktop_poker_lib::npc::provider::{
    LlmProviderConfig, LlmProviderSettings, LlmProviderType,
};
use desktop_poker_lib::npc::strategy::Position;

#[test]
fn embedded_tiny_model_selects_a_legal_npc_action() {
    let model_path = match std::env::var("DESKTOP_POKER_EMBEDDED_TEST_MODEL") {
        Ok(path) => path,
        Err(error) => {
            if std::env::var("DESKTOP_POKER_REQUIRE_EMBEDDED_MODEL").as_deref() == Ok("1") {
                panic!("CI requires DESKTOP_POKER_EMBEDDED_TEST_MODEL: {error}");
            }
            eprintln!("embedded tiny-model smoke test skipped: no model path configured");
            return;
        }
    };

    let client = LlmClient::new(LlmProviderConfig {
        settings: LlmProviderSettings {
            provider: LlmProviderType::EmbeddedLocal,
            endpoint_url: None,
            model: Some(model_path),
        },
        api_key: None,
    })
    .expect("embedded client must initialize");

    let profile = NpcProfile {
        id: "ci-aggressive".to_string(),
        name: "CI Aggressor".to_string(),
        style: "loose-aggressive".to_string(),
        skill: "beginner".to_string(),
        description: "Prefer pressure and raises, but only choose one supplied legal option."
            .to_string(),
        opponent_tendencies: None,
        tilt_behaviour: None,
    };
    let legal_actions = vec![ActionType::Fold, ActionType::Call, ActionType::Raise];
    let snapshot = GameStateSnapshot {
        hand_number: 7,
        street: StreetPhase::Preflop,
        board_cards: vec![],
        hole_cards: vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spades,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spades,
            },
        ],
        pot_total: 150,
        call_amount: 50,
        min_raise_to: Some(150),
        max_raise_to: Some(1_000),
        stack: 1_000,
        position: Position::Late,
        active_player_count: 3,
        legal_actions: legal_actions.clone(),
        blind_level: BlindLevel {
            level_index: 0,
            label: "25/50".to_string(),
            small_blind: 25,
            big_blind: 50,
            ante: 0,
            duration_seconds: 600,
        },
        street_history: vec![],
        session_context: None,
        opponent_context: None,
        tilt_description: None,
    };

    let (action, amount) =
        choose_embedded_action(&client, &profile, &snapshot).expect("model must choose an action");
    assert!(legal_actions.contains(&action), "model returned illegal {action:?}");
    if action == ActionType::Raise {
        let amount = amount.expect("raise must include an amount");
        assert!((150..=1_000).contains(&amount));
    } else {
        assert!(amount.is_none());
    }
}
