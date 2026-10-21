//! Clickable input, terminal navigation, focus mode, and conclusion controls.
use super::{State, worker::Command};
use crate::json::*;
use ratatui::{Frame, layout::Rect, style::{Color, Style}, widgets::Paragraph};

#[derive(Clone)]
pub enum Action { Send, Image, Priority, Conclusion, Approve(String), RequestConclusion(String), Focus(String), Mobile(bool), Expand }
pub fn act(s: &mut State, action: Action) {
    match action {
        Action::Image => {
            if s.attaching || s.sending { return; }
            if let Some(folder) = s.folder() {
                let folder = folder.path.clone();
                s.attaching = true;
                let _ = s.tx.send(Command::Image(folder));
                s.notice = "Bild aus der Mac-Zwischenablage wird angehängt …".into();
            }
        }
        Action::Send => { if s.priority { let _ = s.tx.send(Command::Priority(s.order.text())); } else { s.send(); } },
        Action::Priority => {
            s.priority = !s.priority;
            if s.priority { s.order.set_text(&crate::mission_control::priority()); let _ = s.tx.send(Command::Focus("input".into())); }
            else { let _ = s.tx.send(Command::Fold); }
        }
        Action::Conclusion => {
            s.expanded = !s.expanded; s.summary_scroll = 0;
            let _ = s.tx.send(if s.expanded { Command::Focus("input".into()) } else { Command::Fold });
        }
        Action::Approve(id) => {
            if s.sending { return; }
            if let Some(folder) = s.folder() {
                let folder = folder.path.clone(); s.sending = true;
                let _ = s.tx.send(Command::Approve(folder, id));
            }
        }
        Action::RequestConclusion(id) => { let _ = s.tx.send(Command::Conclusion(id)); }
        Action::Focus(id) => { let _ = s.tx.send(Command::Focus(id)); }
        Action::Mobile(enabled) => { let _ = s.tx.send(Command::Mobile(enabled)); }
        Action::Expand => { let _ = s.tx.send(Command::Focus("input".into())); }
    }
}
pub fn row(f: &mut Frame, s: &mut State, area: Rect, choices: Vec<(String, Action)>) {
    let mut x = area.x; let mut y = area.y;
    for (label, action) in choices {
        let text = format!("[{label}] ");
        let width = crate::text::width(&text) as u16;
        if x + width > area.right() { x = area.x; y += 1; }
        if y >= area.bottom() { break; }
        let rect = Rect::new(x, y, width.min(area.width), area.height.min(1));
        f.render_widget(Paragraph::new(text).style(Style::default().fg(Color::Cyan)), rect);
        s.controls.push((rect, action)); x += width;
    }
}
pub fn navigation(s: &State) -> Vec<(String, Action)> {
    let mut choices = vec![("Orchestrator".into(), Action::Focus("orchestrator".into())), ("Weiter".into(), Action::Focus("next".into())), ("Groß".into(), Action::Expand)];
    for (i, terminal) in s.coordinator.arr("terminals").iter().skip(1).enumerate() {
        choices.push((format!("{}:{}", i + 1, terminal.s("cli")), Action::Focus(terminal.t("pane"))));
    }
    choices
}
pub fn actions(s: &State) -> Vec<(String, Action)> {
    let mut choices = vec![(if s.sending { "Sendet …" } else { "Senden" }.into(), Action::Send)];
    choices.push((if s.attaching { "Bild lädt …" } else { "Bild einfügen" }.into(), Action::Image));
    if s.conclusion.b("pending") && s.conclusion.b("ready") {
        choices.push(("Umsetzung freigeben".into(), Action::Approve(s.conclusion.t("task"))));
    }
    if s.conclusion.b("pending") && !s.conclusion.b("ready") {
        choices.push(("Fazit anfordern".into(), Action::RequestConclusion(s.conclusion.t("task"))));
    }
    if !s.conclusion.is_null() { choices.push((if s.expanded { "Fazit schließen" } else { "Fazit" }.into(), Action::Conclusion)); }
    choices.push(("CLI".into(), Action::Priority));
    choices.push((if s.coordinator.b("mobile") { "Kacheln" } else { "Fokus" }.into(), Action::Mobile(!s.coordinator.b("mobile"))));
    choices
}
