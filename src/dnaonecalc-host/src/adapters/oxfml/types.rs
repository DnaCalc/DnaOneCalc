#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaTextChangeRange {
    pub start: usize,
    pub old_len: usize,
    pub new_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormulaTextSpan {
    pub start: usize,
    pub len: usize,
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
    pub replacement_span: Option<FormulaTextSpan>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaWalkNodeState {
    Evaluated,
    Bound,
    Opaque,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaWalkNode {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: FormulaWalkNodeState,
    pub children: Vec<FormulaWalkNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSummary {
    pub status: String,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindSummary {
    pub variable_count: usize,
    pub reference_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSummary {
    pub step_count: usize,
    pub duration_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceSummary {
    pub profile_summary: String,
    pub blocked_reason: Option<String>,
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
    pub formula_walk: Vec<FormulaWalkNode>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
}

impl EditorDocument {
    pub fn green_tree_key(&self) -> &str {
        &self.editor_syntax_snapshot.green_tree_key
    }
}
