use std::collections::BTreeMap;

use serde::Deserialize;
use zellij_tile::prelude::PaneManifest;

pub const DEFAULT_PREFIX: &str = "\u{1F514} "; // "🔔 "

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Finished,
    NeedsInput,
}

impl SignalKind {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "finished" => Some(SignalKind::Finished),
            "needs-input" => Some(SignalKind::NeedsInput),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct AttentionPayload {
    pane_id: u32,
    kind: String,
}

/// Parses a `claude-attention` pipe payload. Returns `None` (rather than panicking) for
/// malformed JSON or an unrecognized `kind`.
pub fn parse_attention_signal(payload: &str) -> Option<(u32, SignalKind)> {
    let parsed: AttentionPayload = serde_json::from_str(payload).ok()?;
    let kind = SignalKind::parse(&parsed.kind)?;
    Some((parsed.pane_id, kind))
}

/// Rebuilds the pane_id -> tab_position lookup from scratch from the latest `PaneManifest`,
/// per design.md decision 4 (never incrementally patched).
///
/// Only terminal panes are included: `PaneInfo::id` is unique only within its own kind
/// (Zellij assigns terminal panes and plugin panes each their own separate `0, 1, 2…`
/// sequence), and the `pane_id` a hook script observes via `$ZELLIJ_PANE_ID` always refers
/// to a terminal pane. Including plugin panes here would let a plugin pane's id collide
/// with an unrelated terminal pane's id and misroute a signal to the wrong tab.
pub fn build_pane_to_tab_map(manifest: &PaneManifest) -> BTreeMap<u32, usize> {
    let mut map = BTreeMap::new();
    for (tab_position, panes) in manifest.panes.iter() {
        for pane in panes {
            if !pane.is_plugin {
                map.insert(pane.id, *tab_position);
            }
        }
    }
    map
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct TabState {
    pub original_name: String,
    pub marked: bool,
}

/// Decides whether a received signal should mark a tab, and if so, what the tab's new name
/// should be. Returns `None` when the tab is focused or already marked (no-op).
pub fn decide_mark(
    tab_position: usize,
    kind: SignalKind,
    focused_tab_position: Option<usize>,
    tab_states: &BTreeMap<usize, TabState>,
    tab_names: &BTreeMap<usize, String>,
    finished_prefix: &str,
    needs_input_prefix: &str,
) -> Option<(TabState, String)> {
    if focused_tab_position == Some(tab_position) {
        return None;
    }
    if tab_states
        .get(&tab_position)
        .map(|state| state.marked)
        .unwrap_or(false)
    {
        return None;
    }
    let original_name = tab_names.get(&tab_position).cloned().unwrap_or_default();
    let prefix = match kind {
        SignalKind::Finished => finished_prefix,
        SignalKind::NeedsInput => needs_input_prefix,
    };
    let new_name = format!("{}{}", prefix, original_name);
    Some((
        TabState {
            original_name,
            marked: true,
        },
        new_name,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zellij_tile::prelude::PaneInfo;

    fn pane(id: u32) -> PaneInfo {
        pane_of_kind(id, false)
    }

    fn pane_of_kind(id: u32, is_plugin: bool) -> PaneInfo {
        PaneInfo {
            id,
            is_plugin,
            is_focused: false,
            is_fullscreen: false,
            is_floating: false,
            is_suppressed: false,
            title: String::new(),
            exited: false,
            exit_status: None,
            is_held: false,
            pane_x: 0,
            pane_content_x: 0,
            pane_y: 0,
            pane_content_y: 0,
            pane_rows: 0,
            pane_content_rows: 0,
            pane_columns: 0,
            pane_content_columns: 0,
            cursor_coordinates_in_pane: None,
            terminal_command: None,
            plugin_url: None,
            is_selectable: true,
            index_in_pane_group: Default::default(),
            default_fg: None,
            default_bg: None,
        }
    }

    #[test]
    fn parses_finished_signal() {
        let (pane_id, kind) = parse_attention_signal(r#"{"pane_id": 7, "kind": "finished"}"#)
            .expect("valid payload should parse");
        assert_eq!(pane_id, 7);
        assert_eq!(kind, SignalKind::Finished);
    }

    #[test]
    fn parses_needs_input_signal() {
        let (pane_id, kind) = parse_attention_signal(r#"{"pane_id": 3, "kind": "needs-input"}"#)
            .expect("valid payload should parse");
        assert_eq!(pane_id, 3);
        assert_eq!(kind, SignalKind::NeedsInput);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_attention_signal("not json").is_none());
        assert!(parse_attention_signal("").is_none());
        assert!(parse_attention_signal(r#"{"pane_id": 7}"#).is_none());
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(parse_attention_signal(r#"{"pane_id": 7, "kind": "exploded"}"#).is_none());
    }

    #[test]
    fn maps_panes_to_their_tab_position() {
        let mut panes = HashMap::new();
        panes.insert(0, vec![pane(1), pane(2)]);
        panes.insert(1, vec![pane(3)]);
        let manifest = PaneManifest { panes };

        let map = build_pane_to_tab_map(&manifest);

        assert_eq!(map.get(&1), Some(&0));
        assert_eq!(map.get(&2), Some(&0));
        assert_eq!(map.get(&3), Some(&1));
    }

    #[test]
    fn ignores_plugin_panes_so_their_ids_cannot_collide_with_terminal_pane_ids() {
        // Zellij assigns ids separately per pane kind, so a plugin pane and a terminal
        // pane in different tabs can legitimately share the same numeric id. Only the
        // terminal pane (the one a hook script's $ZELLIJ_PANE_ID refers to) should end up
        // in the map, at its own correct tab position.
        let mut panes = HashMap::new();
        panes.insert(0, vec![pane_of_kind(1, true)]); // plugin pane id 1, tab 0
        panes.insert(1, vec![pane_of_kind(1, false)]); // terminal pane id 1, tab 1

        let map = build_pane_to_tab_map(&PaneManifest { panes });

        assert_eq!(map.get(&1), Some(&1));
    }

    #[test]
    fn map_reflects_panes_moved_between_tabs() {
        let mut before = HashMap::new();
        before.insert(0, vec![pane(1)]);
        before.insert(1, vec![pane(2)]);
        let before_map = build_pane_to_tab_map(&PaneManifest { panes: before });
        assert_eq!(before_map.get(&1), Some(&0));

        // Pane 1 moves from tab 0 to tab 1; the manifest (and therefore the map) is rebuilt
        // wholesale, not patched.
        let mut after = HashMap::new();
        after.insert(0, vec![]);
        after.insert(1, vec![pane(1), pane(2)]);
        let after_map = build_pane_to_tab_map(&PaneManifest { panes: after });

        assert_eq!(after_map.get(&1), Some(&1));
        assert_eq!(after_map.get(&2), Some(&1));
    }

    #[test]
    fn marks_a_background_tab_with_its_prefix() {
        let tab_states = BTreeMap::new();
        let mut tab_names = BTreeMap::new();
        tab_names.insert(1, "backend".to_string());

        let (state, new_name) = decide_mark(
            1,
            SignalKind::Finished,
            Some(0),
            &tab_states,
            &tab_names,
            "🔔 ",
            "❓ ",
        )
        .expect("background tab should be marked");

        assert_eq!(new_name, "🔔 backend");
        assert_eq!(state.original_name, "backend");
        assert!(state.marked);
    }

    #[test]
    fn does_not_mark_the_focused_tab() {
        let tab_states = BTreeMap::new();
        let tab_names = BTreeMap::new();

        let result = decide_mark(0, SignalKind::Finished, Some(0), &tab_states, &tab_names, "🔔 ", "❓ ");

        assert!(result.is_none());
    }

    #[test]
    fn does_not_re_mark_an_already_marked_tab() {
        let mut tab_states = BTreeMap::new();
        tab_states.insert(
            1,
            TabState {
                original_name: "backend".to_string(),
                marked: true,
            },
        );
        let tab_names = BTreeMap::new();

        let result = decide_mark(1, SignalKind::Finished, Some(0), &tab_states, &tab_names, "🔔 ", "❓ ");

        assert!(result.is_none());
    }

    #[test]
    fn uses_distinct_prefix_per_kind() {
        let tab_states = BTreeMap::new();
        let mut tab_names = BTreeMap::new();
        tab_names.insert(1, "backend".to_string());

        let (_, new_name) = decide_mark(
            1,
            SignalKind::NeedsInput,
            Some(0),
            &tab_states,
            &tab_names,
            "🔔 ",
            "❓ ",
        )
        .expect("background tab should be marked");

        assert_eq!(new_name, "❓ backend");
    }
}
