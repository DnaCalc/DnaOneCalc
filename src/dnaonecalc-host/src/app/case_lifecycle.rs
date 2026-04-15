//! Case / formula-space lifecycle reducer functions.
//!
//! Covers the shell-level actions the rail surfaces: create a fresh formula
//! space, rename it, duplicate it, close it, toggle pinned status. Every
//! function is a pure mutation on `OneCalcHostState` so `OneCalcShellApp` can
//! wire them to callbacks without threading additional services.

use crate::domain::ids::FormulaSpaceId;
use crate::state::{AppMode, ClosedFormulaSpaceRecord, FormulaSpaceState, OneCalcHostState};

const MAX_RECENT_FORMULA_SPACES: usize = 8;

/// Create a fresh empty formula space, insert it into the workspace, and
/// activate it. Returns the generated id so the caller can show toast or
/// focus-related UI.
pub fn new_formula_space(state: &mut OneCalcHostState) -> FormulaSpaceId {
    let next_index = next_untitled_index(state);
    let id_string = format!("untitled-{next_index}");
    let formula_space_id = FormulaSpaceId::new(id_string);
    let label = format!("Untitled {next_index}");

    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "");
    formula_space.context.scenario_label = label;
    state.formula_spaces.insert(formula_space);
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .formula_space_modes
        .insert(formula_space_id.clone(), AppMode::Explore);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(formula_space_id.clone());
    formula_space_id
}

fn next_untitled_index(state: &OneCalcHostState) -> usize {
    let mut max_index = 0usize;
    for id in &state.workspace_shell.open_formula_space_order {
        if let Some(rest) = id.as_str().strip_prefix("untitled-") {
            if let Ok(index) = rest.parse::<usize>() {
                if index > max_index {
                    max_index = index;
                }
            }
        }
    }
    max_index + 1
}

pub fn rename_formula_space(
    state: &mut OneCalcHostState,
    formula_space_id: &str,
    next_label: impl Into<String>,
) -> bool {
    let next_label = next_label.into();
    if next_label.trim().is_empty() {
        return false;
    }
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    match state.formula_spaces.get_mut(&id) {
        Some(formula_space) => {
            formula_space.context.scenario_label = next_label;
            true
        }
        None => false,
    }
}

pub fn duplicate_formula_space(
    state: &mut OneCalcHostState,
    formula_space_id: &str,
) -> Option<FormulaSpaceId> {
    let source_id = FormulaSpaceId::new(formula_space_id.to_string());
    let source = state.formula_spaces.get(&source_id)?.clone();
    let next_index = next_untitled_index(state);
    let new_id = FormulaSpaceId::new(format!("copy-{next_index}-of-{formula_space_id}"));

    let mut duplicate =
        FormulaSpaceState::new(new_id.clone(), source.raw_entered_cell_text.clone());
    duplicate.context = source.context.clone();
    duplicate.context.scenario_label = format!("{} (copy)", source.context.scenario_label);
    duplicate.committed_cell_text = source.committed_cell_text.clone();
    duplicate.proofed_cell_text = source.proofed_cell_text.clone();
    duplicate.expanded_editor = source.expanded_editor;

    state.formula_spaces.insert(duplicate);
    state
        .workspace_shell
        .open_formula_space_order
        .push(new_id.clone());
    state.workspace_shell.active_formula_space_id = Some(new_id.clone());
    let duplicated_mode = state
        .workspace_shell
        .formula_space_modes
        .get(&source_id)
        .copied()
        .unwrap_or(AppMode::Explore);
    state
        .workspace_shell
        .formula_space_modes
        .insert(new_id.clone(), duplicated_mode);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(new_id.clone());
    Some(new_id)
}

