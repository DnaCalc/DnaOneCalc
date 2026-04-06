use crate::adapters::oxfml::{EditorAnalysisStage, OxfmlEditorBridge, OxfmlEditorBridgeError};
use crate::app::intents::ApplyFormulaEditIntent;
use crate::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
};
use crate::domain::ids::FormulaSpaceId;
use crate::services::editor_session::{EditorSessionError, EditorSessionService};
use crate::state::{FormulaSpaceState, OneCalcHostState};
use crate::ui::editor::commands::{EditorCommand, EditorInputEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveEditError {
    NoActiveFormulaSpace,
    UnknownFormulaSpace(FormulaSpaceId),
    Session(EditorSessionError),
    Bridge(OxfmlEditorBridgeError),
}

pub fn apply_live_editor_input(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    input_event: EditorInputEvent,
) -> Result<bool, LiveEditError> {
    let formula_space_id =
        active_formula_space_id(state).ok_or(LiveEditError::NoActiveFormulaSpace)?;
    let changed = apply_editor_input_to_active_formula_space(state, input_event);
    if !changed {
        return Ok(false);
    }
    refresh_active_formula_space_from_bridge(bridge, state, &formula_space_id)?;
    Ok(true)
}

pub fn apply_live_editor_command(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    command: EditorCommand,
) -> Result<bool, LiveEditError> {
    let formula_space_id =
        active_formula_space_id(state).ok_or(LiveEditError::NoActiveFormulaSpace)?;
    let changed = apply_editor_command_to_active_formula_space(state, command.clone());
    if !changed {
        return Ok(false);
    }
    if matches!(
        command,
        EditorCommand::SelectPreviousCompletion
            | EditorCommand::SelectNextCompletion
            | EditorCommand::SelectCompletionByIndex(_)
    ) {
        return Ok(true);
    }
    refresh_active_formula_space_from_bridge(bridge, state, &formula_space_id)?;
    Ok(true)
}

fn active_formula_space_id(state: &OneCalcHostState) -> Option<FormulaSpaceId> {
    state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())
}

fn refresh_active_formula_space_from_bridge(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
) -> Result<(), LiveEditError> {
    let formula_space = state
        .formula_spaces
        .get(formula_space_id)
        .ok_or_else(|| LiveEditError::UnknownFormulaSpace(formula_space_id.clone()))?;
    let intent = build_live_edit_intent(formula_space);
    EditorSessionService::handle_formula_edit_intent(bridge, &mut state.formula_spaces, intent)
        .map_err(|error| match error {
            EditorSessionError::UnknownFormulaSpace(id) => LiveEditError::UnknownFormulaSpace(id),
            EditorSessionError::Bridge(bridge_error) => LiveEditError::Bridge(bridge_error),
        })
}

fn build_live_edit_intent(formula_space: &FormulaSpaceState) -> ApplyFormulaEditIntent {
    let formula_stable_id = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.formula_stable_id.clone())
        .unwrap_or_else(|| formula_space.formula_space_id.as_str().to_string());

    ApplyFormulaEditIntent {
        formula_space_id: formula_space.formula_space_id.clone(),
        formula_stable_id,
        entered_text: formula_space.raw_entered_cell_text.clone(),
        cursor_offset: formula_space.editor_surface_state.caret.offset,
        analysis_stage: EditorAnalysisStage::SyntaxAndBind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{FormulaEditRequest, FormulaEditResult, PreviewOxfmlBridge};
    use crate::state::FormulaSpaceState;
    use crate::test_support::sample_editor_document;
    use crate::ui::editor::commands::EditorInputKind;

    struct FakeBridge {
        document: crate::adapters::oxfml::EditorDocument,
    }

    impl OxfmlEditorBridge for FakeBridge {
        fn apply_formula_edit(
            &self,
            request: FormulaEditRequest,
        ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
            assert_eq!(request.analysis_stage, EditorAnalysisStage::SyntaxAndBind);
            Ok(FormulaEditResult {
                document: self.document.clone(),
            })
        }
    }

    #[test]
    fn live_input_refreshes_active_formula_space_through_bridge() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let bridge = FakeBridge {
            document: sample_editor_document("=SUM(1,2,3)"),
        };

        let changed = apply_live_editor_input(
            &bridge,
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: Some(11),
                selection_end: Some(11),
                input_kind: EditorInputKind::InsertText,
                inserted_text: Some("3".to_string()),
            },
        )
        .expect("live edit should refresh active formula space");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert!(active.editor_document.is_some());
        assert_eq!(active.completion_help.completion_count, 1);
    }

    #[test]
    fn live_command_refreshes_after_local_command_path() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let bridge = FakeBridge {
            document: sample_editor_document("=UM(1,2)"),
        };

        let changed = apply_live_editor_command(&bridge, &mut state, EditorCommand::Delete)
            .expect("live command should refresh active formula space");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=UM(1,2)");
        assert!(active.editor_document.is_some());
    }

    #[test]
    fn live_completion_navigation_stays_local_without_bridge_refresh() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.editor_surface_state.completion_selected_index = Some(0);
        state.formula_spaces.insert(formula_space);

        let bridge = FakeBridge {
            document: sample_editor_document("=SUM(1,2)"),
        };

        let changed =
            apply_live_editor_command(&bridge, &mut state, EditorCommand::SelectNextCompletion)
                .expect("completion navigation should stay local");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(
            active.editor_surface_state.completion_selected_index,
            Some(0)
        );
    }

    #[test]
    fn live_caret_movement_refreshes_signature_help_argument_index() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_surface_state.caret.offset = 6;
        formula_space.editor_surface_state.selection =
            crate::ui::editor::state::EditorSelection::collapsed(6);
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let changed = apply_live_editor_command(
            &PreviewOxfmlBridge,
            &mut state,
            EditorCommand::MoveCaretRight,
        )
        .expect("caret move should refresh live signature help");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(
            active
                .editor_document
                .as_ref()
                .and_then(|document| document.signature_help.as_ref())
                .map(|help| help.active_argument_index),
            Some(1)
        );
    }
}
