# zellij-ai-tab-notify

```
 ┌─────────┐ ┌─────────┐ ┌─────────────┐ ┌─────────┐
 │  editor │ │  build  │ │ 🔔 ai-agent │ │  logs   │
 └─────────┘ └─────────┘ └─────────────┘ └─────────┘
                                ▲
                          needs attention
```



https://github.com/user-attachments/assets/4cad2063-399e-4e5f-8155-c3199e8a30a9

A [Zellij](https://zellij.dev) plugin that renames a background tab to flag it as
"needs attention" when a process running in one of its panes — in particular a
[Claude Code](https://claude.com/claude-code) session — finishes responding or is
waiting on input. The marker clears automatically the moment you focus that tab
again, and your original tab name is restored exactly.

The plugin never reads pane content. It only reacts to explicit signals sent over
Zellij's `zellij pipe` mechanism, typically from a Claude Code hook.

## Supported coding harnesses

| Harness     | Supported |
| ----------- | --------- |
| Claude Code | Yes       |
| Codex       | TBD       |
| Gemini CLI  | TBD       |
| OpenCode    | TBD       |

## Installing

1. Add the `wasm32-wasip1` target and build the plugin:

   ```bash
   rustup target add wasm32-wasip1
   cd zellij-ai-tab-notify
   cargo build --release --target wasm32-wasip1
   ```

   This produces `target/wasm32-wasip1/release/zellij_ai_tab_notify.wasm`.

2. Load it **once, in the background, for the whole session** via the
   `load_plugins` block in `~/.config/zellij/config.kdl`:

   ```kdl
   load_plugins {
       "file:/absolute/path/to/zellij_ai_tab_notify.wasm"
   }
   ```

   **Don't** add it as a pane inside a layout's `pane { plugin location=... }`
   block — that creates a separate instance per tab, which breaks the tracking
   this plugin relies on.

3. Restart (or reload) Zellij and approve the two permission prompts (read and
   change application state) on first load. If the prompt pane doesn't render,
   focus it and press `y` anyway.

4. Wire up Claude Code to send this plugin its signals, by pointing the `Stop`
   and `Notification` hooks in `~/.claude/settings.json` at the included
   `scripts/claude-attention-hook.sh`:

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

### What the hook script does

`scripts/claude-attention-hook.sh` is the bridge between Claude Code and the
plugin. Claude Code invokes it automatically for the hook events above, passing
the hook's JSON on stdin; the script reads the `ZELLIJ_PANE_ID` environment
variable (set for any pane running inside Zellij), maps the event to a signal —
`Stop` → `finished`, `Notification` → `needs-input` — and forwards it to the
plugin via `zellij pipe`. It exits silently (doing nothing) when not running
inside Zellij or for any other hook event, so it's safe to leave wired up even
outside a Zellij session.

You don't need to run it yourself, but you can invoke it by hand (e.g. to test
your setup) from inside a Zellij pane:

```bash
echo '{"hook_event_name": "Stop"}' | ./scripts/claude-attention-hook.sh
```
