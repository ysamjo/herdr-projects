//! Mission Control's bottom input strip and full-screen terminal navigation within one project space.
use crate::coordinator::state;
use crate::err::Res;
use crate::herdr::{Herdr, cli::args};
use crate::json::*;
use crate::store::files::{data_dir, read_obj, write_json};
use std::path::Path;

fn file(socket: &str, folder: &Path) -> std::path::PathBuf {
    data_dir().join(format!("mission-view-{}-{}.json", crate::herdr::cli::session_name(socket), crate::mission_layout::project_key(&folder.display().to_string())))
}
pub fn preferences(socket: &str, folder: &Path) -> Obj { read_obj(&file(socket, folder)).unwrap_or_default() }
fn save(socket: &str, folder: &Path, value: Obj) -> Res<()> { write_json(&file(socket, folder), &Value::Object(value)) }
fn ratio(layout: &Value) -> String { (1.0 - 9.0 / layout.at("area").i("height").max(16) as f64).clamp(0.55, 0.9).to_string() }

/// Swap positions, keeping both terminal identities: the new coordinator occupies the large upper area.
/// A layout can still name a pane that was closed a moment ago (the retired coordinator): only a live pane is split.
pub fn split_coordinator(api: &Herdr, control: &str, cwd: &str, env: &[(String, String)]) -> Res<Value> {
    let layout = api.pane_layout(control).unwrap_or_default();
    if let Some(other) = layout.arr("panes").iter().filter(|p| p.s("pane_id") != control && api.pane_get(&p.s("pane_id")).is_some())
        .max_by_key(|p| p.at("rect").i("width") * p.at("rect").i("height")) {
        return Ok(api.pane_split(other.s("pane_id"), cwd, "right", env)?);
    }
    let mut a = args(&["pane", "split", control, "--direction", "down", "--ratio", &ratio(&layout), "--cwd", cwd, "--no-focus"]);
    for (k, v) in env { a.extend(["--env".into(), format!("{k}={v}")]); }
    let pane = api.call(a)?.at("pane").clone();
    api.call(args(&["pane", "swap", "--source-pane", control, "--target-pane", pane.s("pane_id")]))?;
    Ok(pane)
}

/// Old side-by-side composers move through a temporary space; the running coordinator is never restarted.
/// Herdr does not reorient a split when `pane move` targets the same tab.
pub fn dock(api: &Herdr, folder: &Path, control: Value) -> Res<Value> {
    let Some(co) = crate::mission_control::project_coordinator(folder, &api.socket_path()).ok().and_then(state::get) else { return Ok(control) };
    let Some(agent) = api.pane_get(co.s("pane")).filter(|a| a.s("tab_id") == control.s("tab_id")) else { return Ok(control) };
    let layout = api.pane_layout(control.s("pane_id")).unwrap_or_default();
    let mine = layout.arr("panes").iter().find(|p| p.s("pane_id") == control.s("pane_id"));
    if mine.map(|p| p.at("rect").i("y") > layout.at("area").i("y")).unwrap_or(false) { return Ok(control); }
    if !api.pane_idle(control.s("pane_id")) { return Ok(control); }
    let moved = api.call(args(&["pane", "move", control.s("pane_id"), "--new-workspace", "--label", "Control-Layout", "--no-focus"]))?;
    let transit = moved.at("move_result").at("pane");
    let result = api.call(args(&["pane", "move", transit.s("pane_id"), "--tab", agent.s("tab_id"), "--split", "down", "--target-pane", agent.s("pane_id"), "--ratio", &ratio(&layout), "--no-focus"]));
    match result {
        Ok(result) => Ok(result.at("move_result").at("pane").clone()),
        Err(e) => {
            crate::mission_layout::prepare_project(&api.socket_path(), &folder.display().to_string(), transit.s("workspace_id"), transit.s("pane_id"))?;
            Err(e.into())
        }
    }
}

