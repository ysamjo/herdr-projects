//! What the board knows about each allowance, for the whole machine: an exhausted subscription is not preferred by the
//! next task, another model of it is the same empty budget, and an entry whose time is up is tried again.
use crate::common::*;
use sw::store::quota::{self, note, prefer, rank, release, reset_ms};

fn order(list: &[&str]) -> Vec<String> { list.iter().map(|s| s.to_string()).collect() }

#[test]
fn another_model_of_the_same_subscription_is_the_same_empty_budget() {
    let _w = world();
    note(&quota::group_of("claude"), quota::EXHAUSTED, "You've hit your usage limit", sw::clock::now() + 3600000, "proj/1");
    let list = order(&["claude/claude-opus-5-5", "claude/claude-sonnet-5", "codex", "pi"]);
    assert_eq!(prefer(&list), order(&["codex", "pi", "claude/claude-opus-5-5", "claude/claude-sonnet-5"]));
    assert_eq!(rank("claude/claude-sonnet-5"), 2);
    assert_eq!(rank("codex"), 0);
    assert!(quota::why_not("claude/claude-sonnet-5").unwrap().starts_with("out of allowance"));
    assert_eq!(quota::why_not("codex"), None);
}

#[test]
fn an_entry_whose_time_is_up_is_tried_again_without_being_cleaned_up() {
    let _w = world();
    let now = sw::clock::now();
    note(&quota::group_of("codex"), quota::EXHAUSTED, "usage limit", now + 60000, "proj/1");
    assert_eq!(rank("codex"), 2);
    sw::clock::set(Some(now + 120000));
    assert_eq!(rank("codex"), 0, "the provider named a time, and it has come");
    assert_eq!(prefer(&order(&["codex", "pi"])), order(&["codex", "pi"]));
    sw::clock::set(None);
}

#[test]
fn a_login_stays_until_a_turn_of_that_subscription_works_again() {
    let _w = world();
    note(&quota::group_of("codex"), quota::LOGIN, "401 Unauthorized", 0, "proj/1");
    assert_eq!(rank("codex"), 2);
    assert!(quota::why_not("codex").unwrap().starts_with("needs a login"));
    release(&quota::group_of("codex"));
    assert_eq!(rank("codex"), 0);
    assert_eq!(quota::why_not("codex"), None);
}

#[test]
fn a_subscription_that_was_failing_a_moment_ago_is_tried_last_but_not_refused() {
    let _w = world();
    note(&quota::group_of("pi"), quota::DEGRADED, "connection dropped", 0, "proj/1");
    assert_eq!(rank("pi"), 1);
    assert_eq!(prefer(&order(&["pi", "codex"])), order(&["codex", "pi"]));
    // Everything failing still tries the first: a list is never emptied by what is known.
    assert_eq!(prefer(&order(&["pi"])), order(&["pi"]));
}

#[test]
fn the_reset_time_the_provider_names_is_when_it_is_looked_at_again() {
    let _w = world();
    let now = sw::clock::now();
    let at = reset_ms("Your limit resets 5pm (UTC)", now);
    assert!(at > now, "a moment in the future, not a word: {at}");
    assert_eq!(reset_ms("usage limit reached", now), 0, "no time named: the board waits its own waits");
}

#[test]
fn the_next_task_does_not_go_back_to_an_empty_subscription() {
    let w = world();
    sw::mission_control::apply_priority("codex,claude").unwrap();
    let id = w.add("Ein Task");
    assert_eq!(sw::mission_control::worker_choice(&task(&id), &w.socket).unwrap(), "claude", "the list's own order");
    note(&quota::group_of("claude"), quota::EXHAUSTED, "usage limit", sw::clock::now() + 3600000, &id);
    assert_eq!(sw::mission_control::worker_choice(&task(&id), &w.socket).unwrap(), "codex", "the empty subscription is passed over");
}
