# zellij-ai-tab-notify

A [Zellij](https://zellij.dev) plugin that renames a background tab to flag it as
"needs attention" when a process running in one of its panes — in particular a
[Claude Code](https://claude.com/claude-code) session — finishes responding or is
waiting on input. The marker clears automatically the moment you focus that tab
again, and your original tab name (including any name you set by hand) is restored
exactly.

The plugin never reads pane content. It only reacts to explicit signals sent over
Zellij's `zellij pipe` mechanism, typically from a Claude Code hook.

## Building

Requires Rust with the `wasm32-wasip1` target:

```bash
rustup target add wasm32-wasip1
cd zellij-ai-tab-notify
cargo build --release --target wasm32-wasip1
```

This produces `target/wasm32-wasip1/release/zellij_ai_tab_notify.wasm`.

Run the unit tests (pure logic only — no Zellij runtime needed) with:

```bash
cargo test
```

## Adding it to your config

Load the plugin **once, in the background, for the whole session** via the
`load_plugins` block in `~/.config/zellij/config.kdl` — the same mechanism Zellij
uses for its own built-in `zellij:link` plugin:

```kdl
load_plugins {
    "file:/absolute/path/to/zellij_ai_tab_notify.wasm"
}
```

**Don't** add it as a pane inside a layout's `pane { plugin location=... }` block.
Zellij creates a fresh plugin instance for every tab a layout-embedded pane exists
in, and each instance only tracks the tab(s) it happens to receive events and pipe
signals for — so a tab marked by one instance may never be seen (and therefore
never cleared) by the instance living in that same tab. Loading it once via
`load_plugins` gives you a single instance that sees every tab and pane in the
session, which is what this plugin is designed around.

Adjust the path to wherever you built (or copied) the `.wasm` file.

### Permissions

On first load, Zellij will prompt you to grant the plugin two permissions:

- **Read application state** — to see the current tabs and panes.
- **Change application state** — to rename tabs.

Approve the prompt once; Zellij caches the grant for this plugin path so you won't
be asked again. If the pane hosting the prompt doesn't display anything (a known
Zellij rendering timing issue on first load), you can still grant it "blind": once
the plugin pane is loaded, focus it and press `y` — the keypress is honored even
when the prompt text itself didn't render.

### Configuration (optional)

Set a custom marker prefix (or distinct prefixes per signal kind) as a child block
of the plugin entry in `load_plugins`:

```kdl
load_plugins {
    "file:/absolute/path/to/zellij_ai_tab_notify.wasm" {
        prefix "🔔 "                 // default marker for both kinds if the below are unset
        finished_prefix "✅ "        // used for "finished" signals
        needs_input_prefix "❓ "     // used for "needs-input" signals
    }
}
```

If your terminal font doesn't render emoji well, use plain text instead, e.g.
`prefix "[!] "`.

## The pipe signal contract

An external process signals the plugin by sending a pipe message named
`claude-attention` with a JSON payload identifying the pane and the kind of signal:

```bash
zellij pipe --name claude-attention -- '{"pane_id": 12, "kind": "finished"}'
```

- `pane_id` — the Zellij pane id (an integer) whose tab should be flagged. You can
  get this from the `ZELLIJ_PANE_ID` environment variable inside that pane.
- `kind` — either `"finished"` (a task completed) or `"needs-input"` (blocked on a
  question/prompt). Unrecognized or malformed payloads are ignored.

If `pane_id` doesn't resolve to a pane the plugin currently knows about (e.g. it
was closed), or the signal targets the tab you're already focused on, the signal is
silently ignored — no error, no rename.

## Claude Code hook integration

A ready-to-use signaling script is included at
[`scripts/claude-attention-hook.sh`](scripts/claude-attention-hook.sh). It reads
`ZELLIJ_PANE_ID` from its environment and the hook's JSON from stdin, maps the
Claude Code hook event to a signal `kind` (`Stop` → `finished`, `Notification` →
`needs-input`), and calls `zellij pipe`. It exits `0` silently — never blocking or
failing the hook — when it's not running inside Zellij, when `zellij` isn't on
`PATH`, or for any other hook event.

Wire it up in your Claude Code settings (`~/.claude/settings.json` for all
projects, or `.claude/settings.json` for one project):

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/zellij-ai-tab-notify/scripts/claude-attention-hook.sh"
          }
        ]
      }
    ],
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/zellij-ai-tab-notify/scripts/claude-attention-hook.sh"
          }
        ]
      }
    ]
  }
}
```

With this in place, a Claude Code session running in a background Zellij pane will
flag its tab when it finishes responding or needs your input, and the tab reverts
to its normal name the moment you switch to it.

## Known limitation: per-tab, not per-pane

Zellij's tab bar has no way to indicate "this specific pane" needs attention —
only the tab as a whole. If a tab has multiple panes and more than one is running
a monitored session, focusing that tab clears the marker for the whole tab even if
only one of those sessions was the one that finished. This is a deliberate scope
limitation (see `design.md` in the originating change), not a bug: attention state
is tracked per tab, matching what Zellij's UI can actually represent.
