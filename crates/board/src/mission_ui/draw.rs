//! A compact bottom task composer; full-screen input shows saved conclusions without duplicating task lists.
use super::{State, controls};
use crate::board::keys::key_name;
use crate::board::textbox::TextBox;
use crate::json::*;
use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::Line, widgets::{Block, Borders, Paragraph}};

fn block(title: impl Into<String>) -> Block<'static> { Block::default().borders(Borders::ALL).title(title.into()).border_style(Style::default().fg(Color::Cyan)) }
pub fn draw(f: &mut Frame, s: &mut State) {
    let area = f.area();
    s.controls.clear(); s.folders_rect = Rect::default(); s.send_rect = Rect::default(); s.priority_rect = Rect::default(); s.refresh_rect = Rect::default();
    let full = s.expanded || s.prompting() || area.height > 12;
    let narrow = area.width < 80;
    let height = if full { area.height } else { area.height.min(if narrow { 9 } else { 7 }) };
    let strip = Rect::new(area.x, area.bottom().saturating_sub(height), area.width, height);
    // The row of controls grows by a line rather than dropping the buttons that no longer fit.
    let choices = if s.priority { vec![("Anwenden".into(), controls::Action::Send), ("Zurück".into(), controls::Action::Priority)] }
        else if s.git.is_some() { vec![("Anlegen".into(), controls::Action::Send), ("Zurück".into(), controls::Action::Github)] }
        else { controls::actions(s) };
    let acts = controls::lines_needed(&choices, strip.width);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(1), Constraint::Length(if narrow { 2 } else { 1 }), Constraint::Min(1), Constraint::Length(acts), Constraint::Length(1),
    ]).split(strip);
    let actual = if s.coordinator.b("agent") { s.coordinator.s("agent") } else { "startet" };
    let status = match s.coordinator.s("state") { "none" | "starting" => "startet", "working" => "arbeitet", "blocked" => "wartet auf dich", "idle" => "bereit", "exited" => "beendet", other => other };
    let header = format!("Space: {} · {actual} · {status}", s.folder().map(|p| p.name.as_str()).unwrap_or("Projekt"));
    f.render_widget(Paragraph::new(header), chunks[0]);
    controls::row(f, s, chunks[1], controls::navigation(s));
    let input = if full && !s.prompting() {
        let rows = (s.draft.visual(area.width.saturating_sub(3).max(1) as usize).len() as u16 + 2).clamp(3, 10);
        let parts = Layout::vertical([Constraint::Min(1), Constraint::Length(rows)]).split(chunks[2]);
        super::conclusion::draw(f, s, parts[0]); parts[1]
    } else { chunks[2] };
    let title = match (s.priority, s.git) {
        (true, _) => "CLI-Reihenfolge · erste CLI orchestriert".to_string(),
        (_, Some(super::Git::Repo)) => "Projekt aus GitHub · owner/name".to_string(),
        (_, Some(super::Git::New)) => "Neues Projekt · Name (Repository im eigenen Konto)".to_string(),
        _ => format!("Task für {}", s.folder().map(|p| p.name.as_str()).unwrap_or("Projekt")),
    };
    let b = block(title); s.input_rect = b.inner(input); f.render_widget(b, input);
    let rect = s.input_rect;
    if s.prompting() { editor(f, &mut s.order, rect, true); } else { editor(f, &mut s.draft, rect, true); }
    controls::row(f, s, chunks[3], choices);
    let note = if !s.error.is_empty() { s.error.clone() }
        else if !s.notice.is_empty() && s.notice != "Task eingeben und senden" { s.notice.clone() }
        else if !s.coordinator.at("selection").s("reason").is_empty() { s.coordinator.at("selection").t("reason") }
        else { format!("{} senden · {} neue Zeile · {} CLI", key_name("enter"), key_name("newline"), key_name("settings")) };
    f.render_widget(Paragraph::new(note).style(Style::default().fg(if s.error.is_empty() { Color::Gray } else { Color::Red })), chunks[4]);
}
fn editor(f: &mut Frame, box_: &mut TextBox, rect: Rect, focus: bool) {
    let width = rect.width.saturating_sub(1).max(1) as usize;
    box_.layout_width = width;
    let visual = box_.visual(width);
    let at = box_.cursor_visual(&visual);
    let height = rect.height.max(1) as usize;
    if at < box_.viewport_top { box_.viewport_top = at; }
    if at >= box_.viewport_top + height { box_.viewport_top = at + 1 - height; }
    let text = box_.text();
    let source: Vec<&str> = text.split('\n').collect();
    let lines: Vec<Line> = visual.iter().skip(box_.viewport_top).take(height).map(|(n, chunk)| Line::from(source.get(*n).unwrap_or(&"")[chunk.start..chunk.end].to_string())).collect();
    f.render_widget(Paragraph::new(lines), rect);
    if focus && rect.height > 0 && rect.width > 0 {
        let (n, chunk) = &visual[at];
        let line = source.get(*n).unwrap_or(&"");
        let col = box_.cursor().1.clamp(chunk.start, chunk.end);
        let x = crate::text::width(&line[chunk.start..col]) as u16;
        f.set_cursor_position((rect.x + x.min(rect.width - 1), rect.y + at.saturating_sub(box_.viewport_top) as u16));
    }
}
