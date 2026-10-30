//! The one global CLI order: the programs it may name, the file it is kept in, and the settings it is written into
//! once — for every project, not per project.
use crate::err::Res;
use crate::json::*;
use crate::store::files::{user_home, write_atomic};
use crate::store::settings::{change::set_many, lookup::Scope};
use std::path::PathBuf;

pub const POOL: &[&str] = &["freebuff", "codebuddy", "claude", "codex", "agy", "cursor", "opencode", "kiro", "cline", "qodercli", "grok"];
pub fn priority_file() -> PathBuf {
    std::env::var("HERDR_CONTROL_CONFIG_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| user_home().join(".config")).join("herdr/plugins/config/herdr-control")
    }).join("priority.txt")
}
/// The list that was written into the settings last: the order is applied once, not on every start.
pub fn applied_file() -> PathBuf { priority_file().with_extension("applied") }
pub fn priority() -> String { std::fs::read_to_string(priority_file()).unwrap_or_else(|_| POOL.join(",")).trim().to_string() }
pub fn parse_priority(text: &str) -> Res<Vec<String>> {
    let mut order = vec![];
    for value in text.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (name, suffix) = value.split_once('/').map(|(n, m)| (n, format!("/{m}"))).unwrap_or((value, String::new()));
        let name = match name { "antigravity" | "antigravity-cli" => "agy", "qoder" => "qodercli", "gemini" => crate::fail!("Bitte agy für Antigravity CLI verwenden."), n => n };
        if !POOL.contains(&name) || value.chars().any(char::is_whitespace) { crate::fail!("Unbekannter CLI-Eintrag: {value}. Erlaubt: {}", POOL.join(", ")) }
        let on = format!("{name}{suffix}");
        if order.contains(&on) { crate::fail!("Doppelter Eintrag: {on}") }
        order.push(on);
    }
    if order.is_empty() { crate::fail!("Bitte mindestens eine CLI angeben.") }
    Ok(order)
}
/// All values are validated together before settings or the saved order change. A list already in effect is not
/// written again: a setting changed by hand afterwards stays, until the list itself changes.
pub fn apply_priority(text: &str) -> Res<String> {
    let order = parse_priority(text)?;
    let list = order.join(",");
    if std::fs::read_to_string(applied_file()).map(|s| s.trim() == list).unwrap_or(false) { return Ok(order[0].clone()); }
    let primary = &order[0];
    let workers: Vec<_> = order.iter().filter(|a| crate::store::settings::values::agent_of(a).kind != crate::store::settings::values::agent_of(primary).kind).collect();
    let Some(worker) = workers.first() else { crate::fail!("Orchestrator und Worker brauchen mindestens zwei unterschiedliche CLIs.") };
    let fallback = if workers.len() > 1 { workers[1..].iter().map(|a| a.as_str()).collect::<Vec<_>>().join(",") } else { "none".into() };
    set_many(&[
        ("worker.agent".into(), json!(worker)), ("worker.fallback".into(), json!(fallback)),
        ("coordinator.agent".into(), json!(primary)), ("watcher.agent".into(), json!("none")),
        ("review.by".into(), json!("coordinator")), ("review.when".into(), json!("always")),
    ], &Scope::you())?;
    write_atomic(&priority_file(), format!("{list}\n").as_bytes())?;
    write_atomic(&applied_file(), format!("{list}\n").as_bytes())?;
    Ok(primary.clone())
}
