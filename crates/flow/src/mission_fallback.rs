//! Coordinator provider handover on the global CLI order, keeping the old context in a readable file. Which error is
//! waited out and which hands over at once is the one rule every role follows (store/rule.rs).
use crate::coordinator::{co::Co, requests, state, who};
use crate::err::Res;
use crate::herdr::Herdr;
use crate::json::*;
use crate::store::files::write_json;
use crate::store::rule::{next, Cut, Step};

pub fn target(socket: &str) -> Option<Obj> { target_for(&Co::top(socket)) }
pub fn target_for(co: &Co) -> Option<Obj> {
    let c = state::get(co).unwrap_or_default();
    if c.s("controlPriority") == crate::mission_control::priority() && c.b("controlFallback") {
        return who::pick_of(c.s("controlFallback"), "").ok().flatten();
    }
    who::choice()
}
pub fn tick(socket: &str) -> Res<()> {
    for co in state::all(socket) { tick_one(&co)?; }
    if let Some(co) = crate::mission_control::selected_coordinator(socket) { tick_one(&co)?; }
    Ok(())
}
pub fn tick_one(co: &Co) -> Res<()> {
    let c = state::get(co).unwrap_or_default();
    if c.b("stopped") || state::pumping(&c) || c.b("want") || !c.b("run") { return Ok(()) }
    let api = Herdr::new(Some(co.socket.clone()));
    if api.pane_get(c.s("pane")).is_none() { return Ok(()) }
    let agent = api.agent_get(c.s("pane")).ok().flatten();
    let explicit = regex::Regex::new(r"(?im)^\s*(?:[✗×!⚠⏺●]\s*)?(?:api error:|error:|usage limit reached|quota exceeded|individual quota reached|rate limit exceeded|you.ve hit your (?:usage )?limit|http (?:429|529|401|403))").unwrap();
    let turn = crate::finish::turn_ended::screen_error(&api, c.s("pane"), agent.as_ref()).filter(|t| explicit.is_match(&t.error));
    let mut error = turn.as_ref().map(|t| t.error.clone()).filter(|s| !s.is_empty()).or_else(|| c.so("error").map(String::from)).unwrap_or_default();
    let screen_raw = crate::herdr::api::read_screen(&api, c.s("pane"));
    let last = c.arr("where").last();
    // Silent-Crash-Erkennung: Exit-Code != 0, Panic, Segfault oder Fatal Error auf dem Screen
    if error.is_empty() {
        let crash_pattern = regex::Regex::new(r"(?im)(?:command terminated with exit code [1-9]\d*|process exited with code [1-9]\d*|exited with status [1-9]\d*|fatal error:|segmentation fault|killed: 9|abort trap: 6|uncaught exception:|panic:)").unwrap();
        if let Some(m) = crash_pattern.find(&screen_raw) {
            error = format!("Prozess-Abbruch erkannt: {}", m.as_str());
        }
    }
    // Stille-Erkennung (Idle Timeout): Unerledigte Nachricht wartet länger als Timeout
    if error.is_empty() {
        let sent_at = if c.b("sentAt") {
            Some(c.i("sentAt"))
        } else {
            last.and_then(|m| if m.b("sentAt") && !m.b("handled") { Some(m.i("sentAt")) } else if m.b("at") { Some(m.i("at")) } else { None })
        };
        if let Some(sent_at) = sent_at {
            let now = crate::clock::now();
            let idle_timeout = std::env::var("HERDR_CONTROL_IDLE_TIMEOUT_MS").ok().and_then(|s| s.parse().ok()).unwrap_or(180_000);
            if now > sent_at && (now - sent_at) >= idle_timeout {
                error = format!("Stille-Erkennung (Idle Timeout): Keine Antwort seit {} Sekunden", (now - sent_at) / 1000);
            }
        }
    }
    if error.is_empty() {
        // It answers again: no wait of an old error is left standing (store/quota.rs keeps the subscription's own record).
        if c.b("controlRetry") { state::change(co, |s| { s.put("controlRetry", Value::Null); true })?; }
        return Ok(())
    }
    let said = crate::agents::provider_error::provider_error(&error, "", 0);
    let (auth, passing) = if error.starts_with("Stille-Erkennung") || error.starts_with("Prozess-Abbruch") {
        (false, false)
    } else {
        turn.as_ref().map(|t| (t.auth, t.passing)).unwrap_or((said.auth, said.passing))
    };
    let reason = if said.reason.is_empty() { "provider failed".to_string() } else { said.reason.clone() };
    let run = c.at("run");
    let on = if run.b("model") { format!("{}/{}", run.s("kind"), run.s("model")) } else { run.t("kind") };
    // What this subscription is known to be, for every project and every model of it (store/quota.rs).
    let q = if auth { crate::store::quota::LOGIN }
        else if crate::store::quota::out_of_allowance(&error) { crate::store::quota::EXHAUSTED } else { crate::store::quota::DEGRADED };
    crate::store::quota::note(&crate::store::quota::group_of_agent(&on), q, &error, crate::store::quota::reset_ms(&error, crate::clock::now()), co.repo().unwrap_or(""));
    let folder = last.filter(|m| std::path::Path::new(m.s("root")).is_dir()).map(|m| std::path::PathBuf::from(m.s("root")))
        .or_else(|| co.root()).unwrap_or_else(crate::store::files::user_home);
    // The one rule (store/rule.rs): what passes by itself is waited out and asked again of the same coordinator; a login,
    // an empty allowance or a refused model goes to the next program at once, since waiting changes nothing.
    let retry = c.at("controlRetry");
    let same = retry.b("until") && retry.s("error") == error;
    let tries = if same { retry.i("tries") } else { 0 };
    let why_not = Cut { error: &error, reason: &reason, auth, passing, tries };
    if let Step::Wait { ms, .. } = next(&why_not) {
        let now = crate::clock::now();
        // The wait it was given is not over: the coordinator is left alone until it is.
        if same && now < retry.i("until") { return Ok(()) }
        if !same { state::change(co, |s| { s.put("controlRetry", json!({ "error": error, "until": now + ms, "tries": 0 })); true })?; return Ok(()) }
        // It was waited out: the same coordinator is asked to carry on, in its own session, and gets the next wait.
        requests::send(&folder, &format!("Der Anbieter meldet: {error}\n\nDas geht oft von selbst vorbei. Setze deine Arbeit dort fort, wo du warst, und lege nichts doppelt an. Wenn es weiter scheitert, übernimmt ein anderes Programm."), false, "", co)?;
        let done = tries + 1;
        match next(&Cut { tries: done, ..why_not }) {
            Step::Wait { ms, .. } => { state::change(co, |s| { s.put("controlRetry", json!({ "error": error, "until": crate::clock::now() + ms, "tries": done })); true })?; return Ok(()) }
            Step::Hand { .. } => {}
        }
    }
    let order = crate::mission_control::parse_priority(&crate::mission_control::priority())?;
    let at = order.iter().position(|s| s == &on).unwrap_or(0);
    // Nothing of a subscription that is out of allowance or needs a login: another model of it is the same budget.
    let next = order.iter().skip(at + 1).find(|s| crate::store::quota::rank(s) < 2 && crate::agents::installed::installed(&crate::store::settings::values::agent_of(s).kind));
    let Some(next) = next else { return Ok(()) };
    let file = state::dir_of(co).join(format!("handover-{}.json", crate::clock::now()));
    write_json(&file, &Value::Object(c.clone()))?;
    crate::store::files::write_atomic(&file.with_extension("terminal.txt"), screen_raw.as_bytes())?;
    state::change(co, |s| { s.put("controlPriority", crate::mission_control::priority()); s.put("controlFallback", next.clone()); s.put("controlHandover", format!("{on} → {next}: {error}")); s.put("controlRetry", Value::Null); true })?;
    requests::handover(who::pick_of(next, "")?, co)?;
    let scope = last.and_then(|m| m.s("said").lines().next()).filter(|s| s.starts_with("[MISSION-CONTROL-")).map(|s| format!("{s}\n")).unwrap_or_default();
    let pending = last.map(|m| format!("\nZuletzt übergebene Nachricht für {} (bei dieser Fehlermeldung unbeantwortet):\n{}", m.s("root"), m.s("said"))).unwrap_or_default();

    // 1. Git-Status & uncommitted changes
    let git_status = crate::store::repos::git_out(&folder, &["status", "-s"]).ok().filter(|s| !s.is_empty());
    let git_diff_stat = crate::store::repos::git_out(&folder, &["diff", "--stat"]).ok().filter(|s| !s.is_empty());
    let git_section = if let Some(st) = git_status {
        let diff_part = git_diff_stat.map(|d| format!("\nDiff-Übersicht:\n{d}")).unwrap_or_default();
        format!("\n\n[Aktueller Git-Status (uncommitted changes)]:\n{st}{diff_part}")
    } else {
        String::new()
    };

    // 2. Letzte 15 Zeilen Konsolenausgabe
    let terminal_tail = crate::finish::shared::last_lines(&screen_raw, 15);
    let terminal_section = if !terminal_tail.trim().is_empty() {
        format!("\n\n[Letzte Konsolenausgabe des vorigen Orchestrators (15 Zeilen)]:\n{terminal_tail}")
    } else {
        String::new()
    };

    // 3. Strukturierter Checkpoint (Offene Aufgaben)
    let open_tasks: Vec<String> = crate::store::tasks::list(Default::default())
        .into_iter()
        .filter(|t| !t.b("archivedAt") && ["starting", "setup", "working", "checking", "pushing", "reviewing", "approval"].contains(&t.s("stage")))
        .filter(|t| {
            if let Some(r) = co.repo() {
                t.s("id").split('/').next().unwrap_or("") == r
            } else { true }
        })
        .map(|t| format!("- #{}: {} [{}]", t.s("id"), crate::store::tasks::one_line(t.s("title"), 60), t.s("stage")))
        .collect();
    let checkpoint_section = if !open_tasks.is_empty() {
        format!("\n\n[Offene Teilaufgaben / Checkpoint]:\n{}", open_tasks.join("\n"))
    } else {
        String::new()
    };

    requests::send(&folder, &format!("{scope}Orchestrator-Providerwechsel von {on} zu {next}. Ursache: {error}. Die vollständige Vorgeschichte liegt in {}. Lies sie und `board list`, bevor du weiterarbeitest. Setze offene Aufträge und offene ORCHESTRATOR-REVIEW-Anfragen fort; lege bereits existierende Teilaufgaben nicht erneut an. Berichte dem Menschen den Wechsel.{git_section}{terminal_section}{checkpoint_section}{pending}", file.display()), false, "", co)?;
    Ok(())
}
