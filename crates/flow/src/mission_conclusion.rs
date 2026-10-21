//! Saved project conclusions and approval of the exact research report the human has read.
use crate::err::Res;
use crate::json::*;
use crate::store::{files, tasks};
use std::path::{Path, PathBuf};

pub fn file(t: &Obj) -> PathBuf { crate::research::report_folder(t).join("conclusion.json") }
pub fn read(t: &Obj) -> Option<Value> {
    let path = file(t);
    if std::fs::metadata(&path).ok()?.len() > 64 * 1024 { return None; }
    let value = files::read_json(&path)?;
    if value.s("task") != t.s("id") || value.s("conclusion").trim().is_empty() || value.s("proposal").trim().is_empty()
        || !value.at("findings").is_array() || !value.arr("findings").iter().all(Value::is_string)
        || !value.at("questions").is_array() || !value.arr("questions").iter().all(Value::is_string) { return None; }
    Some(value)
}
pub fn brief(t: &Obj) -> String {
    format!("\nSpeichere dein Fazit zusätzlich dauerhaft als JSON in {}. Format: {{\"task\":\"{}\",\"conclusion\":\"Fazit\",\"findings\":[\"konkreter Befund\"],\"proposal\":\"begrenzter Umsetzungsvorschlag\",\"questions\":[\"offene Frage, falls vorhanden\"]}}. Leere Fragenliste ist erlaubt. Antworte dem Menschen anschließend auch sichtbar im Terminal. Die Schaltfläche „Umsetzung freigeben“ wird erst nach einem gültigen gespeicherten Fazit aktiv.", file(t).display(), t.s("id"))
}
pub fn latest(folder: &Path) -> Value {
    let tasks = tasks::list(Default::default());
    let eligible: Vec<_> = tasks.iter().filter(|t| crate::store::repos::folder_of(t.s("id")) == folder && !t.b("archivedAt")).collect();
    let pending = eligible.iter().find(|t| t.s("stage") == "done" && t.s("missionImplementationApproval") == "pending");
    let selected = pending.copied().or_else(|| eligible.iter().copied().find(|t| t.at("review").is_object() || t.s("missionImplementationApproval") == "approved"));
    let Some(t) = selected else { return Value::Null };
    let research = crate::research::is_research(t);
    let summary = if research { read(t) } else { None };
    let ready = summary.is_some();
    let review = t.at("review").s("text");
    json!({"task":t.at("id"),"title":t.at("title"),"pending":t.s("missionImplementationApproval") == "pending", "research":research,
        "ready": if research { ready } else { true }, "conclusion":summary.as_ref().map(|s| s.t("conclusion")).unwrap_or_else(|| if research { "Das Fazit wird vom Orchestrator erstellt.".into() } else if !review.is_empty() { review.into() } else { t.t("msg") }),
        "findings":summary.as_ref().map(|s| s.at("findings").clone()).unwrap_or_else(|| json!([])),
        "proposal":summary.as_ref().map(|s| s.t("proposal")).unwrap_or_default(),"questions":summary.as_ref().map(|s| s.at("questions").clone()).unwrap_or_else(|| json!([])),
        "report":t.at("report").at("link"),"stage":crate::mission_agents::status(t),"review":review})
}
pub fn validate(folder: &Path, id: &str) -> Res<Obj> {
    let t = tasks::get(id).ok_or_else(|| crate::err::Error::new("Der Rechercheauftrag existiert nicht mehr."))?;
    if crate::store::repos::folder_of(id) != folder || t.s("stage") != "done" || t.b("archivedAt")
        || !crate::research::is_research(&t) || t.s("missionImplementationApproval") != "pending" || !t.at("report").is_object() {
        crate::fail!("Dieser Rechercheauftrag wartet in diesem Projekt nicht auf eine Freigabe.")
    }
    if read(&t).is_none() { crate::fail!("Das gespeicherte Fazit fehlt noch oder ist ungültig. Bitte zuerst das Fazit des Orchestrators abwarten.") }
    Ok(t)
}
pub fn approve(folder: &Path, id: &str, socket: &str) -> Res<()> {
    let folder = std::fs::canonicalize(folder)?;
    files::with_lock_wait(&format!("mission-approval:{id}"), 5000, || {
        validate(&folder, id)?;
        crate::mission_control::submit_report(&folder, "Go für Umsetzung", socket, Some(id))
    })
}

/// Reports completed before the conclusion UI existed need one fresh request in their existing coordinator.
pub fn request(folder: &Path, id: &str, socket: &str, force: bool) -> Res<()> {
    files::with_lock_wait(&format!("mission-conclusion:{id}"), 5000, || {
        let t = tasks::get(id).ok_or_else(|| crate::err::Error::new("Der Rechercheauftrag existiert nicht mehr."))?;
        if crate::store::repos::folder_of(id) != folder || !crate::research::is_research(&t) || !t.at("report").is_object() || t.s("missionImplementationApproval") != "pending" || t.s("stage") != "done" || t.b("archivedAt") { crate::fail!("Dieser Auftrag wartet nicht auf ein Fazit.") }
        if !force && (t.b("missionConclusionRequested") || read(&t).is_some()) { return Ok(()) }
        let folder = crate::store::repos::folder_of(id);
        let co = crate::mission_control::project_coordinator(&folder, socket)?;
        let prompt = format!("[MISSION-CONTROL-RESEARCH-RESULT]\nDer Rechercheauftrag {id} („{}“) wartet auf dein gespeichertes Fazit. Lies den vollständigen Bericht unter {}. Berichtlink: {}. Erkläre Befunde, Umsetzungsvorschlag und offene Fragen. Es wurde noch nichts umgesetzt. Warte danach auf das Go für Umsetzung; lege keine Umsetzungstasks an.{}", t.s("title"), crate::research::report_folder(&t).join(t.at("report").s("file")).display(), t.at("report").s("link"), brief(&t));
        crate::coordinator::requests::send(&folder, &prompt, false, id, &co)?;
        tasks::patch(id, |t| t.put("missionConclusionRequested", true))?;
        Ok(())
    })
}
pub fn ensure(folder: &Path, socket: &str, summary: &Value) -> Res<()> {
    if !summary.b("pending") || summary.b("ready") { return Ok(()) }
    let workspace = crate::mission_layout::project_workspace(socket, &folder.display().to_string()).unwrap_or_default();
    if !crate::herdr::Herdr::new(Some(socket.into())).workspace_get(&workspace).map(|w| w.b("focused")).unwrap_or(false) { return Ok(()) }
    request(folder, summary.s("task"), socket, false)
}
