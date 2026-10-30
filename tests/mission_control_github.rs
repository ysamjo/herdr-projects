//! The two ways into a GitHub project in Mission Control: a button each, and one line that is the repository or the
//! project's name. What is checked is that the buttons are there, that the prompt takes the input box, and that Enter
//! hands exactly that line to the worker (which does the cloning, flow/project.rs).
use crate::common::*;
use ratatui::backend::TestBackend;
use sw::json::*;
use sw::mission_ui::{Git, State, controls::{self, Action}, draw};

fn state(w: &World) -> State {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut s = State {
        projects: vec![sw::mission_projects::Project { name: "proj".into(), path: w.root.clone(), remote: String::new() }],
        project: Default::default(), draft: Default::default(), order: Default::default(), priority: false, sidebar: false, sending: false, attaching: false, git: None,
        notice: String::new(), error: String::new(), coordinator: json!({ "agent": "agy" }), drafts: Default::default(),
        folders_rect: Default::default(), input_rect: Default::default(), send_rect: Default::default(), priority_rect: Default::default(), refresh_rect: Default::default(),
        conclusion: serde_json::Value::Null, expanded: false, summary_scroll: 0, controls: vec![],
        socket: w.socket.clone(), tx, closed_at: None,
    };
    s.project.select(Some(0));
    s
}
fn screen(s: &mut State) -> String {
    let mut terminal = ratatui::Terminal::new(TestBackend::new(104, 40)).unwrap();
    terminal.draw(|f| draw::draw(f, s)).unwrap();
    terminal.backend().buffer().content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn both_ways_in_are_buttons_and_the_prompt_takes_the_input_box() {
    let w = world();
    let mut s = state(&w);
    let text = screen(&mut s);
    assert!(text.contains("[GitHub-Repo]") && text.contains("[Neues Projekt]"), "{text}");
    // A repository that exists: the button opens the prompt in place of the task composer.
    let repo = s.controls.iter().find(|(_, a)| matches!(a, Action::Github)).map(|(r, a)| (*r, a.clone())).unwrap();
    controls::act(&mut s, repo.1);
    assert_eq!(s.git, Some(Git::Repo));
    assert!(s.prompting());
    let text = screen(&mut s);
    assert!(text.contains("Projekt aus GitHub · owner/name") && text.contains("[Anlegen]") && text.contains("[Zurück]"), "{text}");
    // Back closes it, and the task composer is back.
    controls::act(&mut s, Action::Github);
    assert_eq!(s.git, None);
    assert!(screen(&mut s).contains("Task für proj"));
}

#[test]
fn a_new_project_asks_for_its_name_and_not_for_a_repository() {
    let w = world();
    let mut s = state(&w);
    controls::act(&mut s, Action::NewProject);
    assert_eq!(s.git, Some(Git::New));
    let text = screen(&mut s);
    assert!(text.contains("Neues Projekt · Name") && text.contains("[Anlegen]"), "{text}");
}

#[test]
fn enter_hands_the_repository_to_the_worker_and_an_empty_line_is_refused() {
    let w = world();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut s = state(&w);
    s.tx = tx;
    controls::act(&mut s, Action::Github);
    while rx.try_recv().is_ok() {} // the focus request the prompt sends
    // Nothing typed: said so, and the worker hears nothing.
    controls::act(&mut s, Action::Send);
    assert!(s.error.contains("owner/name"), "{}", s.error);
    assert!(rx.try_recv().is_err());
    s.order.set_text("https://github.com/owner/thing.git");
    controls::act(&mut s, Action::Send);
    match rx.try_recv() {
        Ok(sw::mission_ui::worker::Command::Github(Git::Repo, line)) => assert_eq!(line, "https://github.com/owner/thing.git"),
        _ => panic!("the prompt did not reach the worker"),
    }
    assert!(s.sending, "the screen says it is working while gh clones");
}
