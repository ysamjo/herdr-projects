//! Background operations for Mission Control: loading folders, delivering tasks, selecting priority, and live status.
use super::State;
use crate::json::*;
use crate::mission_projects::Project;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, RecvTimeoutError};
use std::time::Duration;

pub enum Command { Refresh, Submit(PathBuf, String), Priority(String), Approve(PathBuf, String), Conclusion(String), Focus(String), Mobile(bool), Fold }
pub enum Update { Notice(String), Folders(Vec<Project>), Snapshot(Value, Value), Approved(String), Sent(PathBuf, String), Applied(String), Error(String) }
pub fn start(socket: String, commands: Receiver<Command>, updates: Sender<Update>) {
    std::thread::spawn(move || {
        crate::flow::send::shim();
        crate::extras::refresh_sidebar();
        crate::mission_keeper::ensure();
        refresh(&updates);
        loop {
            match commands.recv_timeout(Duration::from_millis(1000)) {
                Ok(Command::Refresh) => refresh(&updates),
                Ok(Command::Submit(folder, text)) => match crate::mission_control::submit(&folder, &text, &socket) {
                    Ok(()) => { let _ = updates.send(Update::Sent(folder, text)); }
                    Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                },
                Ok(Command::Approve(folder, id)) => match crate::mission_conclusion::approve(&folder, &id, &socket) {
                    Ok(()) => { let _ = updates.send(Update::Approved(id)); }
                    Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                },
                Ok(Command::Conclusion(id)) => {
                    if let Some(folder) = crate::mission_control::selected_folder(&socket) {
                        match crate::mission_conclusion::request(&folder, &id, &socket, true) {
                            Ok(()) => { let _ = updates.send(Update::Notice("Fazit beim Orchestrator angefordert".into())); }
                            Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                        }
                    }
                },
                Ok(Command::Focus(target)) => view(&socket, &updates, |folder| crate::mission_view::focus(&socket, folder, &target)),
                Ok(Command::Mobile(enabled)) => view(&socket, &updates, |folder| crate::mission_view::mobile(&socket, folder, enabled)),
                Ok(Command::Fold) => view(&socket, &updates, |folder| {
                    if crate::mission_view::preferences(&socket, folder).b("mobile") { crate::mission_view::focus(&socket, folder, "orchestrator") }
                    else { crate::mission_view::command("tiles") }
                }),
                Ok(Command::Priority(text)) => match crate::mission_control::apply_priority(&text) {
                    Ok(primary) => { let _ = crate::coordinator::state::change(socket.as_str(), |s| { s.put("controlPriority", ""); true }); let _ = updates.send(Update::Applied(primary)); }
                    Err(e) => { let _ = updates.send(Update::Error(e.msg)); }
                },
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if let Err(e) = crate::mission_control::ensure_visible(&socket) { let _ = updates.send(Update::Error(e.msg)); }
            crate::mission_keeper::ensure();
            if !crate::keeper::keeper_alive() { crate::keeper::tick_once(&socket, false); }
            let tasks = crate::store::tasks::list(Default::default());
            crate::mission_agents::sync(&tasks, &socket);
            let mut coordinator = crate::mission_control::selected_summary(socket.as_str());
            let mut conclusion = Value::Null;
            if let Some(folder) = crate::mission_control::selected_folder(&socket) {
                if let Err(e) = crate::mission_view::auto(&socket, &folder) { let _ = updates.send(Update::Error(e.msg)); }
                let co = crate::mission_control::project_coordinator(&folder, &socket);
                if let Ok(co) = co { coordinator["selection"] = crate::mission_status::coordinator(&co); }
                coordinator["mobile"] = json!(crate::mission_view::preferences(&socket, &folder).b("mobile"));
                coordinator["terminals"] = json!(crate::mission_view::terminals(&socket, &folder));
                conclusion = crate::mission_conclusion::latest(&folder);
                if let Err(e) = crate::mission_conclusion::ensure(&folder, &socket, &conclusion) { let _ = updates.send(Update::Error(e.msg)); }
            }
            if updates.send(Update::Snapshot(coordinator, conclusion)).is_err() { break; }
        }
    });
}
fn view(socket: &str, updates: &Sender<Update>, action: impl FnOnce(&std::path::Path) -> crate::err::Res<()>) {
    if let Some(folder) = crate::mission_control::selected_folder(socket) {
        if let Err(e) = action(&folder) { let _ = updates.send(Update::Error(e.msg)); }
    }
}
fn refresh(updates: &Sender<Update>) {
    match crate::mission_projects::scan(&crate::mission_projects::root()) {
        Ok(mut folders) => {
            if let Ok(path) = std::env::var("HERDR_CONTROL_PROJECT_PATH") { folders.retain(|p| p.path == PathBuf::from(&path)); }
            let _ = updates.send(Update::Folders(folders));
        }
        Err(e) => { let _ = updates.send(Update::Error(format!("Projektordner können nicht geladen werden: {e}"))); }
    }
}
pub fn receive(s: &mut State, update: Update) {
    match update {
        Update::Notice(notice) => { s.notice = notice; s.error.clear(); }
        Update::Folders(folders) => {
            let wanted = s.folder().map(|p| p.path.display().to_string()).unwrap_or_else(|| crate::mission_layout::state(&s.socket).t("selected"));
            s.save_draft();
            s.projects = folders;
            let index = s.projects.iter().position(|p| p.path.display().to_string() == wanted).unwrap_or(0);
            s.project.select(None); s.choose(index);
            s.notice = "Task eingeben und senden".into();
        }
        Update::Snapshot(coordinator, conclusion) => {
            if s.conclusion.s("task") != conclusion.s("task") { s.summary_scroll = 0; }
            s.coordinator = coordinator; s.conclusion = conclusion;
        }
        Update::Approved(id) => { s.sending = false; s.error.clear(); s.notice = format!("Umsetzung für {id} freigegeben"); }
        Update::Sent(folder, text) => {
            s.sending = false;
            let path = folder.display().to_string();
            if s.folder().map(|p| p.path == folder).unwrap_or(false) && s.draft.text() == text { s.draft.set_text(""); s.save_draft(); }
            else if s.drafts.get(&path).map(|d| d == &text).unwrap_or(false) { s.drafts.remove(&path); }
            s.notice = format!("Task für {} an den Orchestrator übergeben", folder.file_name().unwrap_or_default().to_string_lossy());
        }
        Update::Applied(primary) => { s.priority = false; s.error.clear(); s.notice = format!("Reihenfolge gespeichert und angewendet · Orchestrator: {primary}"); }
        Update::Error(error) => { s.sending = false; s.error = error; }
    }
}
