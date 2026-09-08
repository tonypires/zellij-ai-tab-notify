# Release Notes — zellij-ai-tab-notify

## Run & Test Locally

```bash
rustup target add wasm32-wasip1
cd zellij-ai-tab-notify
cargo build --release --target wasm32-wasip1
cargo test
```
Produces `target/wasm32-wasip1/release/zellij_ai_tab_notify.wasm`.

## Install / Deploy

Add to `~/.config/zellij/config.kdl`:

```kdl
load_plugins {
    "file:/absolute/path/to/zellij_ai_tab_notify.wasm"
}
```

Load it once via `load_plugins` (not as a layout pane) — approve the two
permission prompts (read/change application state) on first load.

Wire up the Claude Code hook in `~/.claude/settings.json` (or
`.claude/settings.json`):

```json
{
  "hooks": {
    "Stop": [{ "hooks": [{ "type": "command", "command": "/absolute/path/to/zellij-ai-tab-notify/scripts/claude-attention-hook.sh" }] }],
    "Notification": [{ "hooks": [{ "type": "command", "command": "/absolute/path/to/zellij-ai-tab-notify/scripts/claude-attention-hook.sh" }] }]
  }
}
```

## Distribute

Share this repo (or copy the `zellij-ai-tab-notify/` directory) plus the
built `.wasm` file — the recipient runs the build step above, points
`load_plugins` at their own copy of the `.wasm`, and wires up the hook script
with their own absolute path.
