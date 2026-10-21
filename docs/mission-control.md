# Mission Control

Für jeden direkten Unterordner von `~/Developer` gibt es einen eigenen Herdr-Space mit fest gebundenem Taskfeld. Die Ordnerliste in Mission Control entfällt; Herdrs Space-Leiste übernimmt die Projektauswahl. Ein Space-Wechsel lässt die anderen Projekt-Sessions bestehen. Die offenen Agenten-Tasks stehen in Herdrs nativer Agentenliste ganz links mit Task-ID, Titel und Status. Mission Control enthält keine zweite Taskliste und keine Kopie der Terminal-Ausgabe. Der Orchestrator startet beim Auswählen des Projekt-Spaces automatisch, auch ohne erste Nachricht; bereits gestartete Sitzungen bleiben erhalten. Noch nicht ausgewählte Spaces starten keine CLIs im Hintergrund. Coordinator und Worker laufen als echte interaktive, gekachelte Terminal-Panes im Projekt-Space. Neue Taskfelder warten auf die startende Shell, statt ihren Start zu überspringen. Die gemeinsame Space-Zuordnung wird unter einer Schreibsperre gespeichert, damit parallele Taskfelder keine Projekte verlieren.

## Einen Task abgeben

1. `herdr-control.open` oder `herdr-projects.open` ausführen. Für jeden Projektordner wird ein Space mit Taskfeld angelegt beziehungsweise wiederverwendet.
2. Den gewünschten Projekt-Space über Herdrs Space-Leiste öffnen und dort den Auftrag schreiben. Entwürfe sind je Projekt getrennt.
3. **Task senden** anklicken oder Enter drücken. Der feste Ordnerpfad und seine Tasks stehen ausdrücklich in jeder übergebenen Nachricht. Statusfragen beziehen sich ausschließlich auf dieses Projekt. Der Orchestrator verteilt die Arbeit mit Herdr Projects.

Die globale CLI-Reihenfolge wird über **CLI-Reihenfolge** im jeweiligen Projekt-Space bearbeitet; sie gilt für alle Projekte, nicht je Projekt. Die erste CLI bestimmt den Orchestrator. Worker nehmen die nächste verfügbare andere CLI aus derselben Liste. Auch bei Provider-Fallbacks bleibt die laufende Orchestrator-CLI für Worker ausgeschlossen; unterschiedliche Modelle derselben CLI zählen nicht als andere CLI. Die Liste braucht mindestens zwei unterschiedliche CLIs. Antigravity heißt `agy`; `antigravity` und `antigravity-cli` werden ebenfalls angenommen. Modelle sind optional als `cli/modell` möglich. Ein Prioritätswechsel wird nach dem Ende des laufenden Orchestrator-Turns umgesetzt. Die Reihenfolge wird einmal in die Einstellungen geschrieben (`priority.txt`, daneben `priority.applied`): steht die Liste bereits, bleibt ein später von Hand gesetzter Wert (`herdr-projects set …`) unangetastet, bis sich die Liste selbst ändert.

## Delegation und Review

Der Orchestrator bleibt in der Space seines Projekts sichtbar und übernimmt ausschließlich Koordination und Reviews. Jede Umsetzung delegiert er. Er legt Teilaufgaben mit dem gewählten Projektkontext an; Herdr Projects erstellt isolierte Git-Worktrees und startet die CLIs als Split-Panes neben dem Orchestrator. Die schmale Mission-Control-Eingabe bleibt unter den Terminals erhalten. Es entstehen keine zusätzlichen Agenten-Tabs. Fehler und Rückfragen erscheinen in den echten Terminal-Panes und im Task-Status. Scheitert der Start eines Workers, erfährt es der laufende Orchestrator in derselben Sitzung – einmal je Ursache, mit Task-ID, Grund und nächstem Schritt –, damit er nicht auf ein Ergebnis wartet, das nie kommt; ein geschlossener Orchestrator wird dafür nicht geweckt. Ein Klick auf einen Task in Herdrs nativer Agentenliste fokussiert dessen Terminal. Ein gestoppter Task mit vorhandenem Terminal bleibt dort als wartender Eintrag sichtbar; der Workflow zählt diesen Anzeigeeintrag niemals als laufende CLI. Noch nicht gestartete Tasks ohne Terminal stehen weiterhin im klassischen Projects-Board.

Mit `review.by coordinator` prüft die bestehende Orchestrator-Sitzung die Ergebnisse. Der Workflow schickt ihr einen Review-Auftrag mit Arbeitsordner und eindeutigem Urteilspfad. Ihr Urteil benutzt denselben Befund- und Korrekturmechanismus wie der bisherige Reviewer: Fehler gehen automatisch zum Worker zurück. `review.by agent` behält den separaten Reviewer für die klassische Board-Nutzung. Mission Control setzt `review.when always` und `review.by coordinator`.

Provider- und Kontingentfehler werden bei ruhender CLI anhand eindeutiger Fehlerausgaben erkannt. Beim Orchestrator übernimmt der nächste installierte Eintrag in einem neuen Split-Pane und bekommt die gespeicherte Vorgeschichte. Sobald der neue Orchestrator bereit ist, wird die Ausgabe des alten im Datenordner unter `coordinator/<session>/handover-*.terminal.txt` gesichert. Danach schließt ausschließlich dessen ruhender Pane. Ein arbeitender Vorgänger wird nicht beendet. Damit bleibt ein aktueller Orchestrator sichtbar, statt mehrere alte Sitzungen zu sammeln. Die ursprüngliche erste Priorität wird dabei nicht umgeschrieben. Workern steht der bestehende taskbezogene Provider-Fallback zur Verfügung. Ein Login- oder Berechtigungsdialog bleibt sichtbar und wartet auf den Menschen. Ein Neustart ersetzt nur den Orchestrator-Pane, nicht den gemeinsamen Tab. Ein Pane, den ein früherer Neustart hinterlassen hat und den kein Übergabeprotokoll nennt, wird an seinem Agentennamen im eigenen Tab und Ordner erkannt, gesichert (`orphan-*.terminal.txt`) und geschlossen, sobald er ruht; das Taskfeld und die Worker bleiben unberührt.

