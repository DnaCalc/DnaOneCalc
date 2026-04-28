//! Bridge type surface for the host.
//!
//! Path B refactor (WS-14 dno-xcq.12): the host previously hand-rolled
//! mirror copies of upstream `oxfml_core` / `oxfunc_core` types and
//! translated upstream → mirror inside `live_bridge.rs`. That boundary
//! repeatedly dropped fields (severity, stage, value-kind, span
//! attribution) and forced follow-up beads each time. We now re-export
//! upstream types directly. The host adds only a small set of bundle /
//! projection types that combine multiple upstream pieces into shapes
//! tailored to the UI surface.

// ---------------------------------------------------------------------------
// Upstream re-exports — single source of truth.
// ---------------------------------------------------------------------------

pub use oxfml_core::consumer::editor::{
    CompletionProposal, CompletionProposalKind, EditorSyntaxSnapshot, EditorToken,
    FormulaEditReuseSummary, FormulaTextChangeRange, FunctionHelpPacket, FunctionHelpSignatureForm,
    LiveDiagnostic, LiveDiagnosticSeverity, LiveDiagnosticSnapshot, LiveDiagnosticStage,
    SignatureHelpContext,
};
pub use oxfml_core::syntax::token::TextSpan as FormulaTextSpan;
pub use oxfunc_core::value::{ArrayCellValue, EvalValue, WorksheetErrorCode};

// ---------------------------------------------------------------------------
// Host-side bundle types.
// ---------------------------------------------------------------------------

/// Discriminator-only view of an `EvalValue`. Convenience for matching
/// when the payload isn't needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaValueKind {
    Number,
    Text,
    Logical,
    Error,
    Array,
    Reference,
    Lambda,
}

impl FormulaValueKind {
    pub fn of(value: &EvalValue) -> Self {
        match value {
            EvalValue::Number(_) => Self::Number,
            EvalValue::Text(_) => Self::Text,
            EvalValue::Logical(_) => Self::Logical,
            EvalValue::Error(_) => Self::Error,
            EvalValue::Array(_) => Self::Array,
            EvalValue::Reference(_) => Self::Reference,
            EvalValue::Lambda(_) => Self::Lambda,
        }
    }
}

/// Render the canonical Excel literal for a worksheet error code
/// (e.g. `#NAME?`, `#DIV/0!`). Free-standing helper over the upstream
/// `WorksheetErrorCode` so call sites don't have to write the match
/// themselves.
pub fn worksheet_error_literal(code: WorksheetErrorCode) -> &'static str {
    match code {
        WorksheetErrorCode::Null => "#NULL!",
        WorksheetErrorCode::Div0 => "#DIV/0!",
        WorksheetErrorCode::Value => "#VALUE!",
        WorksheetErrorCode::Ref => "#REF!",
        WorksheetErrorCode::Name => "#NAME?",
        WorksheetErrorCode::Num => "#NUM!",
        WorksheetErrorCode::NA => "#N/A",
        WorksheetErrorCode::Busy => "#BUSY!",
        WorksheetErrorCode::GettingData => "#GETTING_DATA",
        WorksheetErrorCode::Spill => "#SPILL!",
        WorksheetErrorCode::Calc => "#CALC!",
        WorksheetErrorCode::Field => "#FIELD!",
        WorksheetErrorCode::Blocked => "#BLOCKED!",
        WorksheetErrorCode::Connect => "#CONNECT!",
    }
}

/// Walk-tree projection of a formula. Host-defined: upstream exposes a
/// richer `SemanticPlan` / `GreenTreeRoot`; this is the UX-shaped slice
/// the formula drill-down renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaWalkNode {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: FormulaWalkNodeState,
    pub children: Vec<FormulaWalkNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaWalkNodeState {
    Evaluated,
    Bound,
    Opaque,
    Blocked,
}

/// Compact parse-phase summary. Host-bundle of upstream parse signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSummary {
    pub status: String,
    pub token_count: usize,
}

/// Compact bind-phase summary. Host-bundle of upstream bind signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindSummary {
    pub variable_count: usize,
    pub reference_count: usize,
}

/// Compact eval-phase summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSummary {
    pub step_count: usize,
    pub duration_text: String,
}

/// Provenance fragment used in the legacy three-mode shell. Will retire
/// during the WS-14 cleanup phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceSummary {
    pub profile_summary: String,
    pub blocked_reason: Option<String>,
}

/// Truncated array preview for the result block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaArrayPreview {
    pub label: String,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

/// What the host shows for the cell's value. Bundles the upstream typed
/// value with display strings so the UI layer can pick the right rendering
/// without re-running OxFml.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaValuePresentation {
    pub evaluation_summary: String,
    pub effective_display_summary: Option<String>,
    pub array_preview: Option<FormulaArrayPreview>,
    pub blocked_reason: Option<String>,
    /// Typed value passed through verbatim from `RuntimeFormulaResult.published_worksheet_value`.
    /// View-models dispatch on this discriminator (Number / Text / Logical /
    /// Error / Array / Reference / Lambda) instead of parsing strings.
    pub published_value: EvalValue,
}

/// Host's bundle of upstream editor-interaction + runtime-result. This
/// is what a single `apply_formula_edit` call produces and what the
/// host stores per formula space. Field types reference upstream types
/// directly (no information loss at the bridge boundary).
#[derive(Debug, Clone, PartialEq)]
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
    pub value_presentation: Option<FormulaValuePresentation>,
}

impl EditorDocument {
    pub fn green_tree_key(&self) -> &str {
        &self.editor_syntax_snapshot.green_tree_key
    }
}
