//! The `doctor` check for open agent terminals: how many there are, what they do, and whether cleanup is on.
use crate::common::*;
use serde_json::json;
use sw::doctor::terminals::{self, Count};

#[test]
fn no_terminals_means_nothing_open() {
    let c = Count::default();
    let r = terminals::check(&c);
    assert_eq!(r.len(), 1);
    assert!(!r[0].problem);
    assert!(r[0].what.contains("No agent terminals"));
}

#[test]
fn working_and_reviewing_are_fine_idle_and_done_are_counted() {
    let c = Count { working: 2, idle: 1, review: 1, done: 0 };
    let r = terminals::check(&c);
    assert_eq!(r.len(), 1);
    assert!(r[0].what.contains("4 agent terminals open (2 working, 1 idle, 1 reviewing, 0 done)"));
}

#[test]
fn cleanup_off_is_a_problem() {
    let _w = world_with(Opts { repo: true, settings: json!({ "workspace.cleanup": false }), ..Default::default() });
    let c = Count { working: 1, idle: 0, review: 0, done: 2 };
    let r = terminals::check(&c);
    assert!(r.iter().any(|f| f.problem && f.what.contains("workspace.cleanup is off")));
    assert!(r.iter().any(|f| f.problem && f.what.contains("2 done tasks") && f.fix.contains("workspace.cleanup")));
}

#[test]
fn many_done_tasks_are_warned_even_with_cleanup_on() {
    let c = Count { working: 1, idle: 0, review: 0, done: 5 };
    let r = terminals::check(&c);
    let bad = r.iter().find(|f| f.problem && f.what.contains("done tasks")).unwrap();
    assert!(bad.fix.contains("housekeeping"), "{}", bad.fix);
}
