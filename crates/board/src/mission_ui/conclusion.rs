//! A persistent conclusion with findings, proposed implementation, questions, and the saved report link.
use super::State;
use crate::json::*;
use ratatui::{Frame, layout::Rect, style::{Color, Style}, widgets::{Block, Borders, Paragraph, Wrap}};

pub fn text(value: &Value) -> String {
    if value.is_null() { return "Noch kein Fazit vorhanden. Ergebnisse erscheinen nach dem Review.".into(); }
    let mut lines = vec![format!("{} · {}", value.s("title"), value.s("stage")), String::new(), value.t("conclusion")];
    if !value.arr("findings").is_empty() { lines.push("\nBefunde".into()); lines.extend(value.arr("findings").iter().map(|v| format!("• {}", crate::json::text(v)))); }
    if value.b("proposal") { lines.push(format!("\nUmsetzungsvorschlag\n{}", value.s("proposal"))); }
    if !value.arr("questions").is_empty() { lines.push("\nOffene Fragen".into()); lines.extend(value.arr("questions").iter().map(|v| format!("• {}", crate::json::text(v)))); }
    if value.b("report") { lines.push(format!("\nBericht: {}", value.s("report"))); }
    if value.b("pending") { lines.push("\nNoch nichts umgesetzt. Mit „Umsetzung freigeben“ erteilst du das Go für diesen Auftrag.".into()); }
    lines.join("\n")
}
pub fn draw(f: &mut Frame, s: &State, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Fazit und Review").border_style(Style::default().fg(Color::Cyan));
    let selection = s.coordinator.at("selection");
    let content = format!("Aktive CLI: {} · Priorität 1: {}\n{}\n\n{}", selection.s("actual"), selection.s("configured"), selection.s("reason"), text(&s.conclusion));
    f.render_widget(Paragraph::new(content).block(block).wrap(Wrap { trim: false }).scroll((s.summary_scroll, 0)), area);
}
