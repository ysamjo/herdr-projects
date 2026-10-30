//! [real programs] The command line, as a person or an agent runs it: add, list, report, set, unset, show, --json,
//! and a flag a command does not know refused with the nearest right one named, and nothing done.
use crate::common;
use common::*;
use sw::json::*;

/// The program run in the world's repo, with environment of its own (a pane's, say): its exit code, what it printed
/// and what it said went wrong.
fn run_with(w: &World, args: &[&str], env: &[(&str, &str)]) -> (i32, String, String) {
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_herdr-projects")).args(args).current_dir(&w.root)
        .env("HERDR_SOCKET_PATH", "/nonexistent").env("HERDR_BIN_PATH", "/bin/false").envs(env.iter().copied()).output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}
fn run(w: &World, args: &[&str]) -> (i32, String, String) { run_with(w, args, &[]) }
fn out(w: &World, args: &[&str]) -> String { let (code, o, e) = run(w, args); assert_eq!(code, 0, "{args:?}: {e}"); o }
fn refused(w: &World, args: &[&str], re: &str) { let (code, o, e) = run(w, args); assert_ne!(code, 0, "{args:?} was not refused: {o}"); assert!(regex::Regex::new(re).unwrap().is_match(&e), "{args:?}: {e}"); }
fn tasks() -> Vec<Obj> { sw::store::tasks::list(Default::default()) }
/// A task waiting for its merge, with a link to the pane of the agent that worked on it.
fn ready_to_approve(w: &World, title: &str, pane: &str) -> String {
    let id = w.add(title);
    sw::store::tasks::patch(&id, |t| {
        t.put("stage", "approval"); t.put("msg", "Ready to merge"); t.put("reviewedHead", serde_json::json!("f".repeat(40)));
        t.obj_mut("link").put("pane", pane); t.obj_mut("link").put("socket", w.socket.clone());
    }).unwrap();
    id
}