pub fn close_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    let Some(closed_formula_space) = state.formula_spaces.spaces.remove(&id) else {
        return false;
    };
    state
        .workspace_shell
        .open_formula_space_order
        .retain(|candidate| candidate != &id);
    state.workspace_shell.pinned_formula_space_ids.remove(&id);
    let last_active_mode = state
        .workspace_shell
        .formula_space_modes
        .remove(&id)
        .unwrap_or(AppMode::Explore);
    remember_recent_formula_space(state, closed_formula_space, last_active_mode);

    let was_active = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .map(|active| active == &id)
        .unwrap_or(false);
    if was_active {
        state.workspace_shell.active_formula_space_id = state
            .workspace_shell
            .open_formula_space_order
            .first()
            .cloned();
        if let Some(next_active_formula_space_id) =
            state.workspace_shell.active_formula_space_id.as_ref()
        {
            state.active_formula_space_view.active_mode = state
                .workspace_shell
                .formula_space_modes
                .get(next_active_formula_space_id)
                .copied()
                .unwrap_or(AppMode::Explore);
            state.active_formula_space_view.selected_formula_space_id =
                Some(next_active_formula_space_id.clone());
        }
    }

    // Keep the workspace from ever being empty: if closing the last space
    // leaves nothing open, spin a fresh Untitled so the editor still has a
    // surface to mount against.
    if state.workspace_shell.open_formula_space_order.is_empty() {
        let _ = new_formula_space(state);
    }
    true
}

pub fn reopen_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    if state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .any(|candidate| candidate == &id)
    {
        state.workspace_shell.active_formula_space_id = Some(id.clone());
        state.active_formula_space_view.selected_formula_space_id = Some(id.clone());
        state.active_formula_space_view.active_mode = state
            .workspace_shell
            .formula_space_modes
            .get(&id)
            .copied()
            .unwrap_or(AppMode::Explore);
        state.workspace_shell.navigation_selection =
            crate::state::WorkspaceNavigationSelection::FormulaSpace(id);
        return true;
    }

    let Some(record) = state.workspace_shell.recent_formula_spaces.remove(&id) else {
        return false;
    };
    state
        .workspace_shell
        .recent_formula_space_order
        .retain(|candidate| candidate != &id);
    state.formula_spaces.insert(record.formula_space);
    state
        .workspace_shell
        .open_formula_space_order
        .push(id.clone());
    state.workspace_shell.active_formula_space_id = Some(id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(id.clone());
    state.active_formula_space_view.active_mode = record.last_active_mode;
    state
        .workspace_shell
        .formula_space_modes
        .insert(id.clone(), record.last_active_mode);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(id);
    true
}

pub fn toggle_pin_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    if state.formula_spaces.get(&id).is_none() {
        return false;
    }
    if state.workspace_shell.pinned_formula_space_ids.contains(&id) {
        state.workspace_shell.pinned_formula_space_ids.remove(&id);
    } else {
        state.workspace_shell.pinned_formula_space_ids.insert(id);
    }
    true
}

