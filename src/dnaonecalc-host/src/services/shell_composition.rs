use crate::services::explore_mode::{build_explore_view_model, ExploreViewModel};
use crate::services::inspect_mode::{build_inspect_view_model, InspectViewModel};
use crate::services::workbench_mode::{build_workbench_view_model, WorkbenchViewModel};
use crate::state::{AppMode, FormulaSpaceState, OneCalcHostState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModeProjection {
    Explore(ExploreViewModel),
    Inspect(InspectViewModel),
    Workbench(WorkbenchViewModel),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellModeTabViewModel {
    pub mode: AppMode,
    pub label: &'static str,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFormulaSpaceListItemViewModel {
    pub formula_space_id: String,
    pub label: String,
    pub is_active: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFrameViewModel {
    pub active_formula_space_label: String,
    pub mode_tabs: Vec<ShellModeTabViewModel>,
    pub formula_spaces: Vec<ShellFormulaSpaceListItemViewModel>,
}

pub fn active_formula_space(state: &OneCalcHostState) -> Option<&FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .or(state.active_formula_space_view.selected_formula_space_id.as_ref())?;
    state.formula_spaces.get(formula_space_id)
}

pub fn build_active_mode_projection(state: &OneCalcHostState) -> Option<ActiveModeProjection> {
    let formula_space = active_formula_space(state)?;
    match state.active_formula_space_view.active_mode {
        AppMode::Explore => Some(ActiveModeProjection::Explore(build_explore_view_model(
            formula_space,
        ))),
        AppMode::Inspect => Some(ActiveModeProjection::Inspect(build_inspect_view_model(
            formula_space,
        ))),
        AppMode::Workbench => Some(ActiveModeProjection::Workbench(build_workbench_view_model(
            formula_space,
        ))),
    }
}

pub fn build_shell_frame_view_model(state: &OneCalcHostState) -> Option<ShellFrameViewModel> {
    let active_formula_space = active_formula_space(state)?;
    let active_formula_space_id = &active_formula_space.formula_space_id;

    let mode_tabs = [AppMode::Explore, AppMode::Inspect, AppMode::Workbench]
        .into_iter()
        .map(|mode| ShellModeTabViewModel {
            mode,
            label: match mode {
                AppMode::Explore => "Explore",
                AppMode::Inspect => "Inspect",
                AppMode::Workbench => "Workbench",
            },
            is_active: state.active_formula_space_view.active_mode == mode,
        })
        .collect();

    let formula_spaces = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .filter_map(|formula_space_id| {
            state.formula_spaces.get(formula_space_id).map(|formula_space| {
                ShellFormulaSpaceListItemViewModel {
                    formula_space_id: formula_space.formula_space_id.as_str().to_string(),
                    label: formula_space.formula_space_id.as_str().to_string(),
                    is_active: &formula_space.formula_space_id == active_formula_space_id,
                    is_pinned: state
                        .workspace_shell
                        .pinned_formula_space_ids
                        .contains(&formula_space.formula_space_id),
                }
            })
        })
        .collect();

    Some(ShellFrameViewModel {
        active_formula_space_label: active_formula_space_id.as_str().to_string(),
        mode_tabs,
        formula_spaces,
    })
}

pub fn switch_active_mode(state: &mut OneCalcHostState, next_mode: AppMode) {
    state.active_formula_space_view.active_mode = next_mode;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{FormulaSpaceCollectionState, FormulaSpaceState, OneCalcHostState};
    use crate::test_support::sample_editor_document;

    #[test]
    fn active_formula_space_prefers_workspace_shell_selection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let active = active_formula_space(&state).expect("active space should exist");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
    }

    #[test]
    fn build_active_mode_projection_routes_to_explore_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Explore(view_model) => {
                assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
            }
            other => panic!("expected explore projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_to_inspect_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Inspect(view_model) => {
                assert_eq!(view_model.formula_walk_nodes.len(), 1);
            }
            other => panic!("expected inspect projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_returns_none_without_active_space() {
        let state = OneCalcHostState {
            formula_spaces: FormulaSpaceCollectionState::default(),
            ..Default::default()
        };

        assert!(build_active_mode_projection(&state).is_none());
    }

    #[test]
    fn shell_frame_view_model_tracks_active_space_and_mode_tabs() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let frame = build_shell_frame_view_model(&state).expect("frame should exist");
        assert_eq!(frame.active_formula_space_label, "space-1");
        assert_eq!(frame.formula_spaces.len(), 1);
        assert!(frame.formula_spaces[0].is_active);
        assert!(!frame.formula_spaces[0].is_pinned);
        assert!(frame.mode_tabs.iter().any(|tab| tab.mode == AppMode::Inspect && tab.is_active));
    }

    #[test]
    fn switch_active_mode_updates_state() {
        let mut state = OneCalcHostState::default();
        assert_eq!(state.active_formula_space_view.active_mode, AppMode::Explore);

        switch_active_mode(&mut state, AppMode::Inspect);

        assert_eq!(state.active_formula_space_view.active_mode, AppMode::Inspect);
    }

    #[test]
    fn build_active_mode_projection_routes_to_workbench_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }
}
