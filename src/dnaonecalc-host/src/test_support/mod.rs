use crate::adapters::oxfml::{
    BindSummary, CompletionProposal, CompletionProposalKind, EditorDocument,
    EditorSyntaxSnapshot, EditorToken, EvalSummary, FormulaEditReuseSummary,
    FormulaTextChangeRange, FormulaTextSpan, FormulaWalkNode, FormulaWalkNodeState,
    FunctionHelpPacket, FunctionHelpSignatureForm, LiveDiagnostic, LiveDiagnosticSnapshot,
    ParseSummary, ProvenanceSummary, SignatureHelpContext,
};

pub fn sample_editor_document(source_text: &str) -> EditorDocument {
    sample_editor_document_with_green_key(source_text, "green-1")
}

pub fn sample_editor_document_with_green_key(source_text: &str, green_tree_key: &str) -> EditorDocument {
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
            green_tree_key: green_tree_key.to_string(),
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
            call_span: FormulaTextSpan {
                start: 0,
                len: source_text.chars().count(),
            },
            active_argument_index: 0,
        }),
        function_help: Some(FunctionHelpPacket {
            lookup_key: "SUM".to_string(),
            display_name: "SUM".to_string(),
            signature_forms: vec![FunctionHelpSignatureForm {
                display_signature: "SUM(number1, number2, ...)".to_string(),
                min_arity: 1,
                max_arity: None,
            }],
            argument_help: vec![
                "number1".to_string(),
                "number2".to_string(),
                "additional_numbers".to_string(),
            ],
            short_description: Some("Adds numbers together.".to_string()),
            availability_summary: Some("supported".to_string()),
            deferred_or_profile_limited: false,
        }),
        completion_proposals: vec![CompletionProposal {
            proposal_id: "proposal-1".to_string(),
            proposal_kind: CompletionProposalKind::Function,
            display_text: "SUM".to_string(),
            insert_text: "SUM(".to_string(),
            replacement_span: Some(FormulaTextSpan { start: 1, len: 3 }),
            documentation_ref: Some("preview:function:SUM".to_string()),
            requires_revalidation: true,
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
        value_presentation: None,
    }
}

pub fn diagnostic_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-diag-1");
    document.live_diagnostics = LiveDiagnosticSnapshot {
        diagnostics: vec![LiveDiagnostic {
            diagnostic_id: "diag-missing-arg".to_string(),
            message: "Missing trailing argument".to_string(),
            span_start: source_text.len().saturating_sub(2),
            span_len: 1,
        }],
    };
    document.parse_summary = Some(ParseSummary {
        status: "Recoverable".to_string(),
        token_count: source_text.chars().count(),
    });
    document.eval_summary = Some(EvalSummary {
        step_count: 0,
        duration_text: "diagnostic-only".to_string(),
    });
    document
}

pub fn array_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-array-1");
    document.function_help = Some(FunctionHelpPacket {
        lookup_key: "SEQUENCE".to_string(),
        display_name: "SEQUENCE".to_string(),
        signature_forms: vec![FunctionHelpSignatureForm {
            display_signature: "SEQUENCE(rows, columns, start, step)".to_string(),
            min_arity: 1,
            max_arity: Some(4),
        }],
        argument_help: vec![
            "rows".to_string(),
            "columns".to_string(),
            "start".to_string(),
            "step".to_string(),
        ],
        short_description: Some("Returns a spilled array of sequential numbers.".to_string()),
        availability_summary: Some("dynamic arrays supported".to_string()),
        deferred_or_profile_limited: false,
    });
    document.signature_help = Some(SignatureHelpContext {
        callee_text: "SEQUENCE".to_string(),
        call_span: FormulaTextSpan {
            start: 0,
            len: source_text.chars().count(),
        },
        active_argument_index: 1,
    });
    document.formula_walk = vec![FormulaWalkNode {
        node_id: "node-sequence".to_string(),
        label: "SEQUENCE".to_string(),
        value_preview: Some("{1,2;3,4}".to_string()),
        state: FormulaWalkNodeState::Evaluated,
        children: vec![
            FormulaWalkNode {
                node_id: "node-rows".to_string(),
                label: "rows".to_string(),
                value_preview: Some("2".to_string()),
                state: FormulaWalkNodeState::Bound,
                children: vec![],
            },
            FormulaWalkNode {
                node_id: "node-cols".to_string(),
                label: "columns".to_string(),
                value_preview: Some("2".to_string()),
                state: FormulaWalkNodeState::Bound,
                children: vec![],
            },
        ],
    }];
    document.eval_summary = Some(EvalSummary {
        step_count: 4,
        duration_text: "0.3ms".to_string(),
    });
    document
}

pub fn blocked_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-blocked-1");
    document.formula_walk = vec![FormulaWalkNode {
        node_id: "node-xlookup".to_string(),
        label: "XLOOKUP".to_string(),
        value_preview: None,
        state: FormulaWalkNodeState::Blocked,
        children: vec![],
    }];
    document.provenance_summary = Some(ProvenanceSummary {
        profile_summary: "PreviewBridge".to_string(),
        blocked_reason: Some("Excel comparison lane unavailable on this host".to_string()),
    });
    document.eval_summary = Some(EvalSummary {
        step_count: 1,
        duration_text: "blocked".to_string(),
    });
    document
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
