#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaTextChangeRange {
    pub start: usize,
    pub old_len: usize,
    pub new_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorToken {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSyntaxSnapshot {
    pub formula_stable_id: String,
    pub green_tree_key: String,
    pub tokens: Vec<EditorToken>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveDiagnostic {
    pub diagnostic_id: String,
    pub message: String,
    pub span_start: usize,
    pub span_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveDiagnosticSnapshot {
    pub diagnostics: Vec<LiveDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaEditReuseSummary {
    pub reused_green_tree: bool,
    pub reused_red_projection: bool,
    pub reused_bound_formula: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionProposal {
    pub proposal_id: String,
    pub display_text: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpContext {
    pub callee_text: String,
    pub active_argument_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHelpPacket {
    pub lookup_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorDocument {
    pub source_text: String,
    pub text_change_range: Option<FormulaTextChangeRange>,
    pub editor_syntax_snapshot: EditorSyntaxSnapshot,
    pub live_diagnostics: LiveDiagnosticSnapshot,
    pub reuse_summary: FormulaEditReuseSummary,
    pub signature_help: Option<SignatureHelpContext>,
    pub function_help: Option<FunctionHelpPacket>,
    pub completion_proposals: Vec<CompletionProposal>,
}

impl EditorDocument {
    pub fn green_tree_key(&self) -> &str {
        &self.editor_syntax_snapshot.green_tree_key
    }
}
