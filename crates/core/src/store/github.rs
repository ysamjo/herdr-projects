//! Which GitHub repository a board repo is: `owner/name`, read from the checkout's own `origin` remote, or written
//! down once when the project was made from a repository that was picked or created (flow/project.rs). The board never
//! guesses a repository from a folder's name: a wrong guess would push somewhere else.
use super::files::*;
use super::repos::{folder_of, git_out};
use crate::err::Res;
use crate::json::*;
use std::path::PathBuf;

fn file() -> PathBuf { data_dir().join("github.json") }
fn ok_char(c: char) -> bool { c.is_ascii_alphanumeric() || "-_.".contains(c) }
/// `owner/name` from any form a remote is written in (`git@github.com:o/r.git`, `https://github.com/o/r`), or "" when
/// it is not a GitHub repository this can name.
pub fn parse_remote(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let at = match u.find("github.com") { Some(a) => a, None => return String::new() };
    let after = &u[at + "github.com".len()..];
    // `github.com/o/r` (https, ssh, git://) and `github.com:o/r` (scp-like): anything else is not a repository.
    let rest = match after.strip_prefix('/').or_else(|| after.strip_prefix(':')) { Some(r) => r.trim_end_matches('/'), None => return String::new() };
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() != 2 || !parts.iter().all(|p| !p.is_empty() && p.chars().all(ok_char)) { return String::new(); }
    format!("{}/{}", parts[0], parts[1])
}
/// What a person typed as a repository: `owner/name`, or any URL of it. "" when it is neither.
pub fn slug(input: &str) -> String {
    let s = input.trim();
    let parsed = parse_remote(s);
    if !parsed.is_empty() { return parsed; }
    let bare = s.trim_end_matches('/').strip_suffix(".git").unwrap_or(s.trim_end_matches('/'));
    let parts: Vec<&str> = bare.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() == 2 && parts.iter().all(|p| p.chars().all(ok_char)) { format!("{}/{}", parts[0], parts[1]) } else { String::new() }
}
/// The URL a checkout gets as its `origin` when the repository was created here.
pub fn remote(slug: &str) -> String { format!("https://github.com/{slug}.git") }
/// What was written down for this project, without looking at its checkout.
pub fn recorded(repo: &str) -> String { read_obj(&file()).and_then(|m| m.get(repo).map(|v| text(v))).unwrap_or_default() }
/// What the project's own checkout pushes to, as a GitHub repository ("" for a remote that is not GitHub's).
pub fn origin(repo: &str) -> String {
    let folder = folder_of(repo);
    if !folder.join(".git").exists() { return String::new(); }
    git_out(&folder, &["remote", "get-url", "origin"]).map(|u| parse_remote(&u)).unwrap_or_default()
}
/// The project's repository: what was written down, else its checkout's own origin. "" when neither.
pub fn of(repo: &str) -> String {
    let said = recorded(repo);
    if !said.is_empty() { return said; }
    origin(repo)
}
/// What to say when the checkout pushes somewhere else than the project's repository: a push would land in the wrong
/// place, so the finish line stops before it. "" when they agree, or when one of them is not known.
pub fn elsewhere(repo: &str) -> String {
    let (said, at) = (recorded(repo), origin(repo));
    if said.is_empty() || at.is_empty() || said == at { return String::new(); }
    format!("This project is {}, but its checkout ({}) pushes to {}.", said, folder_of(repo).display(), at)
}
/// Written down once, so a repository picked or created stays this project's even when the checkout moves.
pub fn set(repo: &str, slug: &str) -> Res<()> {
    with_lock("github", || {
        let mut m = read_obj(&file()).unwrap_or_default();
        m.insert(repo.to_string(), Value::String(slug.to_string()));
        write_json(&file(), &Value::Object(m))
    })
}
