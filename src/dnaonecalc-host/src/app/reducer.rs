use crate::state::{CompletionHelpState, FormulaSpaceState, OneCalcHostState};
use crate::ui::editor::commands::{apply_editor_command, EditorCommand, EditorInputEvent};
use crate::ui::editor::state::EditorSurfaceState;

pub fn apply_editor_input_to_active_formula_space(
    state: &mut OneCalcHostState,
    input_event: EditorInputEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    let next_editor_state =
        if let (Some(selection_start), Some(selection_end)) =
            (input_event.selection_start, input_event.selection_end)
        {
            EditorSurfaceState::for_text_with_selection(
                &input_event.text,
                selection_start,
                selection_end,
            )
        } else {
            EditorSurfaceState::for_text(&input_event.text)
        };
    apply_local_editor_text_change(formula_space, input_event.text, next_editor_state);
    true
}

pub fn apply_editor_command_to_active_formula_space(
    state: &mut OneCalcHostState,
    command: EditorCommand,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    let result = apply_editor_command(
        &formula_space.raw_entered_cell_text,
        &formula_space.editor_surface_state,
        command,
    );
    apply_local_editor_text_change(formula_space, result.text, result.state);
    true
}

fn active_formula_space_mut(state: &mut OneCalcHostState) -> Option<&mut FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state.active_formula_space_view.selected_formula_space_id.clone())?;
    state.formula_spaces.get_mut(&formula_space_id)
}

fn apply_local_editor_text_change(
    formula_space: &mut FormulaSpaceState,
    text: String,
    editor_surface_state: EditorSurfaceState,
) {
    formula_space.raw_entered_cell_text = text;
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.editor_document = None;
    formula_space.completion_help = CompletionHelpState::default();
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::FormulaSpaceState;
    use crate::ui::editor::state::EditorSelection;

    #[test]
    fn input_event_updates_raw_text_and_editor_state_for_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: None,
                selection_end: None,
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(active.editor_surface_state.caret.offset, 11);
    }

    #[test]
    fn input_event_preserves_selection_metadata_when_provided() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2)".to_string(),
                selection_start: Some(2),
                selection_end: Some(5),
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.editor_surface_state.selection.anchor, 2);
        assert_eq!(active.editor_surface_state.selection.focus, 5);
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn command_updates_editor_state_and_clears_stale_analysis() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "SUM(\n1,\n2)");
        formula_space.editor_surface_state.selection = EditorSelection {
            anchor: 0,
            focus: formula_space.raw_entered_cell_text.chars().count(),
        };
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.effective_display_summary = Some("3".to_string());
        state.formula_spaces.insert(formula_space);

        let changed =
            apply_editor_command_to_active_formula_space(&mut state, EditorCommand::IndentWithSpaces);

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert!(active.raw_entered_cell_text.starts_with("    SUM("));
        assert!(active.latest_evaluation_summary.is_none());
        assert!(active.effective_display_summary.is_none());
    }
}
