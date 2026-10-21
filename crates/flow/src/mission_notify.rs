//! Telling a running project coordinator what the board could not do. Without it the orchestrator waits for a result
//! that never comes, and only the task's own row shows why.
use crate::json::*;
use crate::store::repos::folder_of;
use crate::store::tasks::Task;

/// A worker that could not be started: said in the coordinator's own conversation, once per cause. Only a coordinator
/// that is running hears it; a closed one is not woken for it, its task's row keeps the failure.
pub fn start_failed(t: &Task, short: &str, next: &str) -> bool {
    let link = t.at("link").s("socket");
    let socket = if link.is_empty() { crate::herdr::cli::current_socket() } else { link.to_string() };
    if !crate::mission_control::is_active(&socket) { return false; }
    let Some(co) = crate::mission_control::coordinator_for_task(t, &socket) else { return false };
    let c = crate::coordinator::state::get(&co).unwrap_or_default();
    if c.b("stopped") || c.b("want") { return false; }
    if crate::herdr::Herdr::new(Some(socket.clone())).agent_get(c.s("pane")).ok().flatten().is_none() { return false; }
    let folder = folder_of(t.s("id"));
    let text = format!(
        "[MISSION-CONTROL-PROJEKT]\nProjekt: {}\nOrdner: {}\nEin Worker konnte nicht gestartet werden. Task {} („{}“). Grund: {short}\nNächster Schritt: {next}\nPrüfe den Task mit `board show {}` und sein Terminal in diesem Space. Wähle wenn möglich eine andere Worker-CLI und delegiere denselben Auftrag neu, oder sage dem Menschen klar, was fehlt. Lege für denselben Auftrag keinen zweiten Task an; führe die übrigen Teilaufgaben weiter.",
        folder.file_name().unwrap_or_default().to_string_lossy(), folder.display(), t.s("id"), t.s("title"), t.s("id")
    );
    crate::coordinator::requests::send(&folder, &text, false, t.s("id"), &co).is_ok()
}
