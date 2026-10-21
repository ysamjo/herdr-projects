# Security

## Reporting a vulnerability

Report privately through GitHub's private vulnerability reporting:
https://github.com/eliasstravik/herdr-projects/security/advisories/new

Please do not open a public issue for a vulnerability. Say what you ran, what happened, and what you expected; a proof
of concept helps. Fixes go to `main` and into the next release.

## What the board promises, and what it does not

Herdr Projects is a local tool. Agents it starts run as your Unix user with your rights, and the review gate is a
quality check, not an isolation boundary ([what the review is not](docs/gate.md#what-the-review-is-not)). By default
nothing merges without your yes (`merge.by = you`), and a repo's own file cannot switch that off. The command line
refuses an approval from a task's own agent or reviewer pane, but that marker is a speed bump, not isolation.

Reports are still welcome when the board can reduce the damage: for example when it runs untrusted content with more
rights than it needs, shows or leaks a secret, or lets a task escape the approval you gave.
