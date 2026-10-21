//! Group worker tasks under the human request that the project's orchestrator is currently handling.
use crate::coordinator::state;
use crate::json::*;
use std::path::Path;

pub fn inherit(folder: &Path, pane: &str, socket: &str) -> Option<Value> {
    if pane.is_empty() || !crate::mission_control::is_active(socket) { return None; }
    let co = crate::mission_control::project_coordinator(folder, socket).ok()?;
    let c = state::get(&co)?;
    if c.s("pane") != pane { return None; }
    let message = c.arr("where").iter().rev().find(|m| m.s("said").starts_with("[MISSION-CONTROL-PROJEKT]") || m.s("said").starts_with("[MISSION-CONTROL-UMSETZUNG-FREIGEGEBEN]"))?;
    let said = message.s("said");
    if let Some(id) = said.split("Freigegebener Recherche-Task ").nth(1).and_then(|s| s.split_whitespace().next()) {
        if let Some(research) = crate::store::tasks::get(id) {
            if research.at("missionRequest").is_object() { return Some(research.at("missionRequest").clone()); }
            return Some(json!({"id":message.at("id"),"title":research.at("title"),"at":message.at("at")}));
        }
    }
    let title = said.split("Nachricht des Menschen:\n").nth(1).unwrap_or("Auftrag");
    Some(json!({"id":message.at("id"),"title":crate::store::tasks::one_line(title, 120),"at":message.at("at")}))
}
