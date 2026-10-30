# What is known about each allowance

**One record for the whole machine: which subscription is out of allowance, which needs a login, and when it is worth
looking again.** Every project, every task and every model of that subscription reads it. A model that shares a
subscription with the one that just ran dry is not tried next.

It keeps what a provider itself said — never how much is left. There is no percentage anywhere, because most of these
programs do not say, and a guessed one would be worse than none.

## The four states

| State | Means | What the board does |
|---|---|---|
| `available` | it answers | nothing is known against it |
| `degraded` | it failed in a way that passes by itself (overloaded, a rate limit, a dropped connection) | tried last for a while, never refused |
| `exhausted` | out of allowance | tried last until the time the provider named, else for fifteen minutes |
| `login` | a login is needed | tried last until a turn of that subscription works again |

Each entry keeps the cause in the provider's own words, the task it was seen on, and when it was seen. An entry whose
time is up disappears by itself; nothing has to be cleaned up.

## Which models share an allowance

Several models of one program draw on the same subscription, so handing a task to another model of the same program
would land in the same empty budget. The board knows these:

| Program | Allowance |
|---|---|
| `claude` | `anthropic` |
| `codex` | `openai` |
| `agy`, `gemini` | `google` |
| `copilot` | `github` |
| `cursor`, `cline`, `kiro`, `devin`, `droid`, `grok`, `qwen` | their own |
| anything else | the program itself |

Two programs that happen to bill the same account (`opencode` on an Anthropic key, say) are not known to share it: the
board cannot see that, and guessing would keep a working program out of the rotation.

## What it changes

- **A new task's agent** (`worker_choice`): the order is the task's `worker.agent`, then `worker.fallback`, then the
  global CLI order — with everything that is out of allowance or needs a login put last. The list's own order is kept
  within each rank, and nothing is dropped: with everything out of allowance the first is still tried, and fails where
  you can see it.
- **The Coordinator's next program** (a provider hand-over): the next entry of the global order that is installed and
  whose subscription is not out of allowance or logged out.
- **Nothing else.** A subscription is never refused for a task you started by hand, and a task already under way keeps
  its agent: this is a preference, not a lock.

## Where it is written

- A turn that ended with a provider error notes its subscription: `login`, `exhausted` (with the reset time the
  provider named) or `degraded` (`crates/flow/src/finish/turn_ended.rs`).
- The Coordinator's own hand-over notes it too (`crates/flow/src/mission_fallback.rs`).
- A turn that ended properly releases it: the program answered, so nothing is known against it.

The file is `~/.herdr-projects/data/quota.json` (`store/quota.rs`), written under the board's data folder, so every
project and every board of this machine sees the same record. Tests: `tests/quota.rs`.

More: [the fallback agent](fallback.md), [a turn cut off by the provider](provider-errors.md).
