//! One rule for every role: a turn cut off by its provider either waits and is tried again, or the work goes to the next
//! agent on the role's list — decided in one place for a worker, a reviewer and a coordinator alike (store/rule.rs).
use sw::store::rule::{next, waits, Cut, Step};

fn cut<'a>(error: &'a str, passing: bool) -> Cut<'a> {
    Cut { error, reason: "model is at capacity", auth: false, passing, tries: 0 }
}

#[test]
fn an_error_that_passes_by_itself_is_waited_out_before_anyone_else_is_brought_in() {
    let w = waits();
    assert!(w.len() > 1, "growing waits, not one: {w:?}");
    assert_eq!(next(&cut("Selected model is at capacity", true)).waiting(), Some((w[0], 1)));
    // Each try gets the next wait, and counts what came before.
    assert_eq!(next(&Cut { tries: 1, ..cut("Selected model is at capacity", true) }).waiting(), Some((w[1], 2)));
}

#[test]
fn with_the_waits_used_up_the_rule_hands_over_and_says_how_often_it_tried() {
    let used = waits().len() as i64;
    let why = next(&Cut { tries: used, ..cut("Selected model is at capacity", true) }).hand().unwrap();
    assert_eq!(why, format!("its model is at capacity, still after {} tries", used + 1));
}

#[test]
fn a_login_hands_over_at_once_and_is_never_waited_out() {
    assert_eq!(next(&Cut { auth: true, ..cut("401 Unauthorized", true) }).hand().unwrap(), "logged out");
}

#[test]
fn an_empty_allowance_is_not_waited_out_even_when_the_provider_s_words_sound_temporary() {
    let step = next(&cut("You've hit your usage limit. It resets 5pm", true));
    assert_eq!(step.waiting(), None, "waiting would only spend the same empty budget");
    assert!(step.hand().unwrap().starts_with("out of allowance until "));
}

#[test]
fn a_model_that_is_not_there_hands_over_at_once() {
    assert_eq!(next(&cut("model not found", false)).hand().unwrap(), "its model is not available");
}

#[test]
fn anything_else_is_handed_over_with_the_provider_s_own_words() {
    let why = next(&cut("the request was flagged", false)).hand().unwrap();
    assert_eq!(why, "it stopped (the request was flagged)");
}

#[test]
fn a_wait_is_a_wait_for_every_role() {
    // The rule has no idea which role is asking: the same words, the same waits (that is the point of it).
    assert_eq!(next(&cut("connection dropped", true)), Step::Wait { ms: waits()[0], tries: 1 });
}