pub fn terminals(socket: &str, folder: &Path) -> Vec<Value> {
    let api = Herdr::new(Some(socket.into()));
    let Some(co) = crate::mission_control::project_coordinator(folder, socket).ok().and_then(state::get) else { return vec![] };
    let Some(root) = api.pane_get(co.s("pane")) else { return vec![] };
    let mut panes = vec![json!({"pane":root.at("pane_id"),"label":"Orchestrator","cli":co.at("run").s("kind"),"status":root.s("agent_status")})];
    for t in crate::store::tasks::list(Default::default()) {
        if t.b("archivedAt") || crate::store::repos::folder_of(t.s("id")) != folder || t.at("link").s("socket") != socket { continue; }
        if let Some(p) = api.pane_get(t.at("link").s("pane")).filter(|p| p.s("tab_id") == root.s("tab_id") && crate::flow::live::owns_pane(&t, p)) {
            panes.push(json!({"pane":p.at("pane_id"),"label":crate::flow::sidebar::space_label(&t),"cli":t.at("agent"),"status":crate::mission_agents::status(&t)}));
        }
    }
    panes
}
pub fn focus(socket: &str, folder: &Path, target: &str) -> Res<()> {
    let api = Herdr::new(Some(socket.into()));
    let control = crate::mission_layout::project_anchor(socket, &folder.display().to_string()).unwrap_or_default();
    let choices = terminals(socket, folder);
    let pane = match target {
        "input" => control.clone(),
        "orchestrator" => choices.first().map(|p| p.t("pane")).unwrap_or_default(),
        "next" => {
            let focused = choices.iter().position(|p| api.pane_get(p.s("pane")).map(|p| p.b("focused")).unwrap_or(false));
            choices.get(focused.map(|i| (i + 1) % choices.len()).unwrap_or(0)).map(|p| p.t("pane")).unwrap_or_default()
        }
        id => choices.iter().find(|p| p.s("pane") == id).map(|p| p.t("pane")).unwrap_or_default(),
    };
    if pane.is_empty() { crate::fail!("Dieses Terminal ist noch nicht geöffnet.") }
    let live = api.pane_get(&pane).ok_or_else(|| crate::err::Error::new("Dieses Terminal wurde geschlossen."))?;
    api.workspace_focus(live.s("workspace_id"))?; api.tab_focus(live.s("tab_id"))?;
    if target == "input" || preferences(socket, folder).b("mobile") {
        api.call(args(&["pane", "zoom", &pane, "--on"]))?;
    } else { api.pane_focus(&pane)?; }
    Ok(())
}
pub fn mobile(socket: &str, folder: &Path, enabled: bool) -> Res<()> {
    let mut p = preferences(socket, folder); p.put("mobile", enabled); p.put("manual", true); save(socket, folder, p)?;
    if enabled { focus(socket, folder, "orchestrator")?; }
    else if let Some(pane) = crate::mission_layout::project_anchor(socket, &folder.display().to_string()) {
        Herdr::new(Some(socket.into())).call(args(&["pane", "zoom", &pane, "--off"]))?;
    }
    Ok(())
}
pub fn auto(socket: &str, folder: &Path) -> Res<()> {
    let api = Herdr::new(Some(socket.into()));
    let Some(control) = crate::mission_layout::project_anchor(socket, &folder.display().to_string()) else { return Ok(()) };
    let Some(layout) = api.pane_layout(&control) else { return Ok(()) };
    if !api.workspace_get(layout.s("workspace_id")).map(|w| w.b("focused")).unwrap_or(false) { return Ok(()) }
    let mut p = preferences(socket, folder);
    if p.b("manual") { return Ok(()) }
    let small = layout.at("area").i("width") < 90 || layout.at("area").i("height") < 22;
    if p.get("mobile").is_none() || p.b("mobile") != small {
        p.put("mobile", small); save(socket, folder, p)?;
        if !terminals(socket, folder).is_empty() {
            if small { focus(socket, folder, "orchestrator")?; }
            else { api.call(args(&["pane", "zoom", &control, "--off"]))?; }
        }
    }
    Ok(())
}
pub fn command(action: &str) -> Res<()> {
    let socket = crate::herdr::cli::current_socket();
    let folder = crate::mission_control::selected_folder(&socket).ok_or_else(|| crate::err::Error::new("Bitte einen Projekt-Space auswählen."))?;
    if action == "orchestrator" && reopen(&socket, &folder)? { return Ok(()); }
    match action {
        "mobile" => mobile(&socket, &folder, true), "tiles" => mobile(&socket, &folder, false),
        target => focus(&socket, &folder, target),
    }
}

/// Only an explicit open action or a new task reopens a coordinator the human closed.
pub fn reopen(socket: &str, folder: &Path) -> Res<bool> {
    let co = crate::mission_control::project_coordinator(folder, socket)?;
    let c = state::get(&co).unwrap_or_default();
    if c.b("stopped") || !c.b("pane") || (c.b("pane") && Herdr::new(Some(socket.into())).pane_get(c.s("pane")).is_none()) {
        if !state::pumping(&c) && !c.b("want") {
            let desired = crate::mission_fallback::target_for(&co);
            if c.o("run").zip(desired.as_ref()).map(|(run, pick)| crate::coordinator::who::same_choice(run, pick)).unwrap_or(false) {
                crate::coordinator::requests::restart(&co)?;
            } else { crate::coordinator::requests::fresh(desired, &co)?; }
        }
        return Ok(true);
    }
    Ok(false)
}
