# GitHub projects: two ways in, one repository each

**A project is a folder the board works in, and the GitHub repository that folder belongs to.** Both are known by name,
and the repository is written down — not guessed from the folder's name, which would push work somewhere else.

## The two ways in

| You have | Command | What happens |
|---|---|---|
| a repository on GitHub | `herdr-projects project add owner/name` | cloned into the projects folder, registered under its own name |
| nothing yet | `herdr-projects project create <name> --github owner/name` | a checkout with one commit, the repository on GitHub, the remote here |
| a project that is already here | `herdr-projects project set owner/name --repo NAME` | says which repository it is (the folder you are in, without `--repo`) |

`--into DIR` puts it somewhere else than the projects folder (`HERDR_CONTROL_PROJECTS_DIR`, else `~/Developer`). Without
`--github`, `project create` uses the account `gh` is logged in as. `--public` makes the repository public; it is private
otherwise.

A folder that already has something in it is never touched: the command stops and says which folder it looked at.

## In Mission Control

The same two ways in are buttons under the task composer: **[GitHub-Repo]** and **[Neues Projekt]**. Each takes over the
input box for one line — the repository, or the project's name — and `Enter` starts it; `Zurück` closes it and nothing
happens. The row of buttons grows by a line instead of dropping the ones that no longer fit.

While `gh` clones or creates, the line under it says so. When it is done, the new project is selected in the list, so its
space opens and its coordinator starts; a failure stays on that line with gh's own words, and the prompt keeps the text
so nothing has to be typed again.

## Why `create` makes a commit first

A repository without a commit has nothing to start a branch from, so no task could get a worktree of it. The first
commit is empty on purpose: no file is invented on your behalf.

## What is written down, and where

- `~/.herdr-projects/data/repos.json` — the project's name and its folder (it did this before, `store/repos.rs`).
- `~/.herdr-projects/data/github.json` — the project's repository as `owner/name` (`store/github.rs`).

The second one is what a push is checked against. A project whose checkout drifted to another repository is stopped
**before** the push, with what to do about it:

> The checkout pushes to another repository
> This project is owner/thing, but its checkout (/Users/me/Developer/thing) pushes to someone-else/thing.
> Nothing was pushed and no pull request was opened.

`herdr-projects project list` shows the same difference as `(!) its checkout pushes to …`, so it is visible before a task
ever finishes.

## Push, pull request and merge are three steps

Each has its own state on the task and its own words when it fails, so a failed push never looks like a sync:

| Step | Where it shows | On failure |
|---|---|---|
| `pushing` | the row says *Pushing the branch* | `Could not push the branch` with git's own output |
| the pull request | `pr` on the task, `prState: waiting` | `Could not open the pull request` with gh's output |
| the merge | only after your approval, and only at the reviewed commit | `Head branch was modified` and the like |

A remote that cannot be reached is not a failure of the work: the task waits as *behind main* and the board tries again
by itself (`sync_job::unpushed`). What was never pushed is never said to be pushed — `prHead` stays empty until the push
really landed.

## Where the code is

- `crates/core/src/store/github.rs` — reading a remote in every form GitHub writes it, the written-down mapping, and
  `elsewhere` (the check before a push);
- `crates/flow/src/project.rs` — the three commands: clone, create, assign;
- `src/cli/projects.rs` — what a person reads;
- `crates/board/src/mission_ui/` — the two buttons and their one-line prompt (`controls.rs`, `draw.rs`, `worker.rs`);
- `crates/flow/src/finish/pipeline.rs` — the push, the pull request and their failure states;
- tests: `tests/project.rs`, `tests/mission_control_github.rs`, and the push and pull request tests in
  `tests/pull_requests*.rs`.

More: [how work gets done](how-work-gets-done.md), [Mission Control](mission-control.md), [where things live](where-things-live.md).
