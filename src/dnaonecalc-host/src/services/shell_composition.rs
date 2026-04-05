use crate::services::explore_mode::{build_explore_view_model, ExploreViewModel};
use crate::services::inspect_mode::{build_inspect_view_model, InspectViewModel};
use crate::state::{AppMode, FormulaSpaceState, OneCalcHostState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModeProjection {
    Explore(ExploreViewModel),
    Inspect(InspectViewModel),
    WorkbenchStub,
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
        AppMode::Workbench => Some(ActiveModeProjection::WorkbenchStub),
    }
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
}
