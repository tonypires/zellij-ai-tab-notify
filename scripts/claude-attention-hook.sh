#!/usr/bin/env bash
# Signals the zellij-ai-tab-notify Zellij plugin that the Claude Code session
# running in this pane needs the user's attention. Wired up as a `Stop` and
# `Notification` hook in Claude Code settings (see README.md).
#
# Must never fail or block the hook: no-ops silently outside Zellij, when the
# `zellij` CLI isn't available, or when the hook event isn't one we care about.
set -u

[ -n "${ZELLIJ_PANE_ID:-}" ] || exit 0
command -v zellij >/dev/null 2>&1 || exit 0

hook_input="$(cat 2>/dev/null || true)"
hook_event_name="$(printf '%s' "$hook_input" | sed -n 's/.*"hook_event_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"

case "$hook_event_name" in
    Stop) kind="finished" ;;
    Notification) kind="needs-input" ;;
    *) exit 0 ;;
esac

payload="{\"pane_id\":${ZELLIJ_PANE_ID},\"kind\":\"${kind}\"}"

# Run in the background so a slow or unresponsive `zellij pipe` can never block
# (or delay) the Claude Code hook that invoked this script.
(zellij pipe --name claude-attention -- "$payload" >/dev/null 2>&1 &) 2>/dev/null

exit 0
