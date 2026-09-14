# Zellij AI Tab Notify

```
 ┌─────────┐ ┌─────────┐ ┌─────────────┐ ┌─────────┐
 │  editor │ │  build  │ │ 🔔 ai-agent │ │  logs   │
 └─────────┘ └─────────┘ └─────────────┘ └─────────┘
                                ▲
                          needs attention
```

A [Zellij](https://zellij.dev) plugin that updates a tab to flag it as
"needs attention" when a process running in one of its panes - in particular a
[Claude Code](https://claude.com/claude-code) session - finishes responding or is
waiting on input. The marker clears automatically the moment you focus that tab
again, and your original tab name is restored exactly.

The plugin never reads pane content. It only reacts to explicit signals sent over
Zellij's `zellij pipe` mechanism, from a Claude Code hook.

https://github.com/user-attachments/assets/829d34a4-d5a2-46da-9867-3df2808d6b9f

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

   Optionally, add a short notification sound alongside the tab marker by
   pointing `finished_sound`/`needs_input_sound` at the bundled `assets/*.wav`
   files (or your own):

   ```kdl
   load_plugins {
       "file:/absolute/path/to/zellij_ai_tab_notify.wasm" {
           finished_sound "/absolute/path/to/zellij-ai-tab-notify/assets/done.wav"
           needs_input_sound "/absolute/path/to/zellij-ai-tab-notify/assets/needs-input.wav"
       }
   }
   ```

   Each sound is independent — set only one, or both, or neither. Set
   `sound "off"` in the same block to mute sound entirely without removing
   the configured paths.

3. Restart (or reload) Zellij and approve the three permission prompts on
   first load: read application state, change application state, and run
   commands (used to shell out to a host audio player when a sound is
   configured — requested unconditionally, even if you don't configure
   sound). If the prompt pane doesn't render, focus it and press `y` anyway.

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

### Sound playback

When a sound is configured (see step 2), the plugin plays it by trying
`afplay` (macOS), then `paplay`, then `aplay` (both Linux/PulseAudio and
ALSA) — whichever is available on your host. If none of these are installed,
or the configured path doesn't exist, playback silently fails and the tab is
still marked as usual; nothing else is affected.

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
