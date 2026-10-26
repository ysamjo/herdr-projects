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
fn a_coordinator_whose_error_passes_by_itself_is_asked_to_carry_on_before_another_program_takes_over() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    w.patch(|st| { for p in st["panes"].as_array_mut().unwrap() { if p.s("pane_id") == pane { p["agent"]["screen"] = json!("⏺ API Error: 529 overloaded"); p["agent"]["status"] = json!("done"); } } });
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    // The one rule every role follows (store/rule.rs): what passes by itself is waited out before anyone is replaced.
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert_eq!(c.at("run").s("kind"), "agy", "no hand-over on the first sign of trouble");
    assert!(c.at("controlRetry").b("until"), "{}", c.at("controlRetry"));
    assert!(!prompts(&w, &pane).iter().any(|m| m.contains("Der Anbieter meldet")), "the wait is not over");
    // Waited out: the same coordinator is asked to carry on in its own session, and keeps the task.
    let until = c.at("controlRetry").i("until");
    sw::clock::set(Some(until + 1000));
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    sw::clock::set(None);
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert!(prompts(&w, &pane).iter().any(|m| m.contains("Der Anbieter meldet")), "{:?}", prompts(&w, &pane));
    assert_eq!((c.at("run").s("kind"), c.at("controlRetry").i("tries")), ("agy", 1));
    // With the waits used up the next program on the order takes over, with the hand-over file as before.
    for _ in 0..6 {
        let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
        if c.at("run").s("kind") != "agy" { break }
        sw::clock::set(Some(c.at("controlRetry").i("until") + 1000));
        sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    }
    sw::clock::set(None);
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex", "after the waits: the next program");
    assert!(prompts(&w, &pane).iter().any(|m| m.contains("Providerwechsel")));
}

#[test]
fn a_coordinator_silent_crash_is_detected_and_hands_over_with_git_and_terminal_context() {
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    // Simulate process crashing silently with non-zero exit code on screen
    w.patch(|st| {
        for p in st["panes"].as_array_mut().unwrap() {
            if p.s("pane_id") == pane {
                p["agent"]["screen"] = json!("Build failed.\nfatal error: out of memory\nprocess exited with code 137");
                p["agent"]["status"] = json!("done");
            }
        }
    });
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex", "silent crash triggers immediate handover to next program");
    let told = prompts(&w, &pane);
    assert!(told.iter().any(|m| m.contains("Providerwechsel") && m.contains("Prozess-Abbruch erkannt")), "{told:?}");
    assert!(told.iter().any(|m| m.contains("Letzte Konsolenausgabe des vorigen Orchestrators")), "{told:?}");
}

#[test]
fn a_coordinator_idle_timeout_is_detected_and_hands_over() {
    unsafe { std::env::set_var("HERDR_CONTROL_IDLE_TIMEOUT_MS", "5000"); }
    let w = world(); apply_priority("agy,codex").unwrap(); mission(&w);
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    let pane = sw::coordinator::state::get(coordinator(&w)).unwrap().t("pane");
    submit(&w.root, "Ein Auftrag der hängt", &w.socket).unwrap(); drain();
    // Advance clock past the idle timeout
    sw::clock::set(Some(sw::clock::now() + 10_000));
    sw::mission_fallback::tick(&w.socket).unwrap(); drain();
    sw::clock::set(None);
    let c = sw::coordinator::state::get(coordinator(&w)).unwrap();
    assert_eq!(c.at("run").s("kind"), "codex", "hanging coordinator hands over after idle timeout");
    let told = prompts(&w, &pane);
    assert!(told.iter().any(|m| m.contains("Stille-Erkennung (Idle Timeout)")), "{told:?}");
    unsafe { std::env::remove_var("HERDR_CONTROL_IDLE_TIMEOUT_MS"); }
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
