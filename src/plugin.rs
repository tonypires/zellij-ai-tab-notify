use std::collections::BTreeMap;

use zellij_tile::prelude::*;

use crate::logic::{
    build_pane_to_tab_map, decide_mark, parse_attention_signal, TabState, DEFAULT_PREFIX,
};

const PIPE_NAME: &str = "claude-attention";

#[derive(Default)]
pub struct State {
    pane_to_tab: BTreeMap<u32, usize>,
    tab_states: BTreeMap<usize, TabState>,
    tab_names: BTreeMap<usize, String>,
    focused_tab_position: Option<usize>,
    finished_prefix: String,
    needs_input_prefix: String,
}

/// `rename_tab`'s `tab_index` argument is 1-based (Zellij converts it back to a 0-based
/// position via `saturating_sub(1)`), while `TabInfo::position` — and every position we
/// track internally — is 0-based. Convert at the one call site that leaves our code.
fn rename_tab_at_position(position: usize, new_name: String) {
    rename_tab((position + 1) as u32, new_name);
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let default_prefix = configuration
            .get("prefix")
            .cloned()
            .unwrap_or_else(|| DEFAULT_PREFIX.to_string());
        self.finished_prefix = configuration
            .get("finished_prefix")
            .cloned()
            .unwrap_or_else(|| default_prefix.clone());
        self.needs_input_prefix = configuration
            .get("needs_input_prefix")
            .cloned()
            .unwrap_or(default_prefix);

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PaneUpdate(manifest) => {
                self.pane_to_tab = build_pane_to_tab_map(&manifest);
                false
            }
            Event::TabUpdate(tabs) => {
                let mut newly_focused_position = None;
                for tab in &tabs {
                    let is_marked = self
                        .tab_states
                        .get(&tab.position)
                        .map(|state| state.marked)
                        .unwrap_or(false);
                    // Only cache the name while unmarked, so a marked tab's decorated name
                    // never overwrites the snapshot we'll restore on focus.
                    if !is_marked {
                        self.tab_names.insert(tab.position, tab.name.clone());
                    }
                    if tab.active {
                        newly_focused_position = Some(tab.position);
                    }
                }

                if let Some(position) = newly_focused_position {
                    if let Some(state) = self.tab_states.get_mut(&position) {
                        if state.marked {
                            rename_tab_at_position(position, state.original_name.clone());
                            state.marked = false;
                        }
                    }
                    self.focused_tab_position = Some(position);
                }
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name != PIPE_NAME {
            return false;
        }
        let Some(payload) = pipe_message.payload else {
            return false;
        };
        let Some((pane_id, kind)) = parse_attention_signal(&payload) else {
            return false;
        };
        let Some(&tab_position) = self.pane_to_tab.get(&pane_id) else {
            return false;
        };

        if let Some((new_state, new_name)) = decide_mark(
            tab_position,
            kind,
            self.focused_tab_position,
            &self.tab_states,
            &self.tab_names,
            &self.finished_prefix,
            &self.needs_input_prefix,
        ) {
            rename_tab_at_position(tab_position, new_name);
            self.tab_states.insert(tab_position, new_state);
        }
        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {}
}

register_plugin!(State);

// `register_plugin!` defines a private, unmangled-but-unexported `main()` that only sets
// up the panic hook; it relies on the crate being compiled as a WASI "command" binary
// (which auto-generates `_start` calling `main`). Since this crate is a `cdylib`, not a
// `bin`, no such entry point is generated automatically, but Zellij's plugin loader
// requires a callable `_start` export before it will call `load`. Provide it directly.
#[no_mangle]
pub extern "C" fn _start() {
    std::panic::set_hook(Box::new(report_panic));
}
