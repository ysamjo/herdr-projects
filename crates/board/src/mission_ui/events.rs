//! Keyboard and mouse actions in Mission Control, using the board's existing key definitions and editor.
use super::{State, worker::Command, controls::{self, Action}};
use crate::board::keys::{self, K};
use crossterm::event::{Event, MouseEventKind, MouseButton};
use std::time::{Duration, Instant};

pub fn handle(s: &mut State, event: Event) -> bool {
    match event {
        Event::Paste(text) => { if s.prompting() { s.order.insert(&text); } else { s.sidebar = false; s.draft.insert(&text); } }
        Event::Key(key) => {
            let Some(p) = crate::board::run::input::press_of(&key) else { return false };
            if keys::is(&p, K::Close) {
                if s.closed_at.map(|at| at.elapsed() < Duration::from_secs(2)).unwrap_or(false) { return true; }
                s.closed_at = Some(Instant::now()); s.notice = "Erneut schließen: Mission Control schließen; laufende Agenten bleiben erhalten".into(); return false;
            }
            if keys::is(&p, K::Settings) { controls::act(s, Action::Priority); return false; }
            if keys::is(&p, K::ChatMessage) { controls::act(s, Action::Conclusion); return false; }
            if s.expanded && keys::is(&p, K::ChatScrollUp) { s.summary_scroll = s.summary_scroll.saturating_sub(3); return false; }
            if s.expanded && keys::is(&p, K::ChatScrollDown) { s.summary_scroll = s.summary_scroll.saturating_add(3); return false; }
            if s.expanded && keys::is(&p, K::Back) { controls::act(s, Action::Conclusion); return false; }
            if s.priority {
                if keys::is(&p, K::Back) { controls::act(s, Action::Priority); }
                else if keys::is(&p, K::Enter) { let _ = s.tx.send(Command::Priority(s.order.text())); }
                else { s.order.handle(&p); }
                return false;
            }
            // The GitHub prompt: one line, Enter takes it, Back closes it (nothing is cloned then).
            if s.git.is_some() {
                if keys::is(&p, K::Back) { let _ = s.tx.send(Command::Fold); s.git = None; }
                else if keys::is(&p, K::Enter) { controls::act(s, Action::Send); }
                else { s.order.handle(&p); }
                return false;
            }
            if !s.priority && keys::is(&p, K::Back) { s.expanded = false; let _ = s.tx.send(Command::Fold); return false; }
            if keys::is(&p, K::Delegate) { controls::act(s, Action::Focus("orchestrator".into())); return false; }
            if keys::is(&p, K::Enter) { s.send(); }
            else { s.draft.handle(&p); }
        }
        Event::Mouse(m) => {
            let point = ratatui::layout::Position::new(m.column, m.row);
            match m.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if let Some((_, action)) = s.controls.iter().find(|(rect, _)| rect.contains(point)).cloned() { controls::act(s, action); }
                }
                MouseEventKind::ScrollUp if s.expanded => { s.summary_scroll = s.summary_scroll.saturating_sub(3); }
                MouseEventKind::ScrollDown if s.expanded => { s.summary_scroll = s.summary_scroll.saturating_add(3); }
                _ => {}
            }
        }
        _ => {}
    }
    false
}
