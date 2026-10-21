# Herdr Control — Entwicklerhinweise

- Plugin-Manifest: `herdr-plugin.toml`; jeder direkte Ordner unter `~/Developer` erhält einen eigenen Herdr-Space mit fest gebundenem Taskfeld. Space-Wechsel ersetzt keine Sitzung.
- Beim Auswählen eines Projekt-Spaces startet dessen sichtbarer interaktiver Orchestrator automatisch; Worker folgen bei delegierten Aufgaben als echte Split-Panes. Die Task-Eingabe bleibt bestehen.
- `scripts/control.sh` speichert die globale Reihenfolge unter `HERDR_PLUGIN_CONFIG_DIR` und schreibt sie über `herdr-projects set`.
- Agentenliste muss mit den unterstützten Einträgen in Herdr Projects synchron bleiben: Freebuff, CodeBuddy, Claude Code, Codex, Antigravity (`agy`), Cursor, OpenCode, Kiro, Cline, Qoder CLI (`qodercli`) und Grok.
- Keine Ordnerauswahl-Liste in Mission Control hinzufügen; Herdr-Spaces sind die Projektauswahl. Keine ACP- oder Headless-Steuerung hinzufügen. Keine Agenten in zusätzlichen Tabs starten.
- Nach Funktionsänderungen README und diese Datei aktuell halten.

- Eingabe unten: `mission_view.rs` teilt den ersten Coordinator-Pane nach unten und tauscht die Positionen. Die Eingabe behält etwa sieben Zeilen. Vorhandene Seitenlayouts werden bei ruhendem Eingabeprozess über einen vorübergehenden Space migriert; laufende Agenten bleiben erhalten.
- Fokusmodus verwendet Herdrs Pane-Zoom; `control-input`, `control-orchestrator`, `control-next`, `control-mobile` und `control-tiles` öffnen vorhandene Panes. Keine zusätzlichen Tabs, Headless-Ansichten oder Terminal-Kopien.
- `mission_conclusion.rs` liest taskgebundene `conclusion.json` aus dem externen Berichtordner. Umsetzung erst nach gültigem Fazit und ausdrücklicher Freigabe derselben Task-ID; doppelte Freigaben und projektfremde Berichte ablehnen.
- `mission_requests.rs` bindet Worker an den ursprünglichen Auftrag. `mission_status.rs` liefert deutsche Sidebar-Zustände und aktive/configurierte CLI mit Wechselgrund. UI: `mission_ui/controls.rs` und `conclusion.rs`.
