//! The two ways into a GitHub project: a repository that already exists is cloned, a new project gets its own. What is
//! checked is what a later push depends on: the project is known under a name, its repository is written down, and its
//! checkout really has the remote — and that nothing is registered when gh said no.
use crate::common::*;
use sw::json::*;

/// The `project` command, as a person or an agent runs it.
fn run(argv: &[&str]) -> Value {
    let a = sw::cli::flags::parse(&argv.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::flags::refuse_unknown(argv[0], &a).unwrap_or_else(|e| panic!("{argv:?}: {}", e.msg));
    sw::cli::tasks::command(&a.pos[0], &a).unwrap_or_else(|e| panic!("{argv:?}: {}", e.msg)).0
}
fn fails(argv: &[&str]) -> String {
    let a = sw::cli::flags::parse(&argv.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    sw::cli::tasks::command(&a.pos[0], &a).err().unwrap_or_else(|| panic!("{argv:?} was to fail")).msg
}
fn projects(w: &World) -> std::path::PathBuf { let d = w.base.join("projects"); std::fs::create_dir_all(&d).unwrap(); d }

#[test]
fn a_repository_that_exists_is_cloned_and_becomes_a_project() {
    let w = world();
    let into = projects(&w).display().to_string();
    let said = run(&["project", "add", "https://github.com/owner/thing.git", "--into", &into]);
    assert_eq!(said.s("github"), "owner/thing");
    assert_eq!(said.s("repo"), "thing");
    let folder = w.base.join("projects/thing");
    assert!(folder.join(".git/config").exists(), "{}", folder.display());
    // Known under its own name, so tasks are filed on it (store/repos.rs)…
    assert_eq!(sw::store::repos::folder_of("thing"), folder);
    // …and its repository is written down, not guessed from the folder's name (store/github.rs).
    assert_eq!(sw::store::github::of("thing"), "owner/thing");
    assert!(gh_log(&w).contains(&format!("repo clone owner/thing {}", folder.display())));
}

#[test]
fn a_new_project_gets_its_own_repository() {
    let w = world();
    let into = projects(&w).display().to_string();
    let said = run(&["project", "create", "fresh", "--github", "me/fresh", "--into", &into]);
    assert_eq!((said.s("repo"), said.s("github")), ("fresh", "me/fresh"));
    let folder = w.base.join("projects/fresh");
    // A checkout with one commit: without one, no task could get a worktree of it.
    assert_eq!(git(&folder, &["rev-list", "--count", "HEAD"]).trim(), "1");
    assert_eq!(git(&folder, &["remote", "get-url", "origin"]).trim(), "https://github.com/me/fresh.git");
    assert_eq!(git(&folder, &["branch", "--show-current"]).trim(), "main");
    assert_eq!(sw::store::github::of("fresh"), "me/fresh");
    let log = gh_log(&w);
    assert!(log.contains("repo create me/fresh --source") && log.contains("--push --private"), "{log}");
}

#[test]
fn a_new_repository_goes_to_the_account_gh_is_logged_in_as() {
    let w = world();
    w.gh_says(json!({ "user": "ysamjo" }));
    let into = projects(&w).display().to_string();
    let said = run(&["project", "create", "unnamed", "--into", &into]);
    assert_eq!(said.s("github"), "ysamjo/unnamed");
    assert!(gh_log(&w).contains("repo create ysamjo/unnamed"), "{}", gh_log(&w));
}

#[test]
fn a_repository_that_cannot_be_cloned_registers_nothing() {
    let w = world();
    w.gh_says(json!({ "cloneFail": "gh: Not Found (HTTP 404)" }));
    let into = projects(&w).display().to_string();
    let e = fails(&["project", "add", "owner/missing", "--into", &into]);
    assert!(e.contains("Could not clone owner/missing") && e.contains("404"), "{e}");
    assert!(!sw::store::repos::known_repos().contains_key("missing"));
    assert!(!w.base.join("projects/missing").exists());
}

#[test]
fn a_repository_github_refuses_to_make_is_not_a_project_either() {
    let w = world();
    w.gh_says(json!({ "createRepoFail": "GraphQL: Name already exists on this account (createRepository)" }));
    let into = projects(&w).display().to_string();
    let e = fails(&["project", "create", "taken", "--github", "me/taken", "--into", &into]);
    assert!(e.contains("GitHub did not make me/taken") && e.contains("already exists"), "{e}");
    // The folder is there with its first commit, and the message says so instead of pretending nothing happened.
    assert!(w.base.join("projects/taken/.git").exists());
    assert!(!sw::store::repos::known_repos().contains_key("taken"));
}

#[test]
fn a_project_whose_checkout_pushes_elsewhere_is_said_in_the_list() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    git(&w.root, &["remote", "set-url", "origin", "https://github.com/owner/real.git"]);
    sw::store::repos::repo_of(&w.root).unwrap(); // a repo becomes a project the first time a task is filed on it
    sw::store::github::set("proj", "owner/other").unwrap();
    let list = run(&["project", "list"]);
    let row = list.as_array().cloned().unwrap_or_default().into_iter().find(|r| r.s("repo") == "proj").unwrap_or_default();
    assert_eq!((row.s("github"), row.s("origin")), ("owner/other", "owner/real"));
    assert!(!row.b("agrees"));
}

#[test]
fn a_project_can_be_told_which_repository_it_is() {
    let w = world();
    sw::store::repos::repo_of(&w.root).unwrap();
    let said = run(&["project", "set", "https://github.com/owner/real.git", "--repo", "proj"]);
    assert_eq!(said.s("github"), "owner/real");
    assert_eq!(sw::store::github::of("proj"), "owner/real");
    assert!(fails(&["project", "set", "owner/real", "--repo", "nope"]).contains("No project called \"nope\""));
}

#[test]
fn a_push_stops_when_the_checkout_names_another_repository() {
    let w = world_with(Opts { repo: true, origin: true, ..Default::default() });
    git(&w.root, &["remote", "set-url", "origin", "https://github.com/owner/real.git"]);
    sw::store::repos::repo_of(&w.root).unwrap();
    sw::store::github::set("proj", "owner/other").unwrap();
    let why = sw::store::github::elsewhere("proj");
    assert!(why.contains("This project is owner/other") && why.contains("pushes to owner/real"), "{why}");
    // Nothing written down, or both the same: the finish line goes on as before.
    sw::store::github::set("proj", "owner/real").unwrap();
    assert_eq!(sw::store::github::elsewhere("proj"), "");
    sw::store::github::set("proj", "").unwrap();
    assert_eq!(sw::store::github::elsewhere("proj"), "");
}

#[test]
fn a_remote_is_read_in_every_form_github_writes_it() {
    use sw::store::github::{parse_remote, slug};
    for (url, want) in [("git@github.com:o/r.git", "o/r"), ("https://github.com/o/r", "o/r"), ("https://github.com/o/r/", "o/r"),
        ("ssh://git@github.com/o/r.git", "o/r"), ("https://me@github.com/o/r.git", "o/r"), ("git://github.com/o/r", "o/r")] {
        assert_eq!(parse_remote(url), want, "{url}");
    }
    // Not a repository of GitHub's, or not one this can name: nothing is invented.
    for url in ["https://gitlab.com/o/r.git", "https://github.com/o/r/pull/7", "/srv/git/r.git", ""] {
        assert_eq!(parse_remote(url), "", "{url}");
    }
    assert_eq!(slug("o/r.git"), "o/r");
    assert_eq!(slug("https://github.com/o/r.git"), "o/r");
    assert_eq!(slug("just-a-name"), "");
}
