//! What Mission Control does when the orchestrator is not there or a program cannot be started: the orchestrator hears
//! about a worker that never came up, a program herdr cannot start as an agent runs in its own pane instead, and a
//! coordinator the human closed stays closed until something asks for it.
use crate::common::*;
use serde_json::json;
use sw::herdr::Herdr;
use sw::json::*;
use sw::mission_control::{apply_priority, submit};

fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn mission(w: &World) -> String {
    unsafe { std::env::set_var("HERDR_PROJECTS_COORDINATOR_DIR", w.base.join("mission-coordinator")); }
    let api = Herdr::new(Some(w.socket.clone()));
    let ws = api.workspace_create(&w.root_str(), "proj", &[]).unwrap();
    let pane = ws.at("root_pane").t("pane_id");
    sw::mission_layout::register_project(&w.socket, &w.root_str(), &ws.at("workspace").t("workspace_id"), &pane).unwrap();
    sw::mission_layout::selected(&w.socket, &w.root_str()).unwrap();
    pane
}
fn coordinator(w: &World) -> sw::coordinator::co::Co { sw::mission_control::project_coordinator(&w.root, &w.socket).unwrap() }

#[test]
fn a_worker_that_cannot_be_started_is_reported_in_the_orchestrator_session() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    w.patch(|st| { st["missing"] = json!(["codex"]); });
    let id = w.add("Ein Ergebnis prüfen");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.s("msg").contains("codex"), "{}", t.s("msg"));
    // The orchestrator waits for a result that never comes: it is told, in its own conversation.
    let said = prompts(&w, &pane);
    assert!(said.iter().any(|m| m.contains(&id) && m.contains("nicht gestartet")), "{said:?}");
    assert_eq!(sw::coordinator::state::get(coordinator(&w)).unwrap().s("pane"), pane, "the same session hears it");
}

#[test]
fn a_worker_whose_failure_is_the_same_again_is_told_once() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    w.patch(|st| { st["missing"] = json!(["codex"]); });
    let id = w.add("Zweimal scheitern");
    delegate(&id);
    let _ = settle(&id);
    delegate(&id); // "delegate again": the same cause is not a new message to the orchestrator
    let t = settle(&id);
    assert_eq!(t.s("stage"), "failed");
    let told = prompts(&w, &pane).iter().filter(|m| m.contains(&id) && m.contains("nicht gestartet")).count();
    assert_eq!(told, 1, "{:?}", prompts(&w, &pane));
}

#[test]
fn a_coordinator_pane_closed_by_hand_is_not_opened_again_by_the_tick() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    Herdr::new(Some(w.socket.clone())).pane_close(&pane);
    for _ in 0..3 { sw::mission_control::ensure(&w.socket).unwrap(); drain(); }
    assert!(sw::coordinator::state::get(coordinator(&w)).unwrap().b("stopped"));
    assert_eq!(w.state().arr("panes").iter().filter(|p| p.at("agent").is_object()).count(), 0, "a closed coordinator is not started by itself");
    // A new task does open it again: that is the human asking for it.
    submit(&w.root, "Neue Aufgabe", &w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert!(!c.b("stopped"));
    assert!(c.b("pane") && Herdr::new(Some(w.socket.clone())).agent_get(c.s("pane")).unwrap().is_some());
}

#[test]
fn a_coordinator_whose_program_herdr_cannot_start_as_an_agent_runs_in_its_pane() {
    let w = world(); apply_priority("freebuff,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert_eq!(c.at("run").s("kind"), "freebuff");
    assert_eq!(agent_of(&w, c.s("pane")).s("kind"), "freebuff", "freebuff reports its state in the pane");
    assert!(calls(&w).iter().any(|x| x.starts_with("pane run") && x.contains("HERDR_PROJECTS_AGENT=")), "{:?}", calls(&w));
    assert!(!calls(&w).iter().any(|x| x.starts_with("agent start") && x.contains("freebuff")), "never through agent start");
}
