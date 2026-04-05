use crate::state::{CompletionHelpState, FormulaSpaceState, OneCalcHostState};
use crate::services::retained_artifacts::{
    import_manual_artifact_for_active_formula_space, open_retained_artifact_by_id,
    open_retained_artifact_in_inspect_by_id,
    ManualRetainedArtifactImportRequest,
};
use crate::ui::editor::commands::{
    apply_editor_command, cycle_completion_selection, EditorCommand, EditorInputEvent,
};
use crate::ui::editor::geometry::EditorOverlayMeasurementEvent;
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

    if apply_completion_command(formula_space, &command) {
        return true;
    }

    let result = apply_editor_command(
        &formula_space.raw_entered_cell_text,
        &formula_space.editor_surface_state,
        command,
    );
    apply_local_editor_text_change(formula_space, result.text, result.state);
    true
}

pub fn apply_editor_overlay_measurement_to_active_formula_space(
    state: &mut OneCalcHostState,
    measurement_event: EditorOverlayMeasurementEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    formula_space.editor_overlay_geometry = Some(measurement_event.snapshot);
    true
}

pub fn open_retained_artifact_from_catalog(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_by_id(state, artifact_id).is_ok()
}

pub fn open_retained_artifact_from_catalog_in_inspect(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_in_inspect_by_id(state, artifact_id).is_ok()
}

pub fn import_manual_retained_artifact_into_active_formula_space(
    state: &mut OneCalcHostState,
    request: ManualRetainedArtifactImportRequest,
) -> bool {
    import_manual_artifact_for_active_formula_space(state, request).is_ok()
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
    formula_space.editor_overlay_geometry = None;
    formula_space.editor_document = None;
    formula_space.completion_help = CompletionHelpState::default();
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;
}

fn apply_completion_command(formula_space: &mut FormulaSpaceState, command: &EditorCommand) -> bool {
    match command {
        EditorCommand::SelectPreviousCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index = cycle_completion_selection(
                formula_space.editor_surface_state.completion_selected_index,
                proposal_count,
                -1,
            );
            true
        }
        EditorCommand::SelectNextCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index = cycle_completion_selection(
                formula_space.editor_surface_state.completion_selected_index,
                proposal_count,
                1,
            );
            true
        }
        EditorCommand::SelectCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            true
        }
        EditorCommand::AcceptSelectedCompletion => {
            let Some(document) = formula_space.editor_document.as_ref() else {
                return false;
            };
            let proposal_count = document.completion_proposals.len();
            if proposal_count == 0 {
                return false;
            }
            let selected_index = formula_space
                .editor_surface_state
                .completion_selected_index
                .unwrap_or(0)
                .min(proposal_count.saturating_sub(1));
            let proposal = &document.completion_proposals[selected_index];
            let (selection_start, selection_end) = proposal
                .replacement_span
                .map(|span| (span.start, span.start + span.len))
                .unwrap_or((
                    formula_space.editor_surface_state.selection.start(),
                    formula_space.editor_surface_state.selection.end(),
                ));
            let replacement_state = EditorSurfaceState {
                selection: crate::ui::editor::state::EditorSelection {
                    anchor: selection_start,
                    focus: selection_end,
                },
                caret: crate::ui::editor::state::EditorCaret { offset: selection_end },
                scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                completion_anchor_offset: formula_space.editor_surface_state.completion_anchor_offset,
                completion_selected_index: formula_space.editor_surface_state.completion_selected_index,
                signature_help_anchor_offset: formula_space.editor_surface_state.signature_help_anchor_offset,
            };
            let result = apply_editor_command(
                &formula_space.raw_entered_cell_text,
                &replacement_state,
                EditorCommand::InsertText(proposal.insert_text.clone()),
            );
            apply_local_editor_text_change(formula_space, result.text, result.state);
            true
        }
        EditorCommand::AcceptCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            apply_completion_command(formula_space, &EditorCommand::AcceptSelectedCompletion)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FormulaSpaceState;
    use crate::ui::editor::commands::EditorInputKind;
    use crate::ui::editor::geometry::{
        EditorMeasuredOverlayBox, EditorOverlayGeometrySnapshot, EditorOverlayMeasurementEvent,
    };
    use crate::ui::editor::state::EditorSelection;
    use crate::{
        domain::ids::FormulaSpaceId,
        services::{
            programmatic_testing::{
                ProgrammaticArtifactCatalogEntry, ProgrammaticComparisonStatus,
                ProgrammaticOpenModeHint,
            },
            retained_artifacts::RetainedArtifactImportRequest,
        },
    };

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
                input_kind: EditorInputKind::Other,
                inserted_text: None,
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
                input_kind: EditorInputKind::InsertText,
                inserted_text: Some("M".to_string()),
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

    #[test]
    fn overlay_measurement_event_updates_geometry_on_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_overlay_measurement_to_active_formula_space(
            &mut state,
            EditorOverlayMeasurementEvent {
                snapshot: EditorOverlayGeometrySnapshot {
                    caret_box: Some(EditorMeasuredOverlayBox {
                        top_px: 40,
                        left_px: 64,
                        width_px: 2,
                        height_px: 22,
                        line_index: 0,
                        column_index: 4,
                    }),
                    selection_box: None,
                    completion_anchor_box: None,
                    signature_help_anchor_box: None,
                    completion_popup_box: None,
                    signature_help_popup_box: None,
                },
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(
            active
                .editor_overlay_geometry
                .as_ref()
                .and_then(|geometry| geometry.caret_box.as_ref())
                .map(|box_geometry| box_geometry.left_px),
            Some(64)
        );
    }

    #[test]
    fn open_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)"));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=3 excel=4".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog(&mut state, "artifact-1");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-1")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn importing_manual_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let imported = import_manual_retained_artifact_into_active_formula_space(
            &mut state,
            crate::services::retained_artifacts::ManualRetainedArtifactImportRequest {
                artifact_id: "artifact-2".to_string(),
                case_id: "case-2".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Blocked,
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        assert!(imported);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-2")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn open_retained_artifact_in_inspect_routes_shell_to_inspect_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)"));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-inspect".to_string(),
                    case_id: "case-inspect".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Blocked,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog_in_inspect(&mut state, "artifact-inspect");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-inspect")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Inspect
        );
    }
}
