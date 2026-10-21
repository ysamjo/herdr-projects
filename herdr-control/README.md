# Herdr Control — Mission Control

Dauerhaft geöffnete Projektansicht für Herdr: für jeden direkten Unterordner von `~/Developer` gibt es einen eigenen Herdr-Space. Jeder Space hat eine schmale Task-Eingabe unten und seinen eigenen sichtbaren Orchestrator darüber. Die Ordnerliste entfällt; die Herdr-Space-Leiste übernimmt die Projektauswahl. Worker-CLIs werden als echte interaktive Split-Panes im selben Space geöffnet. Ein Projektwechsel überschreibt keine laufende Sitzung.

## Bedienung

1. Den Projekt-Space in Herdr auswählen.
2. Im eigenen Taskfeld den Auftrag eingeben und **Task senden** wählen.
3. Der Orchestrator teilt die Arbeit auf, öffnet Worker-Panes im selben Space und prüft ihre Ergebnisse in seiner laufenden Sitzung. Fortschritt, Fehler und Rückfragen bleiben sichtbar. Offene Agenten-Tasks stehen in Herdrs Agentenliste ganz links mit Task-ID, Titel und Status; ein Klick fokussiert das Terminal.

**CLI-Reihenfolge** legt eine globale Reihenfolge für Orchestrator und Worker fest. Die erste CLI orchestriert und reviewed. Umsetzungen übernehmen Worker mit der nächsten verfügbaren anderen CLI; die laufende Orchestrator-CLI ist auch bei Worker-Fallbacks ausgeschlossen. Mindestens zwei unterschiedliche CLIs sind erforderlich. Unterstützt: Freebuff (`freebuff`), CodeBuddy (`codebuddy`), Claude Code (`claude`), Codex (`codex`), Antigravity (`agy`), Cursor (`cursor`), OpenCode (`opencode`), Kiro (`kiro`), Cline (`cline`), Qoder (`qodercli`) und Grok (`grok`). Modelle sind optional als `cli/modell` möglich. Antigravity kann auch als `antigravity-cli` oder `antigravity` eingegeben werden.

Das Plugin legt beim Öffnen für jeden direkten Projektordner unter `~/Developer` einen Space mit Taskfeld an. `herdr-control.open` und `herdr-projects.open` öffnen beziehungsweise fokussieren diese Spaces; Herdrs Space-Leiste wechselt zwischen Projekten. Der Orchestrator startet automatisch beim Auswählen seines Projekt-Spaces, auch ohne ersten Task. Worker starten erst bei delegierten Aufgaben. Alle Sitzungen bleiben beim Wechsel erhalten. Ein Prioritätswechsel wartet einen laufenden Orchestrator-Turn ab.

## Handy und Terminal-Navigation

Bei schmalem Herdr-Fenster wechselt Control automatisch in den Fokusmodus. **Fokus** aktiviert ihn auch manuell, **Kacheln** stellt die Split-Ansicht wieder her. **Orchestrator**, die nummerierten Worker-Schaltflächen und **Weiter** wechseln zwischen den vorhandenen echten Terminals. Die Eingabe kann über **Groß** aufgeklappt werden. Im mobilen Pane-Wähler heißt sie **Task / Fazit · Projektname**. Im Herdr-Menü stehen dieselben Aktionen unter „Herdr Control“ bereit, auch wenn die Eingabe gerade ausgeblendet ist: **Task eingeben / Fazit lesen**, **Orchestrator zeigen**, **Nächstes Agent-Terminal**, **Mobiler Fokusmodus** und **Alle Terminal-Panes kacheln**. Ein kleiner Bildschirm öffnet keine zusätzlichen Agenten oder Tabs.

## Fazit und Freigabe

Nach einer Recherche schreibt der Orchestrator sein Fazit dauerhaft außerhalb des Repositories. **Fazit** zeigt Befunde, Umsetzungsvorschlag, offene Fragen und Berichtlink; nach Code-Reviews zeigt dieselbe Ansicht das gespeicherte Review-Ergebnis. Erst ein gültiges Fazit schaltet **Umsetzung freigeben** frei. Die Schaltfläche gibt ausschließlich den angezeigten Rechercheauftrag frei. Das eingegebene „Go für Umsetzung“ benutzt dieselbe Prüfung. Eine Freigabe wird nur einmal übernommen. Ältere offene Rechercheberichte erhalten beim Auswählen ihres Spaces einmalig einen Fazit-Auftrag; **Fazit anfordern** wiederholt ihn bei Bedarf. Enter im normalen Taskfeld sendet weiterhin deinen eingegebenen Auftrag.

Herdrs linke Agentenliste zeigt die Zuordnung „Auftrag ▸ Teilaufgabe“ und deutsche Zustände: arbeitet, wartet auf dich, im Review oder fertig. Ein Klick fokussiert das Terminal. Die Eingabe-Leiste nennt die aktive CLI; die Fazit-Ansicht zeigt zusätzlich die konfigurierte erste Priorität und den Grund eines Providerwechsels. Auch Worker-Wechsel erscheinen mit Ursache in der Agentenliste.

## Installation

Beide Plugins aus demselben Feature-Branch installieren:

```sh
herdr plugin install ysamjo/herdr-projects --ref feature/interactive-cli-pool
herdr plugin install ysamjo/herdr-projects/herdr-control --ref feature/interactive-cli-pool
```

Dieser Feature-Branch baut seinen eigenen Quellstand mit Rust. Ein Upstream-Release darf die Änderungen nicht ersetzen. Weitere Einzelheiten und lokale Entwicklungsbefehle: [Mission Control](../docs/mission-control.md).

ACP und Headless-Modi werden für die CLIs nicht verwendet. Projekt-AGENTS.md und Git-Remote bestimmen die Arbeit und den späteren GitHub-Push. Ordner ohne Remote bleiben lokale Spaces ohne eingerichtetes GitHub-Ziel.
