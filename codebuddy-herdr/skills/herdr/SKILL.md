---
name: herdr
description: Inspect and control Herdr panes, tabs, workspaces, and sibling agents from inside a Herdr session. Use only when the user explicitly mentions Herdr or asks to control Herdr panes, tabs, workspaces, or another agent. Requires HERDR_ENV=1.
---

# Herdr

Herdr is a terminal workspace manager for coding agents. It organizes terminals into
workspaces, tabs, and panes, recognizes agents running in panes, and exposes the
session through the `herdr` CLI.

## Verify you are inside Herdr

```bash
test "${HERDR_ENV:-}" = 1
```

If the check fails, say that you are not running inside a Herdr pane and stop. Do
not inspect or control a Herdr session from outside it.

## Learn the installed CLI

The installed binary is the authority for syntax. Start with `herdr --help`, then
print a command group by running it without a subcommand:

```bash
herdr workspace
herdr tab
herdr pane
herdr agent
herdr notification
```

Do not run bare `herdr` for discovery: it launches or attaches the TUI. Do not
probe a mutating command by omitting arguments; commands such as
`herdr workspace create` are valid with defaults and will execute.

Most control commands return JSON. Read IDs and state from the response instead of
predicting them.

## IDs and caller context

Public IDs are opaque and stable:

- workspace: `w1`
- tab: `w1:t1`
- pane: `w1:p1`

Herdr injects the calling pane's context:

```bash
printf '%s\n' "$HERDR_WORKSPACE_ID" "$HERDR_TAB_ID" "$HERDR_PANE_ID"
```

Prefer `--current` when a pane command should target the calling pane. An omitted
split target uses the calling pane when `HERDR_PANE_ID` is available, otherwise the
focused pane.

Discover live state:

```bash
herdr workspace list
herdr tab list --workspace "$HERDR_WORKSPACE_ID"
herdr pane current --current
herdr pane list --workspace "$HERDR_WORKSPACE_ID"
herdr agent list
```

## Split and run commands

Default to a sibling pane in the current tab and the current working directory. Do
not create a workspace, tab, or worktree unless the user asks for that topology.

Inspect the caller pane first, then split in the direction that fits:

```bash
herdr pane layout --pane "$HERDR_PANE_ID"
herdr pane split --current --direction right --cwd "$PWD" --no-focus
```

Read the new pane ID from `.result.pane.pane_id`. Run a command in it with:

```bash
herdr pane run "$PANE_ID" npm test
herdr pane read "$PANE_ID" --source recent --lines 200
herdr pane wait-output "$PANE_ID" --match "ready" --timeout 60000
```

Send raw input only when a command is already running in the pane:

```bash
herdr pane send-text "$PANE_ID" "text"
herdr pane send-keys "$PANE_ID" Enter
```

## Work with agents

Agent commands accept a unique live agent name or the pane ID hosting that agent.

```bash
herdr agent list
herdr agent get "$AGENT"
herdr agent read "$AGENT" --source recent --lines 200
herdr agent prompt "$AGENT" "Run the test suite and report failures." --wait --timeout 300000
herdr agent wait "$AGENT" --until idle --timeout 300000
herdr agent focus "$AGENT"
```

`idle` and `done` mean the agent is ready for input. `blocked` means Herdr
recognized an approval or question UI. `unknown` does not prove completion.

Start a supported agent in an existing shell pane only when the user asks:

```bash
herdr agent start reviewer --kind claude --pane "$PANE_ID" -- --model sonnet
```

CodeBuddy is not a built-in `agent start` kind. To launch another CodeBuddy
instance, run it as a normal command instead:

```bash
herdr pane run "$PANE_ID" codebuddy
```

This plugin reports the current CodeBuddy session to Herdr, so `herdr agent list`
should show it as `codebuddy` while it runs.

## Notifications

```bash
herdr notification show "Build finished" --body "All tests passed" --sound done
```

## Safety

- Keep the user's focus in the calling pane unless they ask to move it.
- Do not close, move, or rename panes, tabs, or workspaces without asking.
- Do not send destructive input to another agent without asking.
- Use `--no-focus` when creating panes for background work.
- Read IDs from command output; do not reuse IDs from a closed pane or another
  machine.
