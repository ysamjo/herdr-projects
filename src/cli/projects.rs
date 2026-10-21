//! The `project` command: a project from a GitHub repository that exists, a new one with its own, or the list of what
//! each project is on GitHub. What it does is flow/project.rs; this only says it.
use super::flags::Args;
use crate::err::Res;
use crate::json::*;
use crate::project::{self, Added};

fn folder_of(a: &Args) -> Option<std::path::PathBuf> {
    a.flags.get("into").filter(|v| !matches!(v, Value::Bool(_))).map(text).filter(|s| !s.is_empty()).map(std::path::PathBuf::from)
}
fn added(r: Added, how: &str) -> (Value, String) {
    let v = json!({ "repo": r.repo, "folder": r.folder.display().to_string(), "github": r.github });
    let text = format!("{}: {how} · {} · github.com/{}", r.repo, r.folder.display(), r.github);
    (v, text)
}
pub fn command(a: &Args) -> Res<(Value, String)> {
    let what = a.pos.get(1).cloned().unwrap_or_else(|| "list".into());
    let arg = |n: usize| a.pos.get(n).cloned().unwrap_or_default();
    Ok(match what.as_str() {
        "add" => {
            if arg(2).is_empty() { crate::fail!("project add needs a repository: herdr-projects project add owner/name [--into DIR]"); }
            added(project::add(&arg(2), folder_of(a).as_deref())?, "cloned")
        }
        "create" => {
            if arg(2).is_empty() { crate::fail!("project create needs a name: herdr-projects project create <name> [--github owner/name] [--public]"); }
            let named = a.flags.get("github").filter(|v| !matches!(v, Value::Bool(_))).map(text).unwrap_or_default();
            added(project::create(&arg(2), &named, !a.flags.b("public"), folder_of(a).as_deref())?, "new")
        }
        "set" => {
            if arg(2).is_empty() { crate::fail!("project set needs the repository: herdr-projects project set owner/name [--repo NAME]"); }
            let repo = a.flags.get("repo").filter(|v| !matches!(v, Value::Bool(_))).map(text).unwrap_or_default();
            added(project::assign(&arg(2), &repo)?, "says its repository is")
        }
        "list" | "ls" => {
            let all = project::known();
            let rows: Vec<Value> = all.iter().map(|k| json!({
                "repo": k.repo, "folder": k.folder.display().to_string(), "github": k.github,
                // What the checkout itself says: a project whose mapping and origin differ is said so, never guessed at.
                "origin": k.origin, "agrees": k.github.is_empty() || k.origin.is_empty() || k.github == k.origin,
            })).collect();
            let lines: Vec<String> = all.iter().map(|k| {
                let gh = if k.github.is_empty() { "no GitHub repository".to_string() } else { k.github.clone() };
                let odd = if !k.github.is_empty() && !k.origin.is_empty() && k.github != k.origin { format!("  (!) its checkout pushes to {}", k.origin) } else { String::new() };
                format!("{:<18} {:<32} {}{}", k.repo, gh, k.folder.display(), odd)
            }).collect();
            (Value::Array(rows), if lines.is_empty() { "No projects yet. herdr-projects project add owner/name".into() } else { lines.join("\n") })
        }
        _ => crate::fail!("project takes: add <owner/name>, create <name> [--github owner/name] [--public], list. Try: herdr-projects help"),
    })
}