fn remember_recent_formula_space(
    state: &mut OneCalcHostState,
    formula_space: FormulaSpaceState,
    last_active_mode: AppMode,
) {
    let id = formula_space.formula_space_id.clone();
    state
        .workspace_shell
        .recent_formula_space_order
        .retain(|candidate| candidate != &id);
    state.workspace_shell.recent_formula_spaces.insert(
        id.clone(),
        ClosedFormulaSpaceRecord {
            formula_space,
            last_active_mode,
        },
    );
    state
        .workspace_shell
        .recent_formula_space_order
        .insert(0, id);
    while state.workspace_shell.recent_formula_space_order.len() > MAX_RECENT_FORMULA_SPACES {
        if let Some(removed_id) = state.workspace_shell.recent_formula_space_order.pop() {
            state
                .workspace_shell
                .recent_formula_spaces
                .remove(&removed_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state_with_space(id: &str) -> OneCalcHostState {
        let mut state = OneCalcHostState::default();
        let formula_space_id = FormulaSpaceId::new(id.to_string());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=1"));
        state
    }

    #[test]
    fn new_formula_space_inserts_untitled_and_activates_it() {
        let mut state = OneCalcHostState::default();
        let id = new_formula_space(&mut state);
        assert_eq!(id.as_str(), "untitled-1");
        assert_eq!(state.formula_spaces.spaces.len(), 1);
        assert_eq!(
            state.workspace_shell.active_formula_space_id,
            Some(id.clone())
        );
        assert_eq!(state.workspace_shell.open_formula_space_order, vec![id]);
        assert_eq!(
            state
                .workspace_shell
                .formula_space_modes
                .get(&FormulaSpaceId::new("untitled-1")),
            Some(&AppMode::Explore)
        );
    }

    #[test]
    fn new_formula_space_uses_incrementing_index() {
        let mut state = OneCalcHostState::default();
        let first = new_formula_space(&mut state);
        let second = new_formula_space(&mut state);
        assert_eq!(first.as_str(), "untitled-1");
        assert_eq!(second.as_str(), "untitled-2");
    }

    #[test]
    fn rename_updates_scenario_label_when_id_matches() {
        let mut state = fresh_state_with_space("space-1");
        assert!(rename_formula_space(&mut state, "space-1", "Renamed"));
        assert_eq!(
            state
                .formula_spaces
                .get(&FormulaSpaceId::new("space-1".to_string()))
                .unwrap()
                .context
                .scenario_label,
            "Renamed"
        );
    }

    #[test]
    fn rename_rejects_empty_label() {
        let mut state = fresh_state_with_space("space-1");
        assert!(!rename_formula_space(&mut state, "space-1", "   "));
    }

    #[test]
    fn duplicate_creates_new_space_with_suffix_label() {
        let mut state = fresh_state_with_space("space-1");
        let new_id = duplicate_formula_space(&mut state, "space-1").expect("duplicated");
        assert!(new_id.as_str().contains("copy-"));
        let duplicate = state.formula_spaces.get(&new_id).unwrap();
        assert!(duplicate.context.scenario_label.ends_with("(copy)"));
        assert_eq!(duplicate.raw_entered_cell_text, "=1");
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&new_id)
        );
    }

    #[test]
    fn close_removes_space_and_activates_another_when_present() {
        let mut state = fresh_state_with_space("space-1");
        state
            .workspace_shell
            .open_formula_space_order
            .push(FormulaSpaceId::new("space-2".to_string()));
        state.formula_spaces.insert(FormulaSpaceState::new(
            FormulaSpaceId::new("space-2".to_string()),
            "=2",
        ));

        assert!(close_formula_space(&mut state, "space-1"));
        assert!(state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .is_none());
        assert_eq!(
            state.workspace_shell.active_formula_space_id,
            Some(FormulaSpaceId::new("space-2".to_string()))
        );
    }

    #[test]
    fn close_last_space_creates_a_fresh_untitled() {
        let mut state = fresh_state_with_space("space-1");
        close_formula_space(&mut state, "space-1");
        assert_eq!(state.formula_spaces.spaces.len(), 1);
        assert_eq!(state.workspace_shell.recent_formula_space_order.len(), 1);
        let active_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .unwrap();
        assert!(active_id.as_str().starts_with("untitled-"));
    }

    #[test]
    fn close_archives_formula_space_for_reopen() {
        let mut state = fresh_state_with_space("space-1");
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        state
            .workspace_shell
            .formula_space_modes
            .insert(FormulaSpaceId::new("space-1".to_string()), AppMode::Inspect);

        assert!(close_formula_space(&mut state, "space-1"));

        let archived = state
            .workspace_shell
            .recent_formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .expect("archived formula space");
        assert_eq!(archived.formula_space.raw_entered_cell_text, "=1");
        assert_eq!(archived.last_active_mode, AppMode::Inspect);
    }

    #[test]
    fn reopen_restores_formula_space_and_mode() {
        let mut state = fresh_state_with_space("space-1");
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        state.workspace_shell.formula_space_modes.insert(
            FormulaSpaceId::new("space-1".to_string()),
            AppMode::Workbench,
        );
        assert!(close_formula_space(&mut state, "space-1"));

        assert!(reopen_formula_space(&mut state, "space-1"));
        assert!(state
            .workspace_shell
            .recent_formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .is_none());
        assert!(state
            .workspace_shell
            .open_formula_space_order
            .contains(&FormulaSpaceId::new("space-1".to_string())));
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Workbench
        );
    }

    #[test]
    fn toggle_pin_flips_membership() {
        let mut state = fresh_state_with_space("space-1");
        toggle_pin_formula_space(&mut state, "space-1");
        assert!(state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
        toggle_pin_formula_space(&mut state, "space-1");
        assert!(!state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
    }
}