#[test]
fn real_programs_add_list_report_set_unset_and_show() {
    let w = world();
    assert_eq!(out(&w, &["hook"]), "");
    let added: serde_json::Value = serde_json::from_str(&out(&w, &["add", "From", "the", "shell", "--desc", "d", "--json"])).unwrap();
    assert_eq!(added.s("id"), "proj/1");
    out(&w, &["report", "1", "--state", "working", "--msg", "on it"]);
    let listed: serde_json::Value = serde_json::from_str(&out(&w, &["list", "--json"])).unwrap();
    assert_eq!((listed[0].s("state"), listed[0].s("app"), listed[0].s("msg")), ("working", sw::store::files::me().as_str(), "on it"));
    assert_eq!(out(&w, &["set", "board.agents", "2"]), "board.agents: 2  (you)\n");
    assert!(out(&w, &["--skill"]).starts_with("---\nname: herdr-projects\n"));
    assert_eq!(out(&w, &["set", "review.when", "never", "--task", "1"]), "review.when for PRJ-1: never  (this task)\n");
    assert!(regex::Regex::new(r"^Your settings \(\* = changed by you\)\n[\s\S]*\n  merge\.by +you +built in\n[\s\S]*\n\* board\.agents +2 +you\n").unwrap().is_match(&out(&w, &["show"])));
    assert!(regex::Regex::new(r"^PRJ-1: every setting, and the place it came from[\s\S]*\n\* review\.when +never +this task\n[\s\S]*\n  board\.agents +2 +you\n").unwrap().is_match(&out(&w, &["show", "1"])));
    let shown: serde_json::Value = serde_json::from_str(&out(&w, &["show", "1", "--json"])).unwrap();
    assert_eq!(shown.arr("settings").iter().find(|r| r.s("name") == "review.when").cloned().unwrap(), serde_json::json!({ "name": "review.when", "kind": "work", "value": "never", "from": "this task", "place": "task", "own": true, "note": "" }));
    out(&w, &["set", "merge.by", "itself", "--repo", "proj"]);
    assert!(regex::Regex::new(r"\n  merge\.by +itself +your entry for proj\n").unwrap().is_match(&out(&w, &["show", "1"])));
    assert_eq!(out(&w, &["unset", "review.when", "--task", "1"]), "review.when for PRJ-1: repos  (built in)\n");
    assert_eq!(out(&w, &["unset", "board.agents"]), "board.agents: 4  (built in)\n");
    refused(&w, &["set", "merge.pr", "maybe"], r"merge\.pr is a switch: true or false");
    refused(&w, &["set", "review.round", "2"], r#"Unknown setting "review\.round"\. Did you mean review\.rounds\?"#);
    refused(&w, &["set", "board.agents", "2", "--task", "1"], r"board\.agents is a board setting: it is yours only and cannot be set on a task");
    refused(&w, &["delegate", "1", "--merge.by", "sometimes", "--reviewer.agent", "claude"], r"merge\.by must be you, itself");
    assert_eq!(task("proj/1").at("settings"), &serde_json::json!({}));
    refused(&w, &["config", "max", "2"], r#"Unknown command "config""#);
    refused(&w, &["frobnicate"], "Unknown command");
}

#[test]
fn real_programs_a_flag_a_command_does_not_know_is_refused_with_the_nearest_right_one_and_nothing_is_done() {
    let w = world();
    refused(&w, &["list", "--archvied"], r"^list does not know --archvied\. Did you mean --archived\? It takes: --archived, --all, --query, --json, --help\. Nothing was done\.\n?$");
    refused(&w, &["list", "--no-archvied"], r"list does not know --no-archvied\. Did you mean --archived\?");
    refused(&w, &["add", "A task", "--reviewer", "claude"], r"add does not know --reviewer\. Did you mean --reviewer\.agent\? It takes: .*any work setting by its name");
    refused(&w, &["add", "A task", "--delgate"], r"Did you mean --delegate\?");
    assert_eq!(tasks().len(), 0, "nothing was added");
    refused(&w, &["add", "A task", "--review.wen", "never"], r#"Unknown setting "review\.wen"\. Did you mean review\.when\?"#);
    assert_eq!(tasks().len(), 0);
    refused(&w, &["set", "board.agents", "2", "--tsak", "1"], r"set does not know --tsak\. Did you mean --task\?");
    refused(&w, &["automations", "add", "n", "--prompt", "p", "--schedule", "daily 09:00", "--evry-repo"], r"Did you mean --every-repo\?");
    refused(&w, &["report", "--title", "x", "--query=y"], r"report does not know --query\.");
    refused(&w, &["--archived"], r"herdr-projects does not know --archived\. It takes: --skill, --json, --help");
    refused(&w, &["nonsense", "--x"], r#"Unknown command "nonsense""#);
    refused(&w, &["help", "--bogus"], r"help does not know --bogus\. It takes: --json, --help");
    for hidden in ["hook", "keeper", "keeper-work", "open", "open-coordinator", "approve-current", "serve", "run", "turn-ended", "plan-written", "skill", "remote"] { refused(&w, &[hidden, "--bogus"], &format!("{hidden} does not know --bogus")); }
    refused(&w, &["turn-ended", "--id", "x", "--rol", "agent"], r"turn-ended does not know --rol\. Did you mean --role\?");
    refused(&w, &["automations", "list", "--review.wen", "never"], r"automations list does not know --review\.wen: it takes no settings \(and the setting is named review\.when\)\. It takes: --json, --help");
    refused(&w, &["automations", "on", "n", "--prompt", "p"], r"automations on does not know --prompt");
    refused(&w, &["coordinator", "stop", "--agent", "codex"], r"coordinator stop does not know --agent");
    refused(&w, &["delegate", "1", "--worker.agnt", "codex"], r#"Unknown setting "worker\.agnt"\. Did you mean worker\.agent\?"#);
    refused(&w, &["list", "--review.when", "never"], r"list does not know --review\.when: it takes no settings");
    assert_eq!(sw::store::automations::automations().len(), 0);
    // settings given to add without --delegate are the task's own from the start, and kept
    out(&w, &["add", "Kept", "settings", "--review.when", "never", "--merge.by=itself", "--no-research"]);
    let t = tasks().into_iter().next().unwrap();
    assert_eq!((t.t("title"), t.at("settings").t("review.when"), t.at("settings").t("merge.by"), t.at("settings").at("research.only").clone(), t.t("stage")), ("Kept settings".into(), "never".into(), "itself".into(), serde_json::json!(false), "todo".into()));
    assert!(regex::Regex::new(r"\n\* review\.when +never +this task\n").unwrap().is_match(&out(&w, &["show", t.s("id")])));
    assert!(out(&w, &["list", "--archived"]).contains("No tasks"));
    assert!(out(&w, &["list", "--all", "--query", "kept"]).contains("Kept settings"));
}

#[test]
fn real_programs_an_agents_own_pane_cannot_approve_its_work_even_with_anyway() {
    let w = world();
    let id = ready_to_approve(&w, "Work", "worker-pane");
    for args in [vec!["approve", "1"], vec!["approve", "1", "--anyway"]] {
        let (code, o, e) = run_with(&w, &args, &[("HERDR_PANE_ID", "worker-pane"), ("HERDR_SOCKET_PATH", &w.socket)]);
        assert_ne!(code, 0, "{args:?} was not refused: {o}");
        assert!(e.contains("this command ran in the agent pane of PRJ-1"), "{args:?}: {e}");
        assert!(e.contains("Nothing was merged"), "{args:?}: {e}");
        assert_eq!(task(&id).s("stage"), "approval", "{args:?}");
        assert!(!task(&id).b("mergeIntent"), "{args:?}");
    }
}

#[test]
fn real_programs_approve_outside_an_agents_pane_works_and_records_where_the_yes_came_from() {
    let w = world();
    let id = ready_to_approve(&w, "Work", "worker-pane");
    out(&w, &["approve", "1"]);
    let t = task(&id);
    assert_eq!(t.s("stage"), "merging");
    let via = t.at("mergeIntent").t("approvedVia");
    assert!(["terminal", "session"].contains(&via.as_str()), "{via}");
    assert_eq!(t.at("mergeIntent").s("verifiedHead"), "f".repeat(40));
}
