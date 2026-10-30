//! What each subscription's allowance is known to be, for the whole machine and every project: a provider that is out of
//! allowance, or needs a login, is no longer preferred by another task, and another model of the same subscription does
//! not land in the same empty budget. Only what a provider itself said is kept; how much is left is never guessed.
//!
//! It is a record, not a decision made by a model: what a turn ended with is written down here, and every place that
//! picks an agent reads it (mission_control::worker_choice, mission_fallback). A subscription whose time is up is tried
//! again by itself — nothing has to be cleaned up by hand.
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use crate::store::tasks::one_line;
use chrono::{Local, TimeZone, Timelike};
use std::path::PathBuf;

/// It answers.
pub const AVAILABLE: &str = "available";
/// It fails in a way that passes by itself (overloaded, a rate limit, a dropped connection): avoided for a while, not
/// refused — the board's own tries may well succeed.
pub const DEGRADED: &str = "degraded";
/// Out of allowance: nothing of this subscription is preferred before `until`.
pub const EXHAUSTED: &str = "exhausted";
/// A login is needed: only the human can lift this, so it stays until a turn of that subscription works again.
pub const LOGIN: &str = "login";

/// How long a subscription is left alone when its provider named no reset time: as long as the board's own tries take.
const UNKNOWN_RESET_MS: i64 = 900000;

fn file() -> PathBuf { data_dir().join("quota.json") }

/// The allowance a program draws on. Models of one subscription share it, so another model of the same program is the
/// same empty budget. A program not listed here draws on its own.
pub fn group_of(kind: &str) -> String {
    match kind {
        "claude" => "anthropic",
        "codex" => "openai",
        "agy" | "gemini" => "google",
        "copilot" => "github",
        "cursor" => "cursor",
        "cline" => "cline",
        "kiro" => "aws",
        "devin" => "cognition",
        "droid" => "factory",
        "grok" => "xai",
        "qwen" => "alibaba",
        other => other,
    }
    .to_string()
}
/// The same for an agent as it is written in a list (`codex`, `claude/claude-opus-5-5`).
pub fn group_of_agent(agent: &str) -> String { group_of(&crate::store::settings::values::agent_of(agent).kind) }

/// Every subscription the board has heard of and whose time is not up. A login stays until it works again.
pub fn all() -> Obj {
    let now = crate::clock::now();
    read_obj(&file()).unwrap_or_default().into_iter().filter(|(_, v)| v.s("state") == LOGIN || v.i("until") > now).collect()
}
/// What is known about one subscription, if anything.
pub fn of(group: &str) -> Option<Value> { all().get(group).cloned() }
/// What a turn ended with: `state`, why, and when it is worth looking again. `until` 0: the provider named no time, so
/// the board waits as long as its own tries take.
pub fn note(group: &str, state: &str, cause: &str, until: i64, task: &str) {
    let now = crate::clock::now();
    let until = if state == LOGIN { 0 } else if until > now { until } else { now + UNKNOWN_RESET_MS };
    let mut a = all();
    a.put(group, json!({ "state": state, "cause": one_line(cause, 200), "until": until, "at": now, "task": task }));
    let _ = write_json(&file(), &Value::Object(a));
}
/// It worked again: nothing is known against it any more.
pub fn release(group: &str) {
    let mut a = all();
    if a.contains_key(group) { a.del(group); let _ = write_json(&file(), &Value::Object(a)); }
}
/// 0: free, or nothing known against it. 1: temporarily disturbed. 2: out of allowance, or a login is needed.
pub fn rank(agent: &str) -> u8 {
    match all().get(&group_of_agent(agent)) {
        Some(v) if [EXHAUSTED, LOGIN].contains(&v.s("state")) => 2,
        Some(v) if v.s("state") == DEGRADED => 1,
        _ => 0,
    }
}
/// What the provider said about the allowance itself, as opposed to an error that passes by itself. The words are read
/// in one place only (store/handovers.rs), so the record and the hand-over's line never disagree.
pub fn out_of_allowance(words: &str) -> bool { crate::store::handovers::out_of_allowance(words) }
/// When the provider said its allowance comes back, as a moment, from its own words (store/handovers.rs `reset_at`).
/// 0 when it named none: then the board waits as long as its own tries take.
pub fn reset_ms(words: &str, now: i64) -> i64 {
    let said = crate::store::handovers::reset_at(words, now);
    let mut p = said.split(':');
    let (Some(h), Some(m)) = (p.next().and_then(|x| x.parse::<u32>().ok()), p.next().and_then(|x| x.parse::<u32>().ok())) else { return 0 };
    let Some(day) = Local.timestamp_millis_opt(now).single().map(|d| d.date_naive()) else { return 0 };
    let Some(at) = day.and_hms_opt(h, m, 0) else { return 0 };
    let ms = Local.from_local_datetime(&at).single().map(|d| d.timestamp_millis()).unwrap_or(0);
    if ms <= now { ms + 86400000 } else { ms } // that time today is gone: it is tomorrow's
}
/// When it is worth looking again, in the words of the row: " until 17:00"; "" when no time is known.
fn until_words(at: i64) -> String {
    if at <= crate::clock::now() { return String::new(); }
    Local.timestamp_millis_opt(at).single().map(|d| format!(" until {:02}:{:02}", d.hour(), d.minute())).unwrap_or_default()
}
/// Why this agent's allowance is not to be preferred now, in plain words; None when nothing speaks against it.
pub fn why_not(agent: &str) -> Option<String> {
    let v = all().get(&group_of_agent(agent))?.clone();
    let cause = v.t("cause");
    let cause = if cause.is_empty() { String::new() } else { format!(" ({cause})") };
    Some(match v.s("state") {
        LOGIN => format!("needs a login{cause}"),
        EXHAUSTED => format!("out of allowance{}", until_words(v.i("until"))),
        DEGRADED => format!("was failing a moment ago{cause}"),
        _ => return None,
    })
}
/// The same order, with what is not to be preferred now put last: the list's own order is kept within each rank, so
/// nothing is dropped — with everything out of allowance the first is still tried, and fails where you can see it.
pub fn prefer(list: &[String]) -> Vec<String> {
    let mut out: Vec<String> = list.to_vec();
    out.sort_by_key(|a| rank(a)); // stable
    out
}
