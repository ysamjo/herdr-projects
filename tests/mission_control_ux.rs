//! Compact project input, mobile terminal focus, saved conclusions, request grouping, and visible CLI choices.
use crate::common::*;
use serde_json::json;
use sw::herdr::Herdr;
use sw::json::*;
use sw::mission_ui::{State, controls::{self, Action}, draw, worker::Command};
use std::sync::mpsc::{channel, Receiver};

fn drain() { flush(); sw::runner::drain_jobs(&|job, id, extra| { sw::jobs::run_job(job, id, extra); flush(); }); }
fn mission(w: &World) -> (String, sw::coordinator::co::Co) {
    sw::mission_control::apply_priority("agy,codex").unwrap();
    let api = Herdr::new(Some(w.socket.clone()));
    let ws = api.workspace_create(&w.root_str(), "proj", &[]).unwrap();
    let root = ws.at("root_pane").t("pane_id");
    sw::mission_layout::register_project(&w.socket, &w.root_str(), ws.at("workspace").s("workspace_id"), &root).unwrap();
    sw::mission_layout::selected(&w.socket, &w.root_str()).unwrap();
    sw::mission_control::ensure(&w.socket).unwrap(); drain();
    (root, sw::mission_control::project_coordinator(&w.root, &w.socket).unwrap())
}
fn ui(w: &World) -> (State, Receiver<Command>) {
    let (tx, rx) = channel();
    let mut s = State {
        projects: vec![sw::mission_projects::Project { path:w.root.clone(), name:"proj".into(), remote:"https://github.com/example/proj".into() }], project:Default::default(),
        draft:Default::default(),order:Default::default(),priority:false,sidebar:false,sending:false,notice:String::new(),error:String::new(),
        coordinator:json!({"agent":"agy","state":"idle","selection":{"reason":"agy ist Priorität 1."}}),drafts:Default::default(),
        folders_rect:Default::default(),input_rect:Default::default(),send_rect:Default::default(),priority_rect:Default::default(),refresh_rect:Default::default(),
        conclusion:serde_json::Value::Null,expanded:false,summary_scroll:0,controls:vec![],socket:w.socket.clone(),tx,closed_at:None,
    };
    s.project.select(Some(0)); (s,rx)
}
fn render(s: &mut State, width: u16, height: u16) -> String {
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width,height)).unwrap();
    terminal.draw(|f|draw::draw(f,s)).unwrap();
    terminal.backend().buffer().content.iter().map(|c|c.symbol()).collect()
}
fn research(w: &World, title: &str) -> String {
    let id = w.add(title);
    sw::store::tasks::patch(&id,|t| {t.put("research",true);t.put("stage","done");t.put("missionImplementationApproval","pending");t.put("report",json!({"file":"report.md","link":"https://example/report"}));}).unwrap();
    sw::store::files::write_json(&sw::mission_conclusion::file(&task(&id)),&json!({"task":id,"conclusion":"Fazit bleibt sichtbar.","findings":["Ein konkreter Befund"],"proposal":"Zwei kleine Änderungen", "questions":["Welche Farbe?"]})).unwrap();
    id
}
#[test]
fn coordinator_gets_the_upper_area_and_input_keeps_a_compact_bottom_strip() {
    let w = world(); let (root,co) = mission(&w);
    let api = Herdr::new(Some(w.socket.clone())); let c=sw::coordinator::state::get(co).unwrap();
    let layout=api.pane_layout(&root).unwrap();
    let input=layout.arr("panes").iter().find(|p|p.s("pane_id")==root).unwrap().at("rect");
    let agent=layout.arr("panes").iter().find(|p|p.s("pane_id")==c.s("pane")).unwrap().at("rect");
    assert!(input.i("y")>agent.i("y")); assert!(input.i("height")<=10); assert_eq!(input.i("width"),120);
    let (mut s,_)=ui(&w); let text=render(&mut s,120,7);
    assert_eq!(s.input_rect.height,1); assert!(text.contains("[Senden]")); assert!(text.contains("[Fokus]"));
}
#[test]
fn mobile_focus_switches_existing_agent_and_input_without_creating_tabs_or_panes() {
    let w=world(); let (root,co)=mission(&w); let api=Herdr::new(Some(w.socket.clone()));
    let c=sw::coordinator::state::get(co).unwrap(); let count=api.panes(None).unwrap().len();
    sw::mission_view::mobile(&w.socket,&w.root,true).unwrap();
    let layout=api.pane_layout(c.s("pane")).unwrap(); assert!(layout.b("zoomed")); assert_eq!(layout.s("focused_pane_id"),c.s("pane"));
    sw::mission_view::focus(&w.socket,&w.root,"input").unwrap();
    assert_eq!(api.pane_layout(&root).unwrap().s("focused_pane_id"),root);
    sw::mission_view::focus(&w.socket,&w.root,"next").unwrap();
    assert_eq!(api.pane_layout(&root).unwrap().s("focused_pane_id"),c.s("pane"));
    sw::mission_view::mobile(&w.socket,&w.root,false).unwrap(); assert!(!api.pane_layout(&root).unwrap().b("zoomed"));
    assert_eq!(api.panes(None).unwrap().len(),count); assert_eq!(w.state().arr("tabs").len(),1);
    let file=sw::store::files::data_dir().join(format!("mission-view-{}-{}.json",sw::herdr::cli::session_name(&w.socket),sw::mission_layout::project_key(&w.root_str())));
    sw::store::files::write_json(&file,&json!({})).unwrap();
    api.workspace_focus(c.s("space")).unwrap();
    w.patch(|st| {for layout in st["layouts"].as_object_mut().unwrap().values_mut(){layout["area"]["width"]=json!(40);}});
    sw::mission_view::auto(&w.socket,&w.root).unwrap(); assert!(sw::mission_view::preferences(&w.socket,&w.root).b("mobile"));
    sw::mission_view::mobile(&w.socket,&w.root,false).unwrap();
    sw::mission_view::auto(&w.socket,&w.root).unwrap(); assert!(!sw::mission_view::preferences(&w.socket,&w.root).b("mobile"),"manual choice wins");

}
#[test]
fn phone_input_shows_fazit_and_the_task_bound_approval_button() {
    let w=world(); let id=research(&w,"Analyse"); let (mut s,rx)=ui(&w);
    s.conclusion=sw::mission_conclusion::latest(&w.root); s.expanded=true;
    let text=render(&mut s,40,30);
    assert!(text.contains("Fazit bleibt sichtbar.")); assert!(text.contains("Umsetzungsvorschlag")); assert!(text.contains("[Umsetzung freigeben]"));
    assert!(s.controls.iter().all(|(r,_)|r.right()<=40 && r.bottom()<=30));
    let action=s.controls.iter().find_map(|(_,a)|if let Action::Approve(_)=a{Some(a.clone())}else{None}).unwrap();
    controls::act(&mut s,action);
    assert!(matches!(rx.try_recv().unwrap(),Command::Approve(folder,task) if folder==w.root && task==id));
    let (mut pending,_)=ui(&w); pending.conclusion=json!({"pending":true,"ready":false,"task":id});
    render(&mut pending,40,20); assert!(!pending.controls.iter().any(|(_,a)|matches!(a,Action::Approve(_))));
}
#[test]
fn approval_stays_bound_to_the_shown_report_and_rejects_missing_or_foreign_conclusions() {
    let w=world(); let (_,co)=mission(&w); let old=research(&w,"Erster Auftrag"); let other=research(&w,"Neuer Auftrag");
    sw::mission_conclusion::approve(&w.root,&old,&w.socket).unwrap(); drain();
    assert_eq!(task(&old).s("missionImplementationApproval"),"approved"); assert_eq!(task(&other).s("missionImplementationApproval"),"pending");
    let c=sw::coordinator::state::get(co).unwrap(); let text=prompts(&w,c.s("pane")).last().unwrap().clone(); assert!(text.contains(&old)); assert!(!text.contains(&format!("Recherche-Task {other}")));
    assert!(sw::mission_conclusion::approve(&w.root,&old,&w.socket).is_err());
    sw::store::files::write_json(&sw::mission_conclusion::file(&task(&other)),&json!({"task":old,"conclusion":"Falsch","findings":[],"proposal":"Plan","questions":[]})).unwrap();
    assert!(sw::mission_conclusion::approve(&w.root,&other,&w.socket).is_err());
    assert!(sw::mission_conclusion::approve(&w.base,&other,&w.socket).is_err());
    let co=sw::mission_control::project_coordinator(&w.root,&w.socket).unwrap();
    let pane=sw::coordinator::state::get(&co).unwrap().t("pane");
    sw::coordinator::state::change(&co,|c| {c.put("where",json!([{"said":"[MISSION-CONTROL-RESEARCH-RESULT]", "root":w.root_str()}]));true}).unwrap();
    unsafe {std::env::set_var("HERDR_PANE_ID",pane);}
    let before=sw::store::tasks::list(Default::default()).len();
    assert!(sw::actions::filing::add(&w.root,"Verfrühte Umsetzung",Default::default()).is_err());
    assert_eq!(sw::store::tasks::list(Default::default()).len(),before);

}
#[test]
fn worker_tasks_inherit_the_human_request_and_native_sidebar_uses_german_status() {
    let w=world(); let (_,co)=mission(&w);
    sw::mission_control::submit(&w.root,"Suche überarbeiten",&w.socket).unwrap(); drain();
    let c=sw::coordinator::state::get(co).unwrap();
    let request=sw::mission_requests::inherit(&w.root,c.s("pane"),&w.socket).unwrap();
    assert_eq!(request.s("title"),"Suche überarbeiten");
    unsafe { std::env::set_var("HERDR_PANE_ID", c.s("pane")); }
    let added=sw::actions::filing::add(&w.root,"Suchfeld",Default::default()).unwrap();
    assert_eq!(added.at("missionRequest"),&request);
    let id=added.t("id"); sw::store::tasks::patch(&id,|t| {t.put("stage","reviewing");t.put("agent","codex");}).unwrap();
    let e=sw::flow::sidebar_entry::entry_of(&task(&id),"",0);
    assert!(e.name.contains("Suche überarbeiten ▸")); assert!(e.name.contains("Suchfeld")); assert!(e.stands.contains("im Review"));
    for (stage,label) in [("working","arbeitet"),("asks","wartet auf dich"),("done","fertig")] {
        sw::store::tasks::patch(&id,|t|t.put("stage",stage)).unwrap(); assert_eq!(sw::mission_status::status(&task(&id)),label);
    }
}
#[test]
fn provider_choice_exposes_actual_cli_configured_cli_and_handover_reason() {
    let w=world(); let (_,co)=mission(&w);
    sw::coordinator::state::change(&co,|c| {c.obj_mut("run").put("kind","codex");c.put("controlPriority",sw::mission_control::priority());c.put("controlFallback","codex");c.put("controlHandover","agy → codex: Kontingent erreicht");true}).unwrap();
    let choice=sw::mission_status::coordinator(&co); assert_eq!(choice.s("actual"),"codex");assert_eq!(choice.s("configured"),"agy");assert!(choice.s("reason").contains("Kontingent erreicht"));
    sw::mission_control::apply_priority("claude,codex").unwrap();
    assert!(sw::mission_status::coordinator(&co).s("reason").contains("Gewählt: claude"));
}

#[test]
fn old_pending_reports_request_a_saved_conclusion_once_without_restarting_the_coordinator() {
    let w=world(); let (_,co)=mission(&w); let id=research(&w,"Alter Recherchebericht");
    std::fs::remove_file(sw::mission_conclusion::file(&task(&id))).unwrap();
    let pane=sw::coordinator::state::get(&co).unwrap().t("pane");
    sw::mission_conclusion::request(&w.root,&id,&w.socket,false).unwrap(); drain();
    let count=prompts(&w,&pane).len();
    assert!(prompts(&w,&pane).last().unwrap().contains("conclusion.json"));
    sw::mission_conclusion::request(&w.root,&id,&w.socket,false).unwrap(); drain();
    assert_eq!(prompts(&w,&pane).len(),count);
    assert_eq!(sw::coordinator::state::get(co).unwrap().s("pane"),pane);
    assert_eq!(task(&id).s("missionImplementationApproval"),"pending");
}
