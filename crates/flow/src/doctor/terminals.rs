//! Open agent terminals, for `doctor`: how many there are, how many are idle, and whether cleanup is on so they leave
//! by themselves. A terminal that is idle but stays open is a leak; one that works is expected.
use super::Found;
use crate::json::*;
use crate::store::settings::lookup::setting;
use crate::store::tasks::{list, ListOpts};

#[derive(Clone, Debug, Default)]
pub struct Count { pub working: usize, pub idle: usize, pub review: usize, pub done: usize }
/// Every task that has an open pane, by what its agent does.
pub fn ask() -> Count {
    let mut out = Count::default();
    for t in list(ListOpts { include_archived: true, attempts: true, ..Default::default() }) {
        if !t.at("link").b("pane") { continue; }
        let stage = t.s("stage");
        if stage == "reviewing" || stage == "reviewneeds" { out.review += 1; }
        else if ["done", "approval", "tried", "checking", "pushing"].contains(&stage) { out.done += 1; }
        else if ["idle", "todo", "progress", "queued", "after", "stopped", "asks", "failed", "plan"].contains(&stage) { out.idle += 1; }
        else { out.working += 1; }
    }
    out
}
pub fn check(c: &Count) -> Vec<Found> {
    let total = c.working + c.idle + c.review + c.done;
    if total == 0 { return vec![Found::ok("No agent terminals open")]; }
    let mut out = vec![Found::ok(format!("{total} agent terminal{} open ({} working, {} idle, {} reviewing, {} done)", if total == 1 { "" } else { "s" }, c.working, c.idle, c.review, c.done))];
    let cleanup = setting("workspace.cleanup", None) == json!(true);
    if !cleanup {
        out.push(Found::bad("workspace.cleanup is off: done tasks stay in the sidebar until you close them", "Turn it on: herdr-projects set workspace.cleanup true"));
    }
    if c.done > 1 {
        out.push(Found::bad(format!("{} done tasks still have open terminals", c.done), if cleanup { "The housekeeping pass closes them once their agent is quiet. If they stay, the agent may not report itself idle." } else { "Turn workspace.cleanup on, or close them in Herdr." }));
    }
    out
}
