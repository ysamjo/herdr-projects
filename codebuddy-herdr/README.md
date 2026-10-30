# codebuddy-herdr

A CodeBuddy Code plugin that makes CodeBuddy a first-class agent in
[Herdr](https://herdr.dev), the terminal workspace manager for coding agents.

Herdr has no built-in CodeBuddy integration. This plugin fills that gap from the
CodeBuddy side, using Herdr's public agent-state reporting API.

## What it does

- Reports `working`, `idle`, and `blocked` to Herdr as CodeBuddy moves through a
  turn, so the sidebar, notifications, `herdr agent list`, and `herdr agent wait`
  work with CodeBuddy.
- Reports a resume command (`codebuddy --resume <session-id>`) so Herdr can restore
  the same session after a server restart.
- Releases the pane when CodeBuddy exits.
- Adds a `herdr` skill so CodeBuddy can inspect and control Herdr panes, tabs,
  workspaces, and sibling agents when it runs inside Herdr.

Everything is a no-op outside a Herdr-managed pane (`HERDR_ENV` is not `1`).

## Requirements

- Herdr 0.9.2 or newer (for resume commands). State reporting works on older 0.9.x.
- CodeBuddy Code 2.x with plugin support (`/plugin` command available).

## Try it

```bash
codebuddy --plugin-dir ~/Developer/codebuddy-herdr
```

Start CodeBuddy inside a Herdr pane, then check from another pane:

```bash
herdr agent list
herdr pane get "$HERDR_PANE_ID"
```

The agent should appear as `codebuddy` with an `idle` or `working` state.

## Install permanently

`--plugin-dir` only lasts for one session. To keep the plugin enabled, publish the
directory through a CodeBuddy plugin marketplace and install it with
`/plugin install codebuddy-herdr@<marketplace>`. For local use, a marketplace whose
`marketplace.json` points at this directory works too:

```bash
/plugin marketplace add ~/path/to/marketplace.json
/plugin install codebuddy-herdr@<marketplace-name>
```

## Event mapping

| CodeBuddy event | Herdr state |
| --- | --- |
| `SessionStart` | `idle` (claims the pane, sends resume command) |
| `UserPromptSubmit`, `PostToolUse` | `working` |
| `PermissionRequest`, `Notification: permission_prompt` | `blocked` |
| `Notification: idle_prompt`, `Stop`, `StopFailure` | `idle` |
| `SessionEnd` | release |

The hook script caches the last reported state per pane, so repeated events do not
spam the Herdr socket. Sequence numbers are monotonic per pane.

## How it works

`scripts/herdr-agent.sh` reads the hook JSON from stdin and calls:

```bash
"$HERDR_BIN_PATH" pane report-agent "$HERDR_PANE_ID" \
  --source codebuddy-plugin --agent codebuddy --state working --seq 123 \
  --agent-session-id "$SESSION_ID" \
  -- codebuddy --resume "$SESSION_ID" --permission-mode default
```

The `--source` is stable and does not use Herdr's reserved `herdr:` prefix.

Runtime state (last state and sequence number) is kept in
`${TMPDIR:-/tmp}/codebuddy-herdr/`, keyed by pane ID. Nothing is written to the
plugin directory.

## Files

```text
.codebuddy-plugin/plugin.json   manifest
hooks/hooks.json                lifecycle event wiring
scripts/herdr-agent.sh          state reporting bridge
skills/herdr/SKILL.md           Herdr control skill
```

## Uninstall

Remove the plugin through `/plugin`, or stop passing `--plugin-dir`. The plugin
writes nothing outside `${TMPDIR:-/tmp}/codebuddy-herdr/`, which can be deleted
safely.
