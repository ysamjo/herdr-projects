//! GitHub projects, the two ways in: a repository that already exists is cloned and becomes a project here, or a new
//! project is made with its own repository. Either way the project keeps its repository written down (store/github.rs),
//! so it stays this project's when the checkout moves. Pushing, the pull request and the merge are not done here: they
//! are the finish line's separate steps (finish/pipeline.rs), each with its own state on the task.
use crate::err::Res;
use crate::json::*;
use crate::store::github;
use crate::store::repos::{git_out, repo_of};
use std::path::{Path, PathBuf};

/// Where projects are made: Mission Control's own folder (`HERDR_CONTROL_PROJECTS_DIR`, else ~/Developer).
pub fn root() -> PathBuf { crate::mission_projects::root() }
pub struct Added { pub repo: String, pub folder: PathBuf, pub github: String }
/// One line each for a person: what a project is on GitHub, and whether its checkout agrees.
pub struct Known { pub repo: String, pub folder: PathBuf, pub github: String, pub origin: String }

fn not_empty(folder: &Path) -> bool { std::fs::read_dir(folder).map(|mut d| d.next().is_some()).unwrap_or(false) }
fn said(r: &crate::flow::git::GhOut) -> String {
    [r.stdout.trim(), r.stderr.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}
/// A folder a project may be made in: it must not be there already with something in it (that work is not ours).
fn free(folder: &Path) -> Res<()> {
    if not_empty(folder) {
        crate::fail!("{} is not empty. Nothing was touched: pick another folder with --into, or move it away.", folder.display());
    }
    Ok(())
}
fn register(folder: &Path, slug: &str) -> Res<Added> {
    let (repo, folder) = repo_of(folder)?;
    github::set(&repo, slug)?;
    Ok(Added { repo, folder, github: slug.to_string() })
}
/// A repository that exists becomes a project: cloned into the projects folder (or `into`), then known by name.
pub fn add(reference: &str, into: Option<&Path>) -> Res<Added> {
    let slug = github::slug(reference);
    if slug.is_empty() {
        crate::fail!("\"{reference}\" does not name a GitHub repository. Write it as owner/name, or give its URL (https://github.com/owner/name).");
    }
    let name = slug.split('/').nth(1).unwrap_or_default().to_string();
    let base = into.map(Path::to_path_buf).unwrap_or_else(root);
    std::fs::create_dir_all(&base)?;
    let folder = base.join(&name);
    free(&folder)?;
    let r = crate::flow::git::gh(&["repo", "clone", &slug, &folder.display().to_string()], &base, 600000);
    if r.status != 0 {
        crate::fail!("Could not clone {slug} into {}: {}\n\nNothing was registered. The board needs `gh` logged in (`gh auth login`).", folder.display(), said(&r));
    }
    if !folder.is_dir() {
        crate::fail!("gh said it cloned {slug}, but {} is not there. Nothing was registered.", folder.display());
    }
    register(&folder, &slug)
}
/// A new project with its own repository: a checkout with one commit, the repository on GitHub, the remote here.
pub fn create(name: &str, github_ref: &str, private: bool, into: Option<&Path>) -> Res<Added> {
    let name = name.trim();
    if name.is_empty() { crate::fail!("A new project needs a name: herdr-projects project create <name>"); }
    let base = into.map(Path::to_path_buf).unwrap_or_else(root);
    std::fs::create_dir_all(&base)?;
    let folder = base.join(name);
    let slug = if github_ref.trim().is_empty() { default_slug(&base, name)? } else { github::slug(github_ref) };
    if slug.is_empty() {
        crate::fail!("\"{github_ref}\" does not name a GitHub repository. Write it as owner/name, or leave --github out to use your own account.");
    }
    free(&folder)?;
    std::fs::create_dir_all(&folder)?;
    let at = folder.display().to_string();
    git_out(&folder, &["init", "-b", "main"]).map_err(|e| crate::err::Error::new(format!("{} has no git repository: {}", at, e.msg)))?;
    // A repository without a commit has nothing to start a branch from, and no task could get a worktree of it.
    git_out(&folder, &["commit", "--allow-empty", "-m", "Start"]).map_err(|e| crate::err::Error::new(format!(
        "{} is a git repository, but its first commit failed, so no repository was made: {}\n\nSet your name and address for git (git config --global user.name …, user.email …) and try again.", at, e.msg)))?;
    let vis = if private { "--private" } else { "--public" };
    let r = crate::flow::git::gh(&["repo", "create", &slug, "--source", &at, "--push", vis], &folder, 600000);
    if r.status != 0 {
        crate::fail!("{} is there with its first commit, but GitHub did not make {slug}: {}\n\nNothing was registered as a project, and no remote was added. Fix what gh says (`gh auth login`), then: herdr-projects project add {slug}", at, said(&r));
    }
    // gh normally sets the remote itself; where it did not (or could not push), the checkout still gets it, so the
    // finish line's push has somewhere to go. A remote that is already there is never replaced.
    if !crate::flow::git::has_origin(&folder) {
        git_out(&folder, &["remote", "add", "origin", &github::remote(&slug)]).map_err(|e| crate::err::Error::new(format!(
            "The repository {slug} exists and {} is there, but the checkout has no origin remote: {}", at, e.msg)))?;
    }
    register(&folder, &slug)
}
/// Which account a new repository belongs to, when nobody said: the one gh is logged in as.
fn default_slug(base: &Path, name: &str) -> Res<String> {
    let user = crate::flow::git::gh_user(base);
    if user.is_empty() {
        crate::fail!("Not logged in to GitHub, so no repository could be named. Run `gh auth login`, or say which: herdr-projects project create {name} --github owner/{name}");
    }
    Ok(format!("{user}/{name}"))
}
/// Says which repository a project that already exists is: one made before this, or one whose repository moved. The
/// folder is the project's (--repo), or the one the command runs in.
pub fn assign(reference: &str, repo: &str) -> Res<Added> {
    let slug = github::slug(reference);
    if slug.is_empty() { crate::fail!("\"{reference}\" does not name a GitHub repository. Write it as owner/name, or give its URL."); }
    let (repo, folder) = if repo.trim().is_empty() {
        repo_of(&std::env::current_dir()?)?
    } else {
        let found = crate::store::repos::known_repos().keys().find(|k| k.to_lowercase() == repo.trim().to_lowercase()).cloned();
        match found { Some(k) => (k, crate::store::repos::folder_of(repo)), None => crate::fail!("No project called \"{repo}\". herdr-projects project list shows them.") }
    };
    github::set(&repo, &slug)?;
    Ok(Added { repo, folder, github: slug })
}
/// Every project, with the repository it is on GitHub (store/github.rs) and what its own checkout says.
pub fn known() -> Vec<Known> {
    let mut out: Vec<Known> = crate::store::repos::known_repos().iter().map(|(repo, folder)| {
        let folder = PathBuf::from(text(folder));
        let origin = if folder.join(".git").exists() { git_out(&folder, &["remote", "get-url", "origin"]).map(|u| github::parse_remote(&u)).unwrap_or_default() } else { String::new() };
        Known { repo: repo.clone(), folder, github: github::of(repo), origin }
    }).collect();
    out.sort_by(|a, b| a.repo.to_lowercase().cmp(&b.repo.to_lowercase()));
    out
}
