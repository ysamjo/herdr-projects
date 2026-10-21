//! The fallback agents (worker.fallback, reviewer.fallback): the next agent on the list takes the task over in the same
//! worktree and pane when the one on it cannot go on, and is told what was done; the list gone through in order.
use crate::common;
use common::*;
use serde_json::json;
use sw::json::*;
use sw::store::handovers::{handover_line, handovers};

const FAIL: &str = r#"{ "verdict": { "meets": false, "findings": [{ "severity": "major", "subject": "The file says v1, the task asked for v2", "where": "f.txt:1", "proof": "cat f.txt" }], "text": "Not what was asked." } }"#;

#[test]
fn quota_handoff_preserves_commits_loose_files_and_finishes_the_original_task() {
    let w = cut_world("30", json!({ "worker.fallback": "codex", "check.command": "test -f half.txt && test -f new.txt && test -f finished.txt && grep -q changed a.txt", "review.when": "never" }));
    w.script(json!({ "worker": [
        { "files": { "half.txt": "committed\n" }, "commit": "first half", "status": "working", "after": 0,
          "then": { "files": { "a.txt": "changed\n", "new.txt": "uncommitted\n" }, "ends": "claude-usage-limit-made" } },
        { "files": { "finished.txt": "finished\n" }, "note": "Completed the remaining work." }
    ] }));
    let id = w.add("Finish both halves without losing earlier work");
    delegate(&id);
    let first = settle(&id);
    let tree = std::path::Path::new(first.s("worktree"));
    std::fs::write(tree.join("binary.dat"), [0, 255, 42]).unwrap();
    w.go_on(&first);
    let done = settle(&id);
    assert_eq!(done.s("stage"), "approval", "{}", done.at("failure"));
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert_eq!(handovers(&done).len(), 1);
    assert_eq!(done.s("id"), id);
    assert_eq!(done.at("link").s("pane"), first.at("link").s("pane"));
    assert_eq!(done.s("worktree"), first.s("worktree"));
    assert_eq!(done.s("branch"), first.s("branch"));
    assert_eq!(read(&tree.join("half.txt")), "committed\n");
    assert_eq!(read(&tree.join("a.txt")), "changed\n");
    assert_eq!(read(&tree.join("new.txt")), "uncommitted\n");
    assert_eq!(std::fs::read(tree.join("binary.dat")).unwrap(), [0, 255, 42]);
    assert_eq!(read(&tree.join("finished.txt")), "finished\n");
    assert!(git(tree, &["log", "--format=%s"]).contains("first half"));
    let told = prompts(&w, done.at("link").s("pane"))[0].clone();
    assert!(told.contains("Finish both halves without losing earlier work"), "{told}");
    assert!(told.contains("first half"), "{told}");
    assert!(told.contains("new.txt") && told.contains("binary.dat") && told.contains("a.txt"), "the successor needs the loose file paths: {told}");
    assert!(git(tree, &["status", "--porcelain"]).is_empty());
}

struct UnknownAgent(std::rc::Rc<common::herdr::FakeHerdr>);
impl sw::herdr::cli::Fake for UnknownAgent {
    fn run(&self, args: &[String], input: &str) -> sw::herdr::cli::Ran {
        if args.first().map(String::as_str) == Some("agent") && args.get(1).map(String::as_str) == Some("get") {
            return sw::herdr::cli::Ran { status: 1, stdout: String::new(), stderr: json!({ "error": { "code": "timeout", "message": "status unavailable" } }).to_string() };
        }
        self.0.run(args, input)
    }
    fn kids(&self, pid: i64) -> Vec<i64> { self.0.kids(pid) }
    fn wired_to(&self, pid: i64, group: &[i64]) -> Option<bool> { self.0.wired_to(pid, group) }
    fn kill(&self, pid: i64) { self.0.kill(pid) }
}

