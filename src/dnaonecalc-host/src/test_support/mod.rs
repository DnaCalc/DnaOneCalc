use crate::adapters::oxfml::{
    CompletionProposal, EditorDocument, EditorSyntaxSnapshot, EditorToken,
    FormulaEditReuseSummary, FormulaTextChangeRange, FunctionHelpPacket, LiveDiagnostic,
    LiveDiagnosticSnapshot, SignatureHelpContext,
};

pub fn sample_editor_document(source_text: &str) -> EditorDocument {
    EditorDocument {
        source_text: source_text.to_string(),
        text_change_range: Some(FormulaTextChangeRange {
            start: 0,
            old_len: 0,
            new_len: source_text.chars().count(),
        }),
        editor_syntax_snapshot: EditorSyntaxSnapshot {
            formula_stable_id: "formula-1".to_string(),
            green_tree_key: "green-1".to_string(),
            tokens: vec![EditorToken {
                text: source_text.to_string(),
                span_start: 0,
                span_len: source_text.chars().count(),
            }],
        },
        live_diagnostics: LiveDiagnosticSnapshot {
            diagnostics: vec![LiveDiagnostic {
                diagnostic_id: "diag-1".to_string(),
                message: "sample diagnostic".to_string(),
                span_start: 0,
                span_len: source_text.chars().count(),
            }],
        },
        reuse_summary: FormulaEditReuseSummary {
            reused_green_tree: false,
            reused_red_projection: false,
            reused_bound_formula: false,
        },
        signature_help: Some(SignatureHelpContext {
            callee_text: "SUM".to_string(),
            active_argument_index: 0,
        }),
        function_help: Some(FunctionHelpPacket {
            lookup_key: "SUM".to_string(),
        }),
        completion_proposals: vec![CompletionProposal {
            proposal_id: "proposal-1".to_string(),
            display_text: "SUM".to_string(),
            insert_text: "SUM(".to_string(),
        }],
    }
}
