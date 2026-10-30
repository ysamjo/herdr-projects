//! Der Nachweis über die ganze Kette: zwei Aufgaben laufen gleichzeitig, eine verliert ihr Kontingent, die nächste CLI
//! übernimmt im selben Worktree ohne Doppelarbeit, und beide enden mit einem Pull Request.
use crate::common::*;
use serde_json::json;
use sw::json::*;
use sw::store::handovers::{handover_line, handovers};

#[test]
fn two_tasks_one_loses_its_allowance_the_next_cli_finishes_it_and_both_end_with_a_pull_request() {
    let w = cut_world_with(Opts { origin: true, settings: json!({ "worker.fallback": "codex", "review.when": "never" }), ..Default::default() }, "30");
    // Three turns, in the order they are asked for: A's first agent, B's agent, A's successor.
    w.script(json!({ "worker": [
        { "files": { "half.txt": "first half\n" }, "commit": "first half", "ends": "claude-usage-limit-made" },
        { "files": { "b.txt": "b work\n" }, "commit": "b work", "note": "B is done." },
        { "files": { "second.txt": "second half\n" }, "commit": "second half", "note": "Finished the rest." }
    ] }));
    let a = w.add("Aufgabe A, halb fertig");
    let b = w.add("Aufgabe B, ungestört");
    delegate(&a);
    delegate(&b);
    let ta = settle(&a);
    let tb = settle(&b);
    assert_eq!(ta.s("stage"), "approval", "{}", ta.at("failure"));
    assert_eq!(tb.s("stage"), "approval", "{}", tb.at("failure"));
    // One task changed hands, the other did not: the hand-over stays inside its own worktree.
    assert_eq!(handovers(&ta).len(), 1);
    assert!(handover_line(&handovers(&ta)[0]).starts_with("codex took over from claude: out of allowance"), "{}", handover_line(&handovers(&ta)[0]));
    assert!(handovers(&tb).is_empty());
    assert_eq!((ta.s("agent"), tb.s("agent")), ("codex", "claude"));
    assert_ne!(ta.s("branch"), tb.s("branch"));
    assert_ne!(ta.s("worktree"), tb.s("worktree"));
    // No double work: what the first agent committed is there once, and the successor added to it.
    let tree_a = std::path::Path::new(ta.s("worktree"));
    let base = if ta.b("baseSha") { ta.t("baseSha") } else { ta.t("from") };
    assert_eq!(git(tree_a, &["log", "--format=%s", &format!("{base}..HEAD")]).lines().collect::<Vec<_>>(), vec!["second half", "first half"]);
    assert_eq!(read(&tree_a.join("half.txt")), "first half\n");
    assert_eq!(read(&tree_a.join("second.txt")), "second half\n");
    // The successor was told what was there before it started.
    let told = prompts(&w, ta.at("link").s("pane"));
    assert!(told[0].contains("first half") && told[0].contains("carry on from where it stands"), "{:?}", told.first());
    // B's work is untouched by A's hand-over.
    let tree_b = std::path::Path::new(tb.s("worktree"));
    assert_eq!(read(&tree_b.join("b.txt")), "b work\n");
    assert!(!tree_b.join("half.txt").exists() && !tree_b.join("second.txt").exists());
    // Both end the same way: pushed, and a pull request opened for each branch.
    for t in [&ta, &tb] {
        assert!(t.s("pr").contains("/pull/"), "{}: {}", t.s("branch"), t.s("pr"));
        assert!(gh_log(&w).contains(&format!("pr create --draft --head {}", t.s("branch"))), "{}", gh_log(&w));
    }
}
