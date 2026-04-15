use crate::services::retained_artifacts::{
    import_manual_artifact_for_active_formula_space, import_verification_bundle_report_json,
    open_retained_artifact_by_id, open_retained_artifact_in_inspect_by_id,
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use crate::state::{CompletionHelpState, FormulaSpaceState, OneCalcHostState};
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

    let next_editor_state = if let (Some(selection_start), Some(selection_end)) =
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
    match &command {
        EditorCommand::ToggleEditorSettingsPopover => {
            return toggle_editor_settings_popover(state);
        }
        EditorCommand::UpdateEditorSetting(update) => {
            return update_editor_setting(state, *update);
        }
        EditorCommand::ToggleConfigureDrawer => {
            return toggle_configure_drawer(state);
        }
        _ => {}
    }

    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    if apply_completion_command(formula_space, &command) {
        return true;
    }

    if apply_live_state_command(formula_space, &command) {
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

fn apply_live_state_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
    // UI chrome commands are handled at the host level; delegate them back up.
    if matches!(
        command,
        EditorCommand::ToggleEditorSettingsPopover
            | EditorCommand::UpdateEditorSetting(_)
            | EditorCommand::ToggleConfigureDrawer
    ) {
        return false;
    }
    match command {
        EditorCommand::CommitEntry => {
            formula_space.committed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::RequestProof => {
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::CancelEntry => {
            if let Some(committed) = formula_space.committed_cell_text.clone() {
                let restored_state = EditorSurfaceState::for_text(&committed);
                apply_local_editor_text_change(formula_space, committed.clone(), restored_state);
                formula_space.committed_cell_text = Some(committed.clone());
                formula_space.proofed_cell_text = Some(committed);
            }
            true
        }
        EditorCommand::ToggleExpandedHeight => {
            formula_space.expanded_editor = !formula_space.expanded_editor;
            true
        }
        EditorCommand::DismissCompletion => {
            formula_space.editor_surface_state.completion_anchor_offset = None;
            formula_space.editor_surface_state.completion_selected_index = None;
            true
        }
        EditorCommand::CycleReferenceForm => {
            let selection = &formula_space.editor_surface_state.selection;
            if let Some(result) = crate::ui::editor::reference_cycle::cycle_reference_form(
                &formula_space.raw_entered_cell_text,
                selection.start(),
                selection.end(),
            ) {
                let next_state = crate::ui::editor::state::EditorSurfaceState {
                    caret: crate::ui::editor::state::EditorCaret {
                        offset: result.reference_end,
                    },
                    selection: crate::ui::editor::state::EditorSelection {
                        anchor: result.reference_start,
                        focus: result.reference_end,
                    },
                    scroll_window: formula_space
                        .editor_surface_state
                        .scroll_window
                        .clone(),
                    completion_anchor_offset: None,
                    completion_selected_index: None,
                    signature_help_anchor_offset: None,
                };
                apply_local_editor_text_change(formula_space, result.text, next_state);
            }
            true
        }
        EditorCommand::ForceShowCompletion | EditorCommand::SendSelectionToInspect => {
            // Model-level no-ops that the view layer or downstream services consume.
            true
        }
        _ => false,
    }
}

pub fn update_editor_setting(
    state: &mut OneCalcHostState,
    update: crate::ui::editor::state::EditorSettingUpdate,
) -> bool {
    state.global_ui_chrome.editor_settings.apply(update);
    true
}

pub fn toggle_editor_settings_popover(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.editor_settings_popover_open =
        !state.global_ui_chrome.editor_settings_popover_open;
    true
}

pub fn toggle_configure_drawer(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.configure_drawer_open =
        !state.global_ui_chrome.configure_drawer_open;
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

pub fn import_verification_bundle_report_into_workspace(
    state: &mut OneCalcHostState,
    request: VerificationBundleImportRequest,
) -> bool {
    import_verification_bundle_report_json(state, request).is_ok()
}

fn active_formula_space_mut(state: &mut OneCalcHostState) -> Option<&mut FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())?;
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

fn apply_completion_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
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
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
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
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
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
                caret: crate::ui::editor::state::EditorCaret {
                    offset: selection_end,
                },
                scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                completion_anchor_offset: formula_space
                    .editor_surface_state
                    .completion_anchor_offset,
                completion_selected_index: formula_space
                    .editor_surface_state
                    .completion_selected_index,
                signature_help_anchor_offset: formula_space
                    .editor_surface_state
                    .signature_help_anchor_offset,
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

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert!(active.raw_entered_cell_text.starts_with("    SUM("));
        assert!(active.latest_evaluation_summary.is_none());
        assert!(active.effective_display_summary.is_none());
    }

    /// §11.2 invariant 2: `apply_editor_command_to_active_formula_space`
    /// returns `false` when no active formula space exists (and is not a
    /// UI chrome command). Pins the guard in reducer.rs.
    #[test]
    fn apply_editor_command_returns_false_when_no_active_formula_space() {
        let mut state = OneCalcHostState::default();
        assert!(state.workspace_shell.active_formula_space_id.is_none());

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );
        assert!(!changed);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CommitEntry,
        );
        assert!(!changed);
    }

    /// §11.2 invariant 3: UI-chrome commands succeed even when no active
    /// formula space exists. `ToggleConfigureDrawer`,
    /// `ToggleEditorSettingsPopover`, and `UpdateEditorSetting(_)` mutate
    /// workspace-level state and must return `true` against an empty
    /// workspace.
    #[test]
    fn ui_chrome_commands_succeed_with_no_active_formula_space() {
        use crate::ui::editor::state::EditorSettingUpdate;
        let mut state = OneCalcHostState::default();

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleConfigureDrawer,
        ));
        assert!(state.global_ui_chrome.configure_drawer_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        ));
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        ));
        // The setting must have changed even though there's no formula space.
        assert!(!state.global_ui_chrome.editor_settings.highlight_bracket_pairs);
    }

    #[test]
    fn commit_entry_records_committed_text_and_transitions_live_state() {
        use crate::ui::editor::state::EditorLiveState;
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)"));

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CommitEntry,
        ));
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.committed_cell_text.as_deref(), Some("=SUM(1,2)"));
        assert_eq!(active.live_state(), EditorLiveState::Committed);

        apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: None,
                selection_end: None,
                input_kind: EditorInputKind::Other,
                inserted_text: None,
            },
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::EditingLive);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::RequestProof);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::ProofedScratch);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CancelEntry);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(active.live_state(), EditorLiveState::Committed);
    }

    #[test]
    fn editor_settings_popover_toggle_and_update_apply_to_global_chrome() {
        use crate::ui::editor::state::{CompletionAggressiveness, EditorSettingUpdate};
        let mut state = OneCalcHostState::default();
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        );
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        );
        assert!(!state.global_ui_chrome.editor_settings.highlight_bracket_pairs);

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::SetCompletionAggressiveness(
                CompletionAggressiveness::Always,
            )),
        );
        assert_eq!(
            state
                .global_ui_chrome
                .editor_settings
                .completion_aggressiveness,
            CompletionAggressiveness::Always
        );
    }

    /// §11.5 invariant 9: F4 with a single-cell reference under the
    /// caret rotates one step in the cycle and the selection covers the
    /// rewritten reference. Walks all four steps of the cycle through the
    /// reducer entry point so the integration between
    /// `reference_cycle::cycle_reference_form` and the reducer's caret /
    /// selection update is pinned end-to-end.
    #[test]
    fn cycle_reference_form_rewrites_cell_and_selects_new_span() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=A1+B2");
        formula_space.editor_surface_state.caret = crate::ui::editor::state::EditorCaret { offset: 1 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(1);
        state.formula_spaces.insert(formula_space);

        // Step 1: A1 → $A$1
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CycleReferenceForm,
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 5);

        // Step 2: $A$1 → A$1
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CycleReferenceForm,
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 3: A$1 → $A1
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CycleReferenceForm,
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 4: $A1 → A1 (back to the starting form)
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CycleReferenceForm,
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 3);
    }

    /// §11.5 invariant 8: accepting a completion proposal (via
    /// `AcceptCompletionByIndex(0)`) replaces the proposal's
    /// `replacement_span` with the `insert_text` and lands the caret at
    /// the end of the inserted text.
    #[test]
    fn accept_completion_from_tab_replaces_anchor_span_and_lands_caret_at_end() {
        use crate::adapters::oxfml::{
            CompletionProposal, CompletionProposalKind, EditorDocument, EditorSyntaxSnapshot,
            FormulaEditReuseSummary, FormulaTextSpan, LiveDiagnosticSnapshot,
        };

        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.caret =
            crate::ui::editor::state::EditorCaret { offset: 3 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(3);
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SU".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![],
            },
            live_diagnostics: LiveDiagnosticSnapshot::default(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: false,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: None,
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "completion-SUM".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                documentation_ref: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        });
        state.formula_spaces.insert(formula_space);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::AcceptCompletionByIndex(0),
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        // Replacement span [1, 3) covers "SU"; after insert_text = "SUM("
        // the text becomes "=SUM(" and the caret lands at offset 5.
        assert_eq!(active.raw_entered_cell_text, "=SUM(");
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn dismiss_completion_clears_anchor_and_selected_index() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.completion_anchor_offset = Some(1);
        formula_space.editor_surface_state.completion_selected_index = Some(2);
        state.formula_spaces.insert(formula_space);

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::DismissCompletion,
        );

        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert!(active.editor_surface_state.completion_anchor_offset.is_none());
        assert!(active.editor_surface_state.completion_selected_index.is_none());
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
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

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
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

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
