//! One rule for every role: a turn cut off by its provider either waits and is tried again, or the work goes to the next
//! agent on the role's fallback list. It reads what the provider said and what is already known about its subscription
//! (store/quota.rs), and it is the only place that decides — so a worker, a reviewer and a coordinator cannot drift apart
//! (docs/fallback.md). The waits themselves are the board's own (store/retries.rs), configurable there.
use crate::store::handovers::{out_of_allowance, why_of};
use crate::store::retries::retry_waits;

/// What a cut-off turn does next.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// The same agent is asked to carry on after `ms`. `tries` counts this one: it is what the row says meanwhile.
    Wait { ms: i64, tries: i64 },
    /// Waiting gains nothing: the next agent on the list takes over. The words say why.
    Hand { why: String },
}
/// What was cut off, in the provider's own terms. `tries`: how many waits this agent has already been given here.
pub struct Cut<'a> {
    pub error: &'a str,
    pub reason: &'a str,
    pub auth: bool,
    pub passing: bool,
    pub tries: i64,
}
impl Step {
    /// The wait, when it waits: how long, and which try this is.
    pub fn waiting(&self) -> Option<(i64, i64)> { if let Step::Wait { ms, tries } = self { Some((*ms, *tries)) } else { None } }
    /// Why the work goes elsewhere, when it does.
    pub fn hand(self) -> Option<String> { if let Step::Hand { why } = self { Some(why) } else { None } }
}
/// The rule, in order of what is worth doing:
///
/// 1. A login and an empty allowance are not waited out. Another model of the same subscription is the same budget, and
///    only the human (a login) or the provider's own reset (an allowance) changes that: hand over at once.
/// 2. An error that passes by itself is waited out, with the board's growing waits, before anyone else is brought in.
/// 3. Anything else — a model that is not there, a provider that says no — is handed over with the provider's words.
///
/// With the waits used up the rule hands over too, and the words say how often it was tried.
pub fn next(c: &Cut) -> Step {
    let waits = retry_waits();
    if c.auth { return Step::Hand { why: "logged out".into() } }
    let now = crate::clock::now();
    if !c.error.is_empty() && out_of_allowance(c.error) {
        return Step::Hand { why: why_of(c.error, false, false, c.reason, c.tries, now) };
    }
    if c.passing && c.tries < waits.len() as i64 {
        return Step::Wait { ms: waits[c.tries.max(0) as usize], tries: c.tries + 1 };
    }
    Step::Hand { why: why_of(c.error, false, c.passing, c.reason, (c.tries + 1).max(1), now) }
}
/// The words for the row while it waits: "at capacity, trying again in 2m" is built where the wait is shown
/// (store/retries.rs `wait_line`), from the same numbers this rule gives.
pub fn waits() -> Vec<i64> { retry_waits() }
