use crate::adapters::oxfml::EditorDocument;
use crate::domain::ids::FormulaSpaceId;
use crate::state::{CompletionHelpState, FormulaSpaceCollectionState, FormulaSpaceState};

#[derive(Debug, Default)]
pub struct EditorSessionService;

impl EditorSessionService {
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
    formula_space.raw_entered_cell_text = document.source_text.clone();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        CompletionProposal, EditorSyntaxSnapshot, FormulaEditReuseSummary,
        LiveDiagnosticSnapshot, SignatureHelpContext,
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
                active_argument_index: 1,
            }),
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "proposal-1".to_string(),
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
            }],
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
        assert_eq!(
            updated
                .editor_document
                .as_ref()
                .expect("editor document retained")
                .green_tree_key(),
            "green-1"
        );
    }
}