#[test]
fn handoff_waits_when_the_old_agents_status_cannot_be_confirmed() {
    let w = cut_world("30", json!({ "worker.fallback": "codex" }));
    w.script(json!({ "worker": [{ "status": "working" }] }));
    let id = w.add("Unknown process status");
    delegate(&id);
    settle(&id);
    sw::herdr::cli::set_fake(Some(std::rc::Rc::new(UnknownAgent(w.fake.clone()))));
    let mut g = sw::runner::Guard::free(&id);
    assert!(!sw::finish::hand_over::hand_over(&id, &mut g, "out of allowance", "").unwrap());
    assert_eq!(starts(&w), vec!["claude"], "never start a successor while the old process might still write");
    assert!(handovers(&task(&id)).is_empty());
    assert!(task(&id).at("fallbacks").s("worker").contains("status unavailable"));
}

#[test]
fn out_of_allowance_the_next_agent_takes_over_in_the_same_worktree_and_is_told_what_was_done() {
    let w = cut_world("30,30", json!({ "worker.fallback": ["codex", "pi"] }));
    w.script(json!({ "worker": [{ "files": { "half.txt": "half\n" }, "commit": "half the work", "ends": "claude-usage-limit-made" }, { "status": "working" }] }));
    let id = w.add("Runs out");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert_eq!((t.s("agent"), t.s("on"), t.s("app")), ("codex", "codex", "codex"));
    assert_eq!(handovers(&t).len(), 1);
    assert_eq!(handover_line(&handovers(&t)[0]), "codex took over from claude: out of allowance until 17:00");
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.starts_with("Work board task proj/1, role: agent.\n\nRuns out"));
    assert!(told.contains("taking this task over from another agent (claude), which could not go on: out of allowance until 17:00."), "{told}");
    assert!(regex::Regex::new(r"Its commits on this branch so far, newest first:\n- [0-9a-f]+ half the work").unwrap().is_match(&told), "{told}");
    assert_eq!(calls(&w).iter().filter(|c| c.starts_with("worktree create")).count(), 1, "the same worktree");
    assert_eq!(git(std::path::Path::new(t.s("worktree")), &["log", "--format=%s", &format!("{}..HEAD", t.s("baseSha"))]), "half the work");
    assert!(calls(&w).contains(&format!("tab rename {} Agent (codex)", t.at("link").s("tab"))));
    assert!(calls(&w).iter().any(|c| c.contains("--display-agent agent codex · ")));
    let d = sw::actions::finish_line::details(&id).unwrap();
    assert_eq!(d.s("agent"), "codex");
    assert_eq!(d.arr("handovers").iter().map(|h| (h.t("role"), h.t("from"), h.t("to"))).collect::<Vec<_>>(), vec![("worker".into(), "claude".into(), "codex".into())]);
    let mut b = common::board::open(&w.root, 70);
    b.press(&["alt+t"]);
    let panel = b.text();
    assert!(regex::Regex::new(r" On it +codex\n Took over +codex took over from claude: out of allowance until 17:00 · \d+s ago\n").unwrap().is_match(&panel), "{panel}");
    assert!(regex::Regex::new(r" Agent +codex 9\.9\.9 · default model · default effort\n Takes over +codex, then pi · when the agent cannot go on\n").unwrap().is_match(&panel), "{panel}");
}

