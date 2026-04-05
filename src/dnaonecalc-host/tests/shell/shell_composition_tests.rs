use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::shell_composition::{
    build_active_mode_projection, build_shell_frame_view_model, ActiveModeProjection,
};
use dnaonecalc_host::state::{AppMode, FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::test_support::sample_editor_document;

#[test]
fn ex_06_shell_composition_selects_explore_projection_for_active_formula_space() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    state.formula_spaces.insert(formula_space);

    let projection = build_active_mode_projection(&state).expect("projection should exist");
    match projection {
        ActiveModeProjection::Explore(view_model) => {
            assert_eq!(view_model.raw_entered_cell_text, "=LET(x,1,x)");
            assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
        }
        other => panic!("expected explore projection, got {other:?}"),
    }
}

#[test]
fn in_09_shell_composition_switches_to_inspect_projection_for_active_formula_space() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.active_mode = AppMode::Inspect;
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    state.formula_spaces.insert(formula_space);

    let projection = build_active_mode_projection(&state).expect("projection should exist");
    match projection {
        ActiveModeProjection::Inspect(view_model) => {
            assert_eq!(view_model.raw_entered_cell_text, "=LET(x,1,x)");
            assert_eq!(view_model.formula_walk_nodes.len(), 1);
        }
        other => panic!("expected inspect projection, got {other:?}"),
    }
}

#[test]
fn wb_01_shell_composition_switches_to_workbench_projection_for_active_formula_space() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.active_mode = AppMode::Workbench;
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let projection = build_active_mode_projection(&state).expect("projection should exist");
    match projection {
        ActiveModeProjection::Workbench(view_model) => {
            assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
        }
        other => panic!("expected workbench projection, got {other:?}"),
    }
}

#[test]
fn ex_11_shell_frame_view_marks_pinned_formula_space() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state
        .workspace_shell
        .pinned_formula_space_ids
        .insert(formula_space_id.clone());
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

    let frame = build_shell_frame_view_model(&state).expect("frame should exist");
    assert!(frame.formula_spaces[0].is_pinned);
}
