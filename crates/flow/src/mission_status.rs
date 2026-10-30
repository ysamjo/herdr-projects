//! German task labels and the reason the current CLI was selected or handed over.
use crate::flow::sidebar_entry::Entry;
use crate::json::*;

pub fn status(t: &Obj) -> &'static str {
    match t.s("stage") {
        "done" => "fertig", "reviewing" | "review" => "im Review", "reviewneeds" => "Review wartet auf dich",
        "asks" | "blocked" | "plan" | "approval" | "pick" => "wartet auf dich",
        "failed" | "nosummary" => "Fehler – braucht Hilfe", "queued" | "after" | "todo" => "wartet auf Start",
        "stopped" => "angehalten", _ => "arbeitet",
    }
}
pub fn entry(t: &Obj, now: i64) -> Entry { crate::flow::sidebar_entry::entry_of(t, "", now) }
pub fn decorate(t: &Obj, mut base: Entry) -> Entry {
    let tag = base.name;
    let request = t.at("missionRequest").s("title");
    let name = if request.is_empty() { tag } else { format!("{} ▸ {}", crate::store::tasks::one_line(request, 34), tag) };
    let stage = status(t);
    let cli = if t.b("on") { t.s("on") } else { t.s("agent") };
    let place = format!("{stage} · {cli}");
    let last = t.arr("handovers").last();
    let line = last.map(|h| format!("{} → {}: {}", h.s("from"), h.s("to"), h.s("why"))).unwrap_or_else(|| t.t("msg"));
    base.name = name.chars().take(80).collect(); base.place = place.clone(); base.stands = place; base.line = line.chars().take(80).collect();
    base
}
pub fn coordinator(co: &crate::coordinator::co::Co) -> Value {
    let c = crate::coordinator::state::get(co).unwrap_or_default();
    let configured = crate::mission_control::priority().split(',').next().unwrap_or("").to_string();
    let run = c.at("run");
    let actual = if run.b("model") { format!("{}/{}", run.s("kind"), run.s("model")) } else { run.t("kind") };
    let reason = if c.b("controlHandover") && c.s("controlPriority") == crate::mission_control::priority() && c.b("controlFallback") { c.t("controlHandover") }
        else if actual.is_empty() { format!("Start mit Priorität 1: {configured}") }
        else if actual != configured { format!("Aktiv: {actual}. Gewählt: {configured}; Wechsel nach dem laufenden Turn.") }
        else { format!("{actual} ist Priorität 1.") };
    json!({"actual":actual,"configured":configured,"reason":reason,"handover":c.at("controlHandover"),"error":c.at("error")})
}
