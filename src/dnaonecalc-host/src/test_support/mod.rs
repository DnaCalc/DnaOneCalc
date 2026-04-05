use crate::adapters::oxfml::{
    BindSummary, CompletionProposal, EditorDocument, EditorSyntaxSnapshot, EditorToken,
    EvalSummary, FormulaEditReuseSummary, FormulaTextChangeRange, FormulaWalkNode,
    FormulaWalkNodeState, FunctionHelpPacket, LiveDiagnostic, LiveDiagnosticSnapshot,
    ParseSummary, ProvenanceSummary, SignatureHelpContext,
};

pub fn sample_editor_document(source_text: &str) -> EditorDocument {
    let tokens = sample_editor_tokens(source_text);

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
            tokens,
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
        formula_walk: vec![FormulaWalkNode {
            node_id: "node-1".to_string(),
            label: "SUM".to_string(),
            value_preview: Some("3".to_string()),
            state: FormulaWalkNodeState::Evaluated,
            children: vec![],
        }],
        parse_summary: Some(ParseSummary {
            status: "Valid".to_string(),
            token_count: 1,
        }),
        bind_summary: Some(BindSummary {
            variable_count: 0,
            reference_count: 0,
        }),
        eval_summary: Some(EvalSummary {
            step_count: 1,
            duration_text: "0.1ms".to_string(),
        }),
        provenance_summary: Some(ProvenanceSummary {
            profile_summary: "OC-H0".to_string(),
            blocked_reason: None,
        }),
    }
}

fn sample_editor_tokens(source_text: &str) -> Vec<EditorToken> {
    if source_text == "=SUM(1,2)" {
        vec![
            token("=", 0),
            token("SUM", 1),
            token("(", 4),
            token("1", 5),
            token(",", 6),
            token("2", 7),
            token(")", 8),
        ]
    } else if source_text == "=LET(x,1,x)" {
        vec![
            token("=", 0),
            token("LET", 1),
            token("(", 4),
            token("x", 5),
            token(",", 6),
            token("1", 7),
            token(",", 8),
            token("x", 9),
            token(")", 10),
        ]
    } else {
        vec![token(source_text, 0)]
    }
}

fn token(text: &str, span_start: usize) -> EditorToken {
    EditorToken {
        text: text.to_string(),
        span_start,
        span_len: text.chars().count(),
    }
}
