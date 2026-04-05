use dnaonecalc_host::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::{FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::ui::editor::commands::{EditorCommand, EditorInputEvent};

#[test]
fn ex_15_editor_input_event_updates_active_formula_space_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)"));

    let changed = apply_editor_input_to_active_formula_space(
        &mut state,
        EditorInputEvent {
            text: "=SUM(1,2,3)".to_string(),
        },
    );

    assert!(changed);
    let active = state.formula_spaces.get(&formula_space_id).expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
    assert_eq!(active.editor_surface_state.caret.offset, 11);
    assert!(active.editor_document.is_none());
}

#[test]
fn ex_16_editor_command_updates_caret_and_selection_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)"));

    let changed = apply_editor_command_to_active_formula_space(&mut state, EditorCommand::MoveCaretLeft);

    assert!(changed);
    let active = state.formula_spaces.get(&formula_space_id).expect("space exists");
    assert_eq!(active.editor_surface_state.caret.offset, 8);
    assert!(active.editor_surface_state.selection.is_collapsed());
}
