use std::collections::BTreeMap;

use super::super::{settle_showdown, HandCategory, HandStrength, SettlementResult};

// ----- 3.1 Test helpers -----

fn contributions(entries: &[(&str, u32)]) -> BTreeMap<String, u32> {
    entries
        .iter()
        .map(|(id, amount)| ((*id).to_string(), *amount))
        .collect()
}

fn hand(category: HandCategory, key_ranks: [u8; 5]) -> HandStrength {
    HandStrength {
        category,
        key_ranks,
    }
}

fn strengths(entries: &[(&str, HandStrength)]) -> BTreeMap<String, HandStrength> {
    entries
        .iter()
        .map(|(id, strength)| ((*id).to_string(), *strength))
        .collect()
}

fn order(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_string()).collect()
}

fn total_payout(result: &SettlementResult) -> u32 {
    result.payouts_by_player_id.values().sum()
}

fn total_contributions(entries: &[(&str, u32)]) -> u32 {
    entries.iter().map(|(_, amount)| *amount).sum()
}

// ----- 3.2 Basic payouts -----

#[test]
fn single_player_takes_uncontested_pot() {
    let contribs = [("alice", 50)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[("alice", hand(HandCategory::OnePair, [10, 9, 8, 7, 0]))]),
        &order(&["alice"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.payouts_by_player_id["alice"], 50);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn best_hand_wins_whole_pot_heads_up() {
    let contribs = [("alice", 50), ("bob", 50)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("alice", hand(HandCategory::Straight, [9, 0, 0, 0, 0])),
            ("bob", hand(HandCategory::OnePair, [14, 13, 12, 0, 0])),
        ]),
        &order(&["alice", "bob"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.payouts_by_player_id["alice"], 100);
    assert_eq!(result.payouts_by_player_id.get("bob"), None);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

// ----- 3.3 Split pots and odd chips -----

#[test]
fn tied_hands_split_pot_evenly() {
    let contribs = [("alice", 50), ("bob", 50)];
    let tie = hand(HandCategory::Straight, [9, 0, 0, 0, 0]);
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[("alice", tie), ("bob", tie)]),
        &order(&["alice", "bob"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.payouts_by_player_id["alice"], 50);
    assert_eq!(result.payouts_by_player_id["bob"], 50);
    assert_eq!(result.pot_summaries[0].odd_chip_count, 0);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn odd_chip_goes_to_first_in_odd_chip_order() {
    // A single 15-chip pot split between two tied winners leaves one odd chip.
    let contribs = [("alice", 5), ("bob", 5), ("carol", 5)];
    let tie = hand(HandCategory::Straight, [9, 0, 0, 0, 0]);
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("alice", tie),
            ("bob", tie),
            ("carol", hand(HandCategory::HighCard, [8, 7, 6, 5, 3])),
        ]),
        &order(&["bob", "alice", "carol"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.pot_summaries[0].odd_chip_count, 1);
    assert_eq!(
        result.pot_summaries[0].odd_chip_awarded_to.as_deref(),
        Some("bob")
    );
    assert_eq!(result.payouts_by_player_id["bob"], 8);
    assert_eq!(result.payouts_by_player_id["alice"], 7);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn odd_chip_order_is_respected_for_three_way_split() {
    // Four contributors, one folded; the 40-chip pot splits three ways, leaving
    // one odd chip for the first eligible winner in odd-chip order.
    let contribs = [("a", 10), ("b", 10), ("c", 10), ("d", 10)];
    let tie = hand(HandCategory::Flush, [14, 11, 9, 6, 3]);
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[("a", tie), ("b", tie), ("c", tie)]),
        &order(&["c", "a", "b", "d"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.pot_summaries[0].amount, 40);
    assert_eq!(result.pot_summaries[0].odd_chip_count, 1);
    assert_eq!(
        result.pot_summaries[0].odd_chip_awarded_to.as_deref(),
        Some("c")
    );
    assert_eq!(result.payouts_by_player_id["c"], 14);
    assert_eq!(result.payouts_by_player_id["a"], 13);
    assert_eq!(result.payouts_by_player_id["b"], 13);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

// ----- 3.4 Multiple all-ins / side pots -----

#[test]
fn two_all_ins_create_main_and_side_pot() {
    let contribs = [("short", 40), ("big1", 100), ("big2", 100)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("short", hand(HandCategory::OnePair, [5, 4, 3, 2, 0])),
            ("big1", hand(HandCategory::OnePair, [6, 4, 3, 2, 0])),
            ("big2", hand(HandCategory::OnePair, [7, 4, 3, 2, 0])),
        ]),
        &order(&["short", "big1", "big2"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.pot_summaries.len(), 2);
    // Main pot: 40 from each of three contributors.
    assert_eq!(result.pot_summaries[0].amount, 120);
    assert_eq!(result.pot_summaries[0].eligible_player_ids.len(), 3);
    // Side pot: 60 more from the two deep stacks only.
    assert_eq!(result.pot_summaries[1].amount, 120);
    assert_eq!(
        result.pot_summaries[1].eligible_player_ids,
        vec!["big1".to_string(), "big2".to_string()]
    );
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn short_all_in_cannot_win_side_pot() {
    // The short stack has the best hand but can only win the main pot; the side
    // pot is contested by the deeper stacks alone.
    let contribs = [("short", 40), ("big1", 100), ("big2", 100)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("short", hand(HandCategory::StraightFlush, [9, 0, 0, 0, 0])),
            ("big1", hand(HandCategory::ThreeOfAKind, [8, 5, 4, 0, 0])),
            ("big2", hand(HandCategory::OnePair, [7, 5, 4, 3, 0])),
        ]),
        &order(&["short", "big1", "big2"]),
    )
    .expect("settlement should succeed");

    // Short wins only the 120 main pot, not the 120 side pot.
    assert_eq!(result.payouts_by_player_id["short"], 120);
    assert_eq!(result.payouts_by_player_id["big1"], 120);
    assert_eq!(result.payouts_by_player_id.get("big2"), None);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn three_distinct_all_in_amounts_create_two_side_pots() {
    let contribs = [("a", 20), ("b", 50), ("c", 100)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("a", hand(HandCategory::OnePair, [5, 0, 0, 0, 0])),
            ("b", hand(HandCategory::OnePair, [6, 0, 0, 0, 0])),
            ("c", hand(HandCategory::OnePair, [7, 0, 0, 0, 0])),
        ]),
        &order(&["a", "b", "c"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.pot_summaries.len(), 3);
    // Main pot: 20 * 3.
    assert_eq!(result.pot_summaries[0].amount, 60);
    assert_eq!(result.pot_summaries[0].eligible_player_ids.len(), 3);
    // First side pot: (50-20) * 2.
    assert_eq!(result.pot_summaries[1].amount, 60);
    assert_eq!(result.pot_summaries[1].eligible_player_ids.len(), 2);
    // Second side pot: (100-50) * 1.
    assert_eq!(result.pot_summaries[2].amount, 50);
    assert_eq!(
        result.pot_summaries[2].eligible_player_ids,
        vec!["c".to_string()]
    );
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn side_pot_won_by_different_player_than_main_pot() {
    let contribs = [("short", 40), ("big1", 100), ("big2", 100)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[
            ("short", hand(HandCategory::StraightFlush, [9, 0, 0, 0, 0])),
            ("big1", hand(HandCategory::ThreeOfAKind, [8, 5, 4, 0, 0])),
            ("big2", hand(HandCategory::OnePair, [7, 5, 4, 3, 0])),
        ]),
        &order(&["short", "big1", "big2"]),
    )
    .expect("settlement should succeed");

    assert_eq!(
        result.pot_summaries[0].winner_player_ids,
        vec!["short".to_string()]
    );
    assert_eq!(
        result.pot_summaries[1].winner_player_ids,
        vec!["big1".to_string()]
    );
}

// ----- 3.5 Folded contributors and edge cases -----

#[test]
fn folded_contributor_chips_count_but_cannot_win() {
    // Bob contributed but has no hand strength (folded); his chips swell the pot
    // but he cannot be paid.
    let contribs = [("alice", 50), ("bob", 50)];
    let result = settle_showdown(
        &contributions(&contribs),
        &strengths(&[("alice", hand(HandCategory::OnePair, [10, 9, 8, 7, 0]))]),
        &order(&["alice", "bob"]),
    )
    .expect("settlement should succeed");

    assert_eq!(result.payouts_by_player_id["alice"], 100);
    assert_eq!(result.payouts_by_player_id.get("bob"), None);
    assert_eq!(total_payout(&result), total_contributions(&contribs));
}

#[test]
fn empty_contributions_or_strengths_are_rejected() {
    let with_hand = strengths(&[("alice", hand(HandCategory::OnePair, [10, 9, 8, 7, 0]))]);

    assert!(settle_showdown(&BTreeMap::new(), &with_hand, &order(&["alice"])).is_err());
    assert!(settle_showdown(
        &contributions(&[("alice", 50)]),
        &BTreeMap::new(),
        &order(&["alice"]),
    )
    .is_err());
}

#[test]
fn zero_contributions_produce_no_pots_or_payouts() {
    // Only positive contributions form thresholds, so an all-zero board settles
    // to an empty result rather than erroring.
    let result = settle_showdown(
        &contributions(&[("alice", 0), ("bob", 0)]),
        &strengths(&[("alice", hand(HandCategory::OnePair, [10, 9, 8, 7, 0]))]),
        &order(&["alice", "bob"]),
    )
    .expect("settlement should succeed");

    assert!(result.pot_summaries.is_empty());
    assert_eq!(total_payout(&result), 0);
}
