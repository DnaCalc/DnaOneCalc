use crate::adapters::oxfml::{
    EditorDocument, FormulaEditRequest, OxfmlEditorBridge, OxfmlEditorBridgeError,
};
use crate::app::intents::ApplyFormulaEditIntent;
use crate::domain::ids::FormulaSpaceId;
use crate::state::{CompletionHelpState, FormulaSpaceCollectionState, FormulaSpaceState};
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Default)]
pub struct EditorSessionService;

impl EditorSessionService {
    pub fn handle_formula_edit_intent(
        bridge: &dyn OxfmlEditorBridge,
        formula_spaces: &mut FormulaSpaceCollectionState,
        intent: ApplyFormulaEditIntent,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get(&intent.formula_space_id)
            .ok_or_else(|| EditorSessionError::UnknownFormulaSpace(intent.formula_space_id.clone()))?;
        let request = FormulaEditRequest {
            formula_stable_id: intent.formula_stable_id,
            entered_text: intent.entered_text,
            cursor_offset: intent.cursor_offset,
            previous_green_tree_key: formula_space
                .editor_document
                .as_ref()
                .map(|document| document.green_tree_key().to_string()),
            analysis_stage: intent.analysis_stage,
        };
        let result = bridge
            .apply_formula_edit(request)
            .map_err(EditorSessionError::Bridge)?;
        Self::apply_editor_document(formula_spaces, &intent.formula_space_id, result.document)
    }

    pub fn apply_editor_document(
        formula_spaces: &mut FormulaSpaceCollectionState,
        formula_space_id: &FormulaSpaceId,
        document: EditorDocument,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get_mut(formula_space_id)
            .ok_or_else(|| EditorSessionError::UnknownFormulaSpace(formula_space_id.clone()))?;
        update_formula_space_from_editor_document(formula_space, document);
        Ok(())
    }
}

fn update_formula_space_from_editor_document(
    formula_space: &mut FormulaSpaceState,
    document: EditorDocument,
) {
    let mut editor_surface_state = EditorSurfaceState::for_text_with_selection(
        &document.source_text,
        formula_space.editor_surface_state.selection.anchor,
        formula_space.editor_surface_state.selection.focus,
    );
    editor_surface_state.scroll_window = formula_space.editor_surface_state.scroll_window.clone();
    editor_surface_state.completion_anchor_offset =
        (!document.completion_proposals.is_empty()).then_some(editor_surface_state.caret.offset);
    editor_surface_state.completion_selected_index =
        (!document.completion_proposals.is_empty()).then_some(0);
    editor_surface_state.signature_help_anchor_offset =
        document.signature_help.as_ref().map(|_| editor_surface_state.caret.offset);

    formula_space.raw_entered_cell_text = document.source_text.clone();
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.completion_help = CompletionHelpState {
        completion_count: document.completion_proposals.len(),
        has_signature_help: document.signature_help.is_some(),
        function_help_lookup_key: document.function_help.as_ref().map(|packet| packet.lookup_key.clone()),
    };
    formula_space.editor_document = Some(document);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSessionError {
    UnknownFormulaSpace(FormulaSpaceId),
    Bridge(OxfmlEditorBridgeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        CompletionProposal, CompletionProposalKind, EditorAnalysisStage, EditorSyntaxSnapshot,
        FormulaEditResult, FormulaEditReuseSummary, FormulaTextSpan, LiveDiagnosticSnapshot,
        SignatureHelpContext,
    };

    fn sample_document(source_text: &str) -> EditorDocument {
        EditorDocument {
            source_text: source_text.to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![],
            },
            live_diagnostics: LiveDiagnosticSnapshot::default(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: true,
                reused_bound_formula: false,
            },
            signature_help: Some(SignatureHelpContext {
                callee_text: "SUM".to_string(),
                call_span: FormulaTextSpan { start: 0, len: source_text.chars().count() },
                active_argument_index: 1,
            }),
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: None,
                documentation_ref: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
        }
    }

    #[test]
    fn apply_editor_document_updates_formula_space_text_and_help() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=1+1",
        ));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("'123.4"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "'123.4");
        assert_eq!(updated.completion_help.completion_count, 1);
        assert!(updated.completion_help.has_signature_help);
        assert_eq!(updated.editor_surface_state.completion_anchor_offset, Some(4));
        assert_eq!(updated.editor_surface_state.completion_selected_index, Some(0));
        assert_eq!(updated.editor_surface_state.signature_help_anchor_offset, Some(4));
        assert_eq!(
            updated
                .editor_document
                .as_ref()
                .expect("editor document retained")
            .green_tree_key(),
            "green-1"
        );
    }

    struct FakeBridge {
        document: EditorDocument,
    }

    impl OxfmlEditorBridge for FakeBridge {
        fn apply_formula_edit(
            &self,
            request: FormulaEditRequest,
        ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
            assert_eq!(request.formula_stable_id, "formula-1");
            assert_eq!(request.entered_text, "=SUM(1,2,3)");
            assert_eq!(request.cursor_offset, 4);
            assert_eq!(request.analysis_stage, EditorAnalysisStage::SyntaxAndBind);
            assert!(request.previous_green_tree_key.is_none());
            Ok(FormulaEditResult {
                document: self.document.clone(),
            })
        }
    }

    #[test]
    fn handle_formula_edit_intent_routes_through_bridge_and_updates_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=1+1",
        ));
        let bridge = FakeBridge {
            document: sample_document("=SUM(1,2,3)"),
        };

        EditorSessionService::handle_formula_edit_intent(
            &bridge,
            &mut formula_spaces,
            ApplyFormulaEditIntent {
                formula_space_id: formula_space_id.clone(),
                formula_stable_id: "formula-1".to_string(),
                entered_text: "=SUM(1,2,3)".to_string(),
                cursor_offset: 4,
                analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            },
        )
        .expect("edit intent should update via bridge");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "=SUM(1,2,3)");
    }
}
