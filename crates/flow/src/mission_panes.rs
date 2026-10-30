//! Retired Mission Control coordinators: save their output, then close only their own resting panes.
use crate::coordinator::{co::Co, state};
use crate::err::Res;
use crate::herdr::{Herdr, api::read_screen};
use crate::json::*;
use crate::store::files::{read_obj, write_atomic};
use std::collections::HashSet;

pub fn tidy(socket: &str) -> Res<()> {
    let api = Herdr::new(Some(socket.into()));
    if !crate::mission_control::is_active(socket) { return Ok(()); }
    for co in state::all(socket) {
        let current = state::get(&co).unwrap_or_default();
        if api.agent_get(current.s("pane")).ok().flatten().is_none() { continue; }
        let dir = state::dir_of(&co);
        let records: Vec<std::path::PathBuf> = std::fs::read_dir(&dir).into_iter().flatten().flatten()
            .map(|e| e.path()).filter(|p| p.file_name().map(|n| n.to_string_lossy().starts_with("handover-")).unwrap_or(false) && p.extension().and_then(|e| e.to_str()) == Some("json")).collect();
        // A pane a record names is left to that record: herdr may give a closed pane's id to another pane.
        let mut recorded: HashSet<String> = HashSet::new();
        for path in &records {
            let old = read_obj(path).unwrap_or_default();
            recorded.insert(old.t("pane"));
            if old.s("pane") == current.s("pane") || !old.b("terminal") { continue; }
            let Some(pane) = api.pane_get(old.s("pane")) else { continue };
            if pane.s("terminal_id") != old.s("terminal") || pane.s("tab_id") != current.s("tab") || std::path::Path::new(pane.s("cwd")) != state::home(&co) { continue; }
            let agent = api.agent_get(old.s("pane")).ok().flatten();
            let resting = agent.as_ref().map(|a| ["idle", "done"].contains(&a.s("agent_status"))).unwrap_or_else(|| api.pane_idle(old.s("pane")));
            if !resting { continue; }
            let log = path.with_extension("terminal.txt");
            let screen = read_screen(&api, old.s("pane"));
            write_atomic(&log, screen.as_bytes())?;
            api.pane_close(old.s("pane"));
        }
        // Two alike from a restart that left its predecessor behind: no record names it, its agent's name does.
        for a in orphans(&api, &co, &current, &recorded) {
            let log = dir.join(format!("orphan-{}.terminal.txt", crate::clock::now()));
            write_atomic(&log, read_screen(&api, a.s("pane_id")).as_bytes())?;
            api.pane_close(a.s("pane_id"));
        }
    }
    Ok(())
}
/// Panes that are no longer the one in the coordinator's state file: only a resting agent of its own name, in its own
/// tab and its own folder, and no record naming it. The task composer beside it has no agent, a worker's has another
/// name.
fn orphans(api: &Herdr, co: &Co, current: &Obj, recorded: &HashSet<String>) -> Vec<Value> {
    let own = co.agent_name();
    let home = state::home(co);
    // `sw-coordinator-x`, or the `-2` herdr gave a second one of that name (pump.rs).
    let named = |n: &str| n == own || n.strip_prefix(&own).and_then(|r| r.strip_prefix('-')).map(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).unwrap_or(false);
    api.agents().unwrap_or_default().into_iter().filter(|a| {
        a.s("pane_id") != current.s("pane") && a.s("tab_id") == current.s("tab") && named(a.s("name")) && !recorded.contains(a.s("pane_id"))
            && std::path::Path::new(a.s("cwd")) == home && ["idle", "done"].contains(&a.s("agent_status"))
    }).collect()
}