Die Projekt-AGENTS.md und die übergeordnete Developer/AGENTS.md gelten für die Arbeit. Der Git-Remote wird in der Ansicht angezeigt. Der bestehende Projects-Workflow übernimmt Commit, Prüfung, Push und Pull Request; ein Merge benötigt weiterhin eine Freigabe. Ein Ordner ohne Git-Remote wird ausdrücklich als lokaler Ordner angezeigt; dafür existiert noch kein GitHub-Synchronisationsziel.

## Installation aus diesem Feature-Branch

Dieser Branch benötigt seine eigenen Änderungen. `bin/run.sh` ruft deshalb `bin/source.sh` auf und ersetzt sie niemals durch das unveränderte Upstream-Release mit derselben Versionsnummer. Ein bereits geprüftes lokales Build wird benutzt; GitHub-Installationen teilen ansonsten ihren Cargo-Build unter `~/.cache/herdr-control-build`.

Für lokale Entwicklung:

```sh
npm run check mission_control
herdr plugin link /Users/family/Developer/herdr-projects
herdr plugin link /Users/family/Developer/herdr-projects/herdr-control
herdr server reload-config
```

Der Control-Plugin-Start und die Projects-Hooks müssen aus demselben Quellstand kommen. Die Quellarchive vor größeren Änderungen liegen lokal unter `Archiv/`.

## Ruhige Terminal-Anzeige

Automatische Status-, Fehler- und Ausgabeabfragen lesen ausschließlich das sichtbare Terminalbild. Sie sammeln keinen internen CLI-Gesprächsverlauf und schicken keine Scroll-Tasten. Das gilt auch bei einer als bereit oder fertig gemeldeten CLI, damit Cline und andere Terminals ihre Scrollposition behalten. Gesicherte Terminal-Protokolle enthalten entsprechend den sichtbaren Ausschnitt.

## Kompakte Eingabe und Fokusmodus

Die Task-Eingabe belegt unten etwa sieben Terminalzeilen. Der Orchestrator erhält den oberen Hauptbereich; Worker teilen diesen Bereich. Neue Panes werden nach ihrer tatsächlich verfügbaren Breite beziehungsweise Höhe geteilt. Beim Aktualisieren eines alten Seitenlayouts wird ausschließlich die ruhende Eingabe umgehängt, während die Agent-Terminals weiterlaufen.

Unter 90 Spalten beziehungsweise 22 Zeilen wechselt Control automatisch in den mobilen Fokusmodus. Die Auswahl **Fokus** oder **Kacheln** hat danach Vorrang vor der Automatik. Herdr-Zoom zeigt ein echtes Terminal in voller Größe; es wird keine Ausgabe kopiert. **Orchestrator**, nummerierte Worker und **Weiter** wechseln die Sitzung. **Groß** öffnet die Eingabe vollständig; Escape bringt die Terminalansicht zurück. Im Herdr-Menü sind Eingabe, Orchestrator, nächster Agent, Fokusmodus und Kacheln ebenfalls erreichbar. Die Befehle heißen `control-input`, `control-orchestrator`, `control-next`, `control-mobile` und `control-tiles`.

## Dauerhaftes Fazit und Freigabe

Ein abgeschlossener Rechercheauftrag geht mit Bericht und Review zurück an seinen Orchestrator. Dieser speichert `conclusion.json` im externen Berichtordner: `task`, `conclusion`, `findings`, `proposal` und `questions`. Die UI akzeptiert nur das passende Task-Kennzeichen, vorhandenes Fazit und einen Umsetzungsvorschlag sowie Listen aus Zeichenketten. Ungültige oder fehlende Dateien lassen die Freigabe gesperrt. Ältere offene Berichte ohne Fazit-Auftrag werden beim Auswählen des Projekts einmalig an dessen bestehenden Orchestrator zurückgegeben; **Fazit anfordern** erlaubt eine bewusste Wiederholung.

**Fazit** zeigt diese gespeicherten Ergebnisse und den Berichtlink auch nach Space-Wechsel oder Neustart. Nach einer Umsetzung zeigt die Ansicht stattdessen das gespeicherte Review. **Umsetzung freigeben** übergibt genau den angezeigten Rechercheauftrag; ein inzwischen neuerer Bericht wird dabei nicht automatisch freigegeben. Ein projektfremder, archivierter oder bereits freigegebener Auftrag wird abgelehnt. Die Freigabe läuft unter einer Task-Sperre, damit zwei Eingaben nicht doppelt delegieren. Ein ausgeschriebenes „Go für Umsetzung“ folgt derselben Prüfung. Während der Recherche-Rückmeldung darf der Coordinator keine Umsetzungstasks anlegen.

Worker erben über `missionRequest` den ursprünglichen Auftrag. Herdrs native Agentenliste gruppiert nach Projekt-Space und benennt Teilaufgaben als „Auftrag ▸ Teilaufgabe“ mit deutschen Zuständen. Fertige Terminals bleiben bis zum Archivieren sichtbar. Die aktive CLI, die erste Priorität und der Grund eines Providerwechsels sind in der Eingabe beziehungsweise aufgeklappten Fazit-Ansicht nachlesbar; Worker-Fallbacks zeigen ihren Wechselgrund ebenfalls in der Sidebar.
