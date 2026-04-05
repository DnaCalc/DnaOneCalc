use crate::services::explore_mode::{
    ExploreCompletionItemView, ExploreDiagnosticView, ExploreFunctionHelpView,
    ExploreSignatureHelpView, ExploreViewModel,
};
use crate::ui::editor::render_projection::SyntaxRun;
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreEditorClusterViewModel {
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub syntax_runs: Vec<SyntaxRun>,
    pub diagnostics: Vec<ExploreDiagnosticView>,
    pub completion_count: usize,
    pub completion_items: Vec<ExploreCompletionItemView>,
    pub selected_completion_proposal_id: Option<String>,
    pub completion_anchor_span: Option<crate::adapters::oxfml::FormulaTextSpan>,
    pub has_signature_help: bool,
    pub signature_help: Option<ExploreSignatureHelpView>,
    pub function_help: Option<ExploreFunctionHelpView>,
    pub function_help_lookup_key: Option<String>,
    pub green_tree_key: Option<String>,
    pub reused_green_tree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreResultClusterViewModel {
    pub effective_display_summary: Option<String>,
    pub latest_evaluation_summary: Option<String>,
}

pub fn build_explore_editor_cluster(
    view_model: &ExploreViewModel,
) -> ExploreEditorClusterViewModel {
    ExploreEditorClusterViewModel {
        raw_entered_cell_text: view_model.raw_entered_cell_text.clone(),
        editor_surface_state: view_model.editor_surface_state.clone(),
        syntax_runs: view_model.syntax_runs.clone(),
        diagnostics: view_model.diagnostics.clone(),
        completion_count: view_model.completion_count,
        completion_items: view_model.completion_items.clone(),
        selected_completion_proposal_id: view_model
            .editor_surface_state
            .completion_selected_index
            .and_then(|index| view_model.completion_items.get(index))
            .map(|item| item.proposal_id.clone()),
        completion_anchor_span: view_model
            .editor_surface_state
            .completion_selected_index
            .and_then(|index| view_model.completion_items.get(index))
            .and_then(|item| item.replacement_span),
        has_signature_help: view_model.has_signature_help,
        signature_help: view_model.signature_help.clone(),
        function_help: view_model.function_help.clone(),
        function_help_lookup_key: view_model.function_help_lookup_key.clone(),
        green_tree_key: view_model.green_tree_key.clone(),
        reused_green_tree: view_model.reused_green_tree,
    }
}

pub fn build_explore_result_cluster(
    view_model: &ExploreViewModel,
) -> ExploreResultClusterViewModel {
    ExploreResultClusterViewModel {
        effective_display_summary: view_model.effective_display_summary.clone(),
        latest_evaluation_summary: view_model.latest_evaluation_summary.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::FormulaTextSpan;
    use crate::services::explore_mode::{
        ExploreCompletionItemView, ExploreCompletionKindView, ExploreDiagnosticView,
        ExploreFunctionHelpSignatureView, ExploreSignatureHelpView, ExploreViewModel,
    };
    use crate::ui::editor::render_projection::SyntaxTokenRole;

    #[test]
    fn explore_editor_cluster_keeps_editing_surface_fields() {
        let view_model = ExploreViewModel {
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            editor_surface_state: EditorSurfaceState {
                completion_selected_index: Some(0),
                completion_anchor_offset: Some(4),
                signature_help_anchor_offset: Some(4),
                ..EditorSurfaceState::for_text("=SUM(1,2)")
            },
            syntax_runs: vec![SyntaxRun {
                text: "SUM".to_string(),
                span_start: 1,
                span_len: 3,
                role: SyntaxTokenRole::Function,
            }],
            diagnostics: vec![ExploreDiagnosticView {
                diagnostic_id: "diag-1".to_string(),
                message: "sample".to_string(),
                span_start: 1,
                span_len: 3,
            }],
            completion_count: 2,
            completion_items: vec![ExploreCompletionItemView {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: ExploreCompletionKindView::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 3 }),
                documentation_ref: Some("preview:function:SUM".to_string()),
                requires_revalidation: true,
            }],
            has_signature_help: true,
            signature_help: Some(ExploreSignatureHelpView {
                callee_text: "SUM".to_string(),
                call_span: FormulaTextSpan { start: 0, len: 9 },
                active_argument_index: 1,
            }),
            function_help: Some(ExploreFunctionHelpView {
                lookup_key: "SUM".to_string(),
                display_name: "SUM".to_string(),
                signature_forms: vec![ExploreFunctionHelpSignatureView {
                    display_signature: "SUM(number1, number2, ...)".to_string(),
                    min_arity: 1,
                    max_arity: None,
                }],
                argument_help: vec!["number1".to_string(), "number2".to_string()],
                short_description: Some("Adds numbers".to_string()),
                availability_summary: Some("supported".to_string()),
                deferred_or_profile_limited: false,
            }),
            function_help_lookup_key: Some("SUM".to_string()),
            effective_display_summary: Some("3".to_string()),
            latest_evaluation_summary: Some("Number".to_string()),
            green_tree_key: Some("green-1".to_string()),
            reused_green_tree: true,
        };

        let cluster = build_explore_editor_cluster(&view_model);
        assert_eq!(cluster.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(cluster.editor_surface_state.caret.offset, 9);
        assert_eq!(cluster.syntax_runs.len(), 1);
        assert_eq!(cluster.diagnostics.len(), 1);
        assert_eq!(cluster.completion_items.len(), 1);
        assert_eq!(cluster.selected_completion_proposal_id.as_deref(), Some("proposal-1"));
        assert_eq!(cluster.completion_anchor_span, Some(FormulaTextSpan { start: 1, len: 3 }));
        assert_eq!(
            cluster.signature_help.as_ref().map(|help| help.active_argument_index),
            Some(1)
        );
        assert_eq!(cluster.function_help.as_ref().map(|help| help.display_name.as_str()), Some("SUM"));
        assert_eq!(cluster.function_help_lookup_key.as_deref(), Some("SUM"));
        assert!(cluster.reused_green_tree);
    }

    #[test]
    fn explore_result_cluster_keeps_result_surface_fields() {
        let view_model = ExploreViewModel {
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            editor_surface_state: EditorSurfaceState::for_text("=SUM(1,2)"),
            syntax_runs: vec![],
            diagnostics: vec![],
            completion_count: 0,
            completion_items: vec![],
            has_signature_help: false,
            signature_help: None,
            function_help: None,
            function_help_lookup_key: None,
            effective_display_summary: Some("3".to_string()),
            latest_evaluation_summary: Some("Number".to_string()),
            green_tree_key: None,
            reused_green_tree: false,
        };

        let cluster = build_explore_result_cluster(&view_model);
        assert_eq!(cluster.effective_display_summary.as_deref(), Some("3"));
        assert_eq!(cluster.latest_evaluation_summary.as_deref(), Some("Number"));
    }
}
