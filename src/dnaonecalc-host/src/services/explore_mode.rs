use crate::state::FormulaSpaceState;
use crate::ui::editor::render_projection::{syntax_runs_from_snapshot, SyntaxRun};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreViewModel {
    pub raw_entered_cell_text: String,
    pub syntax_runs: Vec<SyntaxRun>,
    pub diagnostics: Vec<ExploreDiagnosticView>,
    pub completion_count: usize,
    pub has_signature_help: bool,
    pub function_help_lookup_key: Option<String>,
    pub effective_display_summary: Option<String>,
    pub latest_evaluation_summary: Option<String>,
    pub green_tree_key: Option<String>,
    pub reused_green_tree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploreDiagnosticView {
    pub diagnostic_id: String,
    pub message: String,
    pub span_start: usize,
    pub span_len: usize,
}

pub fn build_explore_view_model(formula_space: &FormulaSpaceState) -> ExploreViewModel {
    let (
        syntax_runs,
        diagnostics,
        green_tree_key,
        reused_green_tree,
        document_function_help_lookup_key,
    ) = match &formula_space.editor_document {
        Some(document) => (
            syntax_runs_from_snapshot(&document.editor_syntax_snapshot),
            document
                .live_diagnostics
                .diagnostics
                .iter()
                .map(|diagnostic| ExploreDiagnosticView {
                    diagnostic_id: diagnostic.diagnostic_id.clone(),
                    message: diagnostic.message.clone(),
                    span_start: diagnostic.span_start,
                    span_len: diagnostic.span_len,
                })
                .collect(),
            Some(document.green_tree_key().to_string()),
            document.reuse_summary.reused_green_tree,
            document
                .function_help
                .as_ref()
                .map(|packet| packet.lookup_key.clone()),
        ),
        None => (
            vec![SyntaxRun {
                text: formula_space.raw_entered_cell_text.clone(),
                span_start: 0,
                span_len: formula_space.raw_entered_cell_text.chars().count(),
            }],
            Vec::new(),
            None,
            false,
            None,
        ),
    };

    ExploreViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        syntax_runs,
        diagnostics,
        completion_count: formula_space.completion_help.completion_count,
        has_signature_help: formula_space.completion_help.has_signature_help,
        function_help_lookup_key: formula_space
            .completion_help
            .function_help_lookup_key
            .clone()
            .or(document_function_help_lookup_key),
        effective_display_summary: formula_space.effective_display_summary.clone(),
        latest_evaluation_summary: formula_space.latest_evaluation_summary.clone(),
        green_tree_key,
        reused_green_tree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        EditorDocument, EditorSyntaxSnapshot, EditorToken, FormulaEditReuseSummary,
        LiveDiagnostic, LiveDiagnosticSnapshot,
    };
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{CompletionHelpState, FormulaSpaceState};

    #[test]
    fn explore_view_model_projects_editor_document_and_help_state() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
        formula_space.completion_help = CompletionHelpState {
            completion_count: 2,
            has_signature_help: true,
            function_help_lookup_key: Some("SUM".to_string()),
        };
        formula_space.effective_display_summary = Some("3".to_string());
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SUM(1,2)".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: EditorSyntaxSnapshot {
                formula_stable_id: "formula-1".to_string(),
                green_tree_key: "green-1".to_string(),
                tokens: vec![
                    EditorToken {
                        text: "=".to_string(),
                        span_start: 0,
                        span_len: 1,
                    },
                    EditorToken {
                        text: "SUM".to_string(),
                        span_start: 1,
                        span_len: 3,
                    },
                ],
            },
            live_diagnostics: LiveDiagnosticSnapshot {
                diagnostics: vec![LiveDiagnostic {
                    diagnostic_id: "diag-1".to_string(),
                    message: "sample".to_string(),
                    span_start: 1,
                    span_len: 3,
                }],
            },
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: None,
            function_help: None,
            completion_proposals: vec![],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
        });

        let view_model = build_explore_view_model(&formula_space);
        assert_eq!(view_model.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(view_model.syntax_runs.len(), 2);
        assert_eq!(view_model.diagnostics.len(), 1);
        assert_eq!(view_model.function_help_lookup_key.as_deref(), Some("SUM"));
        assert_eq!(view_model.effective_display_summary.as_deref(), Some("3"));
        assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
        assert!(view_model.reused_green_tree);
    }
}