#[test]
fn logged_out_the_list_is_gone_through_in_order_and_used_up_the_task_waits_for_you() {
    let w = cut_world("30", json!({ "worker.fallback": "claude, codex, pi" }));
    w.script(json!({ "worker": [{ "ends": "claude-model-not-found" }, { "ends": "codex-unauthorized" }, { "ends": "pi-unknown-provider" }] }));
    let id = w.add("Nobody can");
    delegate(&id);
    let t = settle(&id);
    assert_eq!(starts(&w), vec!["claude", "codex", "pi"], "claude itself is on the list: it is not handed to itself");
    assert_eq!(handovers(&t).iter().map(handover_line).collect::<Vec<_>>(), vec!["codex took over from claude: claude can't use the model not-a-real-model-zz", "pi took over from codex: logged out"]);
    assert_eq!(t.s("stage"), "failed");
    assert!(t.s("msg").starts_with("Agent stopped: "), "{}", t.s("msg"));
    assert_eq!(row(&id).s("state"), "error");
    assert_eq!(t.at("fallbacks").s("worker"), "worker.fallback: nobody left to take over (claude was on it already; codex was on it already; pi was on it already)");
    assert_eq!(t.arr("tried").iter().map(text).collect::<Vec<_>>(), vec!["claude", "codex", "pi"]);
    w.script(json!({ "worker": [{ "status": "working" }] }));
    sw::actions::talk::tell(&id, "try again", false).unwrap();
    let t = task(&id);
    assert_eq!((t.s("agent"), t.s("on"), t.arr("tried").len(), t.at("fallbacks").at("worker").is_null()), ("pi", "pi", 0, true));
}

#[test]
fn work_that_still_fails_review_after_its_rounds_goes_to_the_next_agent_with_the_open_findings() {
    let w = cut_world("30", json!({ "worker.fallback": "codex", "review.rounds": 1 }));
    let fail: serde_json::Value = serde_json::from_str(FAIL).unwrap();
    let mut again = fail.clone(); again["verdict"]["findings"][0]["id"] = json!("F1");
    w.script(json!({ "worker": [{ "files": { "f.txt": "v1\n" }, "commit": "v1", "note": "Did v1." }, { "note": "Tried." }, { "status": "working" }], "reviewer": [fail, again] }));
    let id = w.add("Stuck");
    delegate(&id);
    let mut t = settle(&id);
    for _ in 0..3 { if t.s("stage") != "working" || starts(&w).len() < 2 { t = settle(&id); } }
    assert_eq!(t.s("stage"), "working", "{}", t.at("failure"));
    assert_eq!(starts(&w), vec!["claude", "codex"]);
    assert_eq!(handover_line(&handovers(&t)[0]), "codex took over from claude: its work still failed review after 1 round");
    let told = prompts(&w, t.at("link").s("pane"))[0].clone();
    assert!(told.contains("The last review of this work failed (Fail · 1 major), and what it found is still open:\n\nNot what was asked.\n\nRequired: fix each of these.\n- [major] The file says v1, the task asked for v2 (f.txt:1)"), "{told}");
    assert!(told.contains("Its last message was:\nTried."), "{told}");
    assert_eq!(sw::store::reviews::review_history(&t).iter().map(|e| (e.i("round"), e.t("by"), e.at("pass").clone())).collect::<Vec<_>>(), vec![(1, "codex".into(), json!(false)), (2, "codex".into(), json!(false))]);
    assert_eq!(t.i("reviewRounds"), 0);
    assert_eq!(t.at("review").at("stale"), &json!(true));
}

#[test]
fn the_successor_hears_what_the_check_last_said_so_it_does_not_trust_an_old_pass() {
    let w = world();
    let id = w.add("Ein geprüfter Stand geht über");
    sw::store::tasks::update(&id, |t| { t.put("checkPassed", json!({ "head": "abc1234", "with": "npm test" })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let told = sw::finish::hand_over::handover_prompt(&task(&id), "claude", "out of allowance", "");
    assert!(told.contains("The check last passed on revision abc1234 (command: npm test)"), "{told}");
    // A check that failed is still open, and its output goes along instead.
    sw::store::tasks::update(&id, |t| { t.put("checkPassed", Value::Null); t.put("failure", json!({ "stage": "check", "detail": "Command: npm test\n\n2 tests failed" })); Ok(sw::store::tasks::Change::Write) }).unwrap();
    let told = sw::finish::hand_over::handover_prompt(&task(&id), "claude", "out of allowance", "");
    assert!(told.contains("The check failed here, and that is still open") && told.contains("2 tests failed"), "{told}");
}
