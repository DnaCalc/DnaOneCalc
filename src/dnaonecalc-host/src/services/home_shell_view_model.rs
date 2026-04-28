//! WS-14 home-shell view-model.
//!
//! Pure projection function that reads the active formula space out of
//! `OneCalcHostState` and produces a small `HomeShellViewModel` describing
//! what the home shell renders: textarea text + caret, a five-way
//! `ResultView`, and a `StatusView` for the status-foot dot + green-tree key.
//!
//! The result projection dispatches **typed**: it reads the bridge's typed
//! `published_value: EvalValue` from `editor_document.value_presentation`,
//! the live diagnostics list from `editor_document.live_diagnostics`, the
//! provenance blocked reason from `editor_document.provenance_summary`, and
//! the host-derived `context.blocked_reason` — never re-parsing the
//! `latest_evaluation_summary` string. Pre-bridge text/number cells (input
//! that does not start with `=`) are hand-evaluated inline against the raw
//! source text. Array results flow through `formula_space.array_preview`.
//!
//! Reference: `docs/WS14_PRE_MVP_PATH.md` §4 Step 2.

use crate::adapters::oxfml::{worksheet_error_literal, EvalValue, LiveDiagnosticSeverity};
use crate::state::{FormulaSpaceState, OneCalcHostState, ProjectionTruthSource};
use crate::ui::editor::render_projection::{syntax_runs_from_snapshot, SyntaxRun, SyntaxTokenRole};
use crate::ui::editor::state::{EditorEntryMode, EditorSurfaceState};

/// Top-level home-shell projection.
///
/// Built freshly per render via `build_home_shell_view_model`. Returns
/// `None` when there is no active formula space (the home shell renders an
/// empty state in that case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeShellViewModel {
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    /// Pill rendered above the editor textarea (Formula / Value / Text /
    /// Empty), classified from `raw_entered_cell_text`.
    pub entry_mode_pill: EntryModePill,
    /// Pill rendered above the result hero (Number / Text / Logical /
    /// Error / Array / Other). `None` when there is no result to label —
    /// `ResultView::Empty` and `ResultView::Pending` suppress the pill.
    pub result_class_pill: Option<ResultClassPill>,
    /// Coloured-token runs for the syntax overlay rendered behind the
    /// textarea. Empty when the editor document is missing or stale (its
    /// source text does not match `raw_entered_cell_text`); the home shell
    /// renders the raw text uncoloured in that case.
    pub syntax_runs: Vec<SyntaxRun>,
    /// Diagnostic squiggles to overlay on top of the textarea, one entry
    /// per upstream `LiveDiagnostic`. Sorted by `span_start` ascending and
    /// pruned of entries that overlap with an earlier one (the upstream
    /// list is already non-overlapping in practice; the prune is a
    /// belt-and-braces guard).
    pub diagnostic_squiggles: Vec<DiagnosticSquiggle>,
    /// Live counts strip rendered at the editor-foot chip:
    /// `tokens N · functions M · diagnostics K`. Counts come straight off
    /// the editor document (zeros when there is no document yet).
    pub editor_metrics: EditorMetricsChip,
    /// Active-context summary rendered at the result-foot chip:
    /// `locale · format · policy`. Each field is either `Live(value)` or
    /// `SeamPending {value, seam_id}` — the renderer surfaces the SEAM id
    /// inline so the chip is honest about which knobs are wired today
    /// and which still need backend work (per WS-14 plan §11).
    pub result_context: ResultContextChip,
    pub result_view: ResultView,
    pub status: StatusView,
}

/// Editor-foot live-metrics chip projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMetricsChip {
    pub token_count: usize,
    pub function_count: usize,
    pub diagnostic_count: usize,
}

/// Result-foot active-context chip projection. Each field carries either a
/// live value or a SEAM-pending placeholder; the renderer composes the
/// dot-separated string `locale · format · policy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultContextChip {
    pub locale: ContextChipField,
    pub format: ContextChipField,
    pub policy: ContextChipField,
}

/// One field inside the result-context chip. SEAM-pending fields carry the
/// canonical SEAM id from WS-14 plan §11 so the renderer can attach
/// `data-seam-id` and `aria-describedby` and tooltips can surface it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextChipField {
    Live(String),
    SeamPending { value: String, seam_id: String },
}

impl ContextChipField {
    pub fn value(&self) -> &str {
        match self {
            Self::Live(value) => value.as_str(),
            Self::SeamPending { value, .. } => value.as_str(),
        }
    }

    pub fn seam_id(&self) -> Option<&str> {
        match self {
            Self::Live(_) => None,
            Self::SeamPending { seam_id, .. } => Some(seam_id.as_str()),
        }
    }
}

/// A single underline overlay positioned at a diagnostic's source span.
/// Carries enough information to render the squiggle, drive its colour by
/// severity, and supply a hover tooltip via `title` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSquiggle {
    pub diagnostic_id: String,
    pub message: String,
    pub severity: SquiggleSeverity,
    pub span_start: usize,
    pub span_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquiggleSeverity {
    Error,
    Warning,
    Info,
}

impl SquiggleSeverity {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    fn from_upstream(severity: LiveDiagnosticSeverity) -> Self {
        match severity {
            LiveDiagnosticSeverity::Error => Self::Error,
            LiveDiagnosticSeverity::Warning => Self::Warning,
            LiveDiagnosticSeverity::Info => Self::Info,
        }
    }
}

/// Editor-caption pill mirroring `EditorEntryMode` but lifted into the
/// view-model so the component never re-classifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryModePill {
    Formula,
    Value,
    Text,
    Empty,
}

impl EntryModePill {
    pub fn label(self) -> &'static str {
        match self {
            Self::Formula => "Formula",
            Self::Value => "Value",
            Self::Text => "Text",
            Self::Empty => "Empty",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::Value => "value",
            Self::Text => "text",
            Self::Empty => "empty",
        }
    }

    fn from_entry_mode(mode: EditorEntryMode) -> Self {
        match mode {
            EditorEntryMode::Formula => Self::Formula,
            EditorEntryMode::Value => Self::Value,
            EditorEntryMode::Text => Self::Text,
            EditorEntryMode::Empty => Self::Empty,
        }
    }
}

/// Result-caption pill labelling the value class shown in the result hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultClassPill {
    Number,
    Text,
    Logical,
    Error,
    Array,
    Other,
}

impl ResultClassPill {
    pub fn label(self) -> &'static str {
        match self {
            Self::Number => "Number",
            Self::Text => "Text",
            Self::Logical => "Logical",
            Self::Error => "Error",
            Self::Array => "Array",
            Self::Other => "Other",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Text => "text",
            Self::Logical => "logical",
            Self::Error => "error",
            Self::Array => "array",
            Self::Other => "other",
        }
    }

    fn from_result_view(view: &ResultView) -> Option<Self> {
        match view {
            ResultView::Empty | ResultView::Pending => None,
            ResultView::Error { .. } => Some(Self::Error),
            ResultView::Array { .. } => Some(Self::Array),
            ResultView::Display { kind, .. } => Some(match kind {
                ResultKind::Number => Self::Number,
                ResultKind::Text => Self::Text,
                ResultKind::Logical => Self::Logical,
                ResultKind::RichValue => Self::Other,
                ResultKind::Other => Self::Other,
            }),
        }
    }
}

/// What the result block should render. Mirrors the shape called out in the
/// path doc and matches the five mutually-exclusive UI states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultView {
    /// Editor is empty — show a muted placeholder.
    Empty,
    /// Editor has text but no eval result yet — show "..." muted.
    Pending,
    /// A scalar / text / logical result we can render.
    Display { text: String, kind: ResultKind },
    /// A diagnostic, blocked-lane, or error code state.
    Error {
        code: String,
        surface_repr: Option<String>,
    },
    /// An array result; preview is deferred to a later WS-14 phase.
    Array {
        rows: usize,
        cols: usize,
        label: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Number,
    Text,
    Logical,
    RichValue,
    Other,
}

/// Status-foot projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub bridge_health: BridgeHealth,
    pub truth_source: ProjectionTruthSource,
    pub green_tree_key: Option<String>,
}

/// Coarse bridge health for the status-foot dot. `Live` is sage; `Stale`
/// is amber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeHealth {
    Live,
    Stale,
}

/// Resolve the active formula space and project its view-model. Returns
/// `None` when no formula space is active or the active id has no entry.
pub fn build_home_shell_view_model(state: &OneCalcHostState) -> Option<HomeShellViewModel> {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref()?;
    let formula_space = state.formula_spaces.get(active_id)?;
    Some(project_formula_space(formula_space))
}

fn project_formula_space(formula_space: &FormulaSpaceState) -> HomeShellViewModel {
    let result_view = project_result_view(formula_space);
    let entry_mode_pill = EntryModePill::from_entry_mode(EditorEntryMode::classify(
        &formula_space.raw_entered_cell_text,
    ));
    let result_class_pill = ResultClassPill::from_result_view(&result_view);
    let syntax_runs = project_syntax_runs(formula_space);
    let diagnostic_squiggles = project_diagnostic_squiggles(formula_space);
    let editor_metrics = project_editor_metrics(formula_space, &syntax_runs);
    let result_context = project_result_context(formula_space);
    HomeShellViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        editor_surface_state: formula_space.editor_surface_state.clone(),
        entry_mode_pill,
        result_class_pill,
        syntax_runs,
        diagnostic_squiggles,
        editor_metrics,
        result_context,
        result_view,
        status: project_status_view(formula_space),
    }
}

/// Build the editor-foot live-metrics chip. Counts come from the editor
/// document where present (token_count, diagnostic_count) and from the
/// projected syntax runs (function_count = run with role == Function).
/// All zeros when there is no document.
fn project_editor_metrics(
    formula_space: &FormulaSpaceState,
    syntax_runs: &[SyntaxRun],
) -> EditorMetricsChip {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => {
            return EditorMetricsChip {
                token_count: 0,
                function_count: 0,
                diagnostic_count: 0,
            }
        }
    };
    let function_count = syntax_runs
        .iter()
        .filter(|run| run.role == SyntaxTokenRole::Function)
        .count();
    EditorMetricsChip {
        token_count: document.editor_syntax_snapshot.tokens.len(),
        function_count,
        diagnostic_count: document.live_diagnostics.diagnostics.len(),
    }
}

/// Build the result-foot active-context chip. The pre-MVP defaults are
/// `en-US` (locale), `GENERAL` (number-format family) and `deterministic`
/// (scenario policy). Locale and format are tagged SEAM-pending because
/// the cascade machinery they belong to is partly stubbed (WS-14 plan
/// §11); policy is rendered live since the deterministic lane is wired.
fn project_result_context(_formula_space: &FormulaSpaceState) -> ResultContextChip {
    ResultContextChip {
        locale: ContextChipField::SeamPending {
            value: "en-US".to_string(),
            seam_id: "SEAM-OXFUNC-LOCALE-EXPAND".to_string(),
        },
        format: ContextChipField::SeamPending {
            value: "GENERAL".to_string(),
            seam_id: "SEAM-OXFUNC-FORMAT-GENERAL".to_string(),
        },
        policy: ContextChipField::Live("deterministic".to_string()),
    }
}

/// Build the coloured-token runs for the syntax overlay. Returns an empty
/// vector when the editor document is missing, or when its `source_text`
/// does not match the current `raw_entered_cell_text` (a stale snapshot
/// from a prior keystroke); the home shell falls back to uncoloured raw
/// text in that case so the overlay never shows misaligned colours.
fn project_syntax_runs(formula_space: &FormulaSpaceState) -> Vec<SyntaxRun> {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => return Vec::new(),
    };
    if document.source_text != formula_space.raw_entered_cell_text {
        return Vec::new();
    }
    syntax_runs_from_snapshot(&document.editor_syntax_snapshot)
}

/// Build the diagnostic squiggle list. Pulls `LiveDiagnostic`s out of the
/// editor document, sorts them by `span_start`, and prunes any entry whose
/// span overlaps with the previously kept entry — the renderer relies on
/// non-overlapping spans for clean text segmentation. Returns empty when
/// the document is missing or stale, so squiggles never sit at offsets
/// that don't match the textarea contents.
fn project_diagnostic_squiggles(formula_space: &FormulaSpaceState) -> Vec<DiagnosticSquiggle> {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => return Vec::new(),
    };
    if document.source_text != formula_space.raw_entered_cell_text {
        return Vec::new();
    }
    let mut squiggles: Vec<DiagnosticSquiggle> = document
        .live_diagnostics
        .diagnostics
        .iter()
        .map(|diag| DiagnosticSquiggle {
            diagnostic_id: diag.diagnostic_id.clone(),
            message: diag.message.clone(),
            severity: SquiggleSeverity::from_upstream(diag.severity),
            span_start: diag.primary_span.start,
            span_len: diag.primary_span.len,
        })
        .collect();
    squiggles.sort_by_key(|s| s.span_start);
    let mut deduped = Vec::with_capacity(squiggles.len());
    let mut last_end: Option<usize> = None;
    for squiggle in squiggles {
        let start = squiggle.span_start;
        if last_end.is_some_and(|end| start < end) {
            continue;
        }
        last_end = Some(squiggle.span_start.saturating_add(squiggle.span_len));
        deduped.push(squiggle);
    }
    deduped
}

fn project_result_view(formula_space: &FormulaSpaceState) -> ResultView {
    let raw_text = formula_space.raw_entered_cell_text.as_str();

    // Empty editor wins regardless of stale residuals.
    if raw_text.is_empty() {
        return ResultView::Empty;
    }

    // Array result projects to shape only; preview grid is a later phase.
    if let Some(preview) = formula_space.array_preview.as_ref() {
        let rows = preview.rows.len();
        let cols = preview.rows.first().map(|row| row.len()).unwrap_or(0);
        return ResultView::Array {
            rows,
            cols,
            label: preview.label.clone(),
        };
    }

    // Host-derived blocked reason wins regardless of value type.
    if let Some(reason) = formula_space.context.blocked_reason.as_deref() {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason.to_string()),
        };
    }

    // Bridge-side blocked-reason on the editor document (provenance summary
    // populated by the live bridge for capability-denied lanes).
    if let Some(reason) = formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.provenance_summary.as_ref())
        .and_then(|prov| prov.blocked_reason.clone())
    {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason),
        };
    }

    // Live diagnostic on the editor document. The bridge does not produce a
    // typed value when it stops at a parse / bind diagnostic, so the
    // diagnostic itself is the visible result. We surface the first one;
    // the drill-down is responsible for the full list.
    if !has_published_value(formula_space) {
        if let Some(message) = formula_space
            .editor_document
            .as_ref()
            .and_then(|doc| doc.live_diagnostics.diagnostics.first())
            .map(|diag| diag.message.clone())
        {
            return ResultView::Error {
                code: "DIAGNOSTIC".to_string(),
                surface_repr: Some(message),
            };
        }
    }

    // Typed dispatch on the bridge's published `EvalValue`.
    if let Some(published_value) = bridge_published_value(formula_space) {
        return project_typed_value(formula_space, published_value);
    }

    // Pre-bridge hand-evaluation for raw text / number cells: anything that
    // doesn't start with `=` is a literal cell entry. The live bridge
    // doesn't run for these; the home shell evaluates them inline against
    // the raw text. Forced-text cells (`'1.5`) keep the leading apostrophe
    // out of the rendered display.
    if let Some(forced_text) = raw_text.strip_prefix('\'') {
        return ResultView::Display {
            text: forced_text.to_string(),
            kind: ResultKind::Text,
        };
    }
    if !raw_text.starts_with('=') {
        if let Ok(number) = raw_text.parse::<f64>() {
            return ResultView::Display {
                text: format_literal_number(number),
                kind: ResultKind::Number,
            };
        }
        return ResultView::Display {
            text: raw_text.to_string(),
            kind: ResultKind::Text,
        };
    }

    ResultView::Pending
}

fn has_published_value(formula_space: &FormulaSpaceState) -> bool {
    bridge_published_value(formula_space).is_some()
}

fn bridge_published_value(formula_space: &FormulaSpaceState) -> Option<&EvalValue> {
    formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.value_presentation.as_ref())
        .map(|vp| &vp.published_value)
}

fn project_typed_value(formula_space: &FormulaSpaceState, value: &EvalValue) -> ResultView {
    let display_text = || {
        formula_space
            .effective_display_summary
            .clone()
            .unwrap_or_default()
    };
    match value {
        EvalValue::Number(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Number,
        },
        EvalValue::Text(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Text,
        },
        EvalValue::Logical(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Logical,
        },
        EvalValue::Error(code) => ResultView::Error {
            code: worksheet_error_literal(*code).to_string(),
            surface_repr: None,
        },
        EvalValue::Array(_) => {
            // Array path normally goes through `formula_space.array_preview`
            // (handled above). If we reach here without a preview, surface
            // the effective display string.
            ResultView::Display {
                text: display_text(),
                kind: ResultKind::Other,
            }
        }
        EvalValue::Reference(_) | EvalValue::Lambda(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Other,
        },
    }
}

fn format_literal_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn project_status_view(formula_space: &FormulaSpaceState) -> StatusView {
    let truth_source = formula_space.context.truth_source.clone();
    let green_tree_key = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.green_tree_key.clone())
        .filter(|key| !key.is_empty());

    let bridge_health = match (&truth_source, &green_tree_key) {
        (ProjectionTruthSource::LiveBacked, Some(_)) => BridgeHealth::Live,
        _ => BridgeHealth::Stale,
    };

    StatusView {
        bridge_health,
        truth_source,
        green_tree_key,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{FormulaValuePresentation, ProvenanceSummary};
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{FormulaArrayPreviewState, FormulaSpaceState};
    use crate::test_support::{
        array_editor_document, blocked_editor_document, diagnostic_editor_document,
        sample_editor_document,
    };

    /// Build a one-formula-space host state with the given (optional)
    /// editor_document attached. Helper for the test cases below.
    fn host_state_with(formula_space: FormulaSpaceState) -> OneCalcHostState {
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space.formula_space_id.clone());
        state.formula_spaces.insert(formula_space);
        state
    }

    /// Attach a typed Number `value_presentation` to the document so the
    /// home view-model dispatches via the typed path.
    fn attach_number_value_presentation(
        document: &mut crate::adapters::oxfml::EditorDocument,
        number: f64,
        display: &str,
    ) {
        document.value_presentation = Some(FormulaValuePresentation {
            evaluation_summary: format!("Number · {display}"),
            effective_display_summary: Some(display.to_string()),
            array_preview: None,
            blocked_reason: None,
            published_value: EvalValue::Number(number),
        });
    }

    #[test]
    fn returns_none_when_no_active_formula_space() {
        let state = OneCalcHostState::default();
        assert!(build_home_shell_view_model(&state).is_none());
    }

    #[test]
    fn empty_text_projects_to_result_view_empty() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.raw_entered_cell_text, "");
        assert_eq!(vm.result_view, ResultView::Empty);
    }

    #[test]
    fn happy_sum_projects_to_result_view_display_number() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "3");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '3'), got {other:?}"),
        }
    }

    #[test]
    fn diagnostic_in_editor_document_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "DIAGNOSTIC");
                assert_eq!(surface_repr.as_deref(), Some("Missing trailing argument"));
            }
            other => panic!("expected Error(DIAGNOSTIC, ...), got {other:?}"),
        }
    }

    #[test]
    fn host_blocked_reason_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        formula_space.editor_document = Some(blocked_editor_document("=XLOOKUP(...)"));
        formula_space.context.blocked_reason = Some("XLOOKUP not admitted on this host".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("XLOOKUP not admitted on this host")
                );
            }
            other => panic!("expected Error(BLOCKED, ...), got {other:?}"),
        }
    }

    #[test]
    fn bridge_blocked_provenance_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        let mut document = blocked_editor_document("=XLOOKUP(...)");
        // `blocked_editor_document` already sets a provenance blocked reason.
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OxFml blocked lane".to_string(),
            blocked_reason: Some("Excel comparison lane unavailable".to_string()),
        });
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("Excel comparison lane unavailable")
                );
            }
            other => panic!("expected Error(BLOCKED, ...), got {other:?}"),
        }
    }

    #[test]
    fn array_preview_projects_to_result_view_array_with_shape() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Array { rows, cols, label } => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 3);
                assert_eq!(label, "Array[2 × 3]");
            }
            other => panic!("expected Array(2 × 3), got {other:?}"),
        }
    }

    #[test]
    fn pending_text_with_no_summary_projects_to_pending() {
        // `=SU` starts with `=`, so the pre-bridge hand-eval doesn't fire;
        // there's also no published_value or diagnostics → Pending.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_view, ResultView::Pending);
    }

    #[test]
    fn literal_number_input_renders_inline_as_display_number() {
        // `1.5` is a literal number cell entry; the bridge doesn't run for
        // these. The home shell evaluates inline.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "1.5");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "1.5");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '1.5'), got {other:?}"),
        }
    }

    #[test]
    fn literal_text_input_renders_inline_as_display_text() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "hello");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "hello");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, 'hello'), got {other:?}"),
        }
    }

    #[test]
    fn forced_text_input_strips_leading_apostrophe() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "'123");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "123");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, '123'), got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Caption pills
    // -----------------------------------------------------------------

    #[test]
    fn entry_mode_pill_is_empty_for_blank_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Empty);
    }

    #[test]
    fn entry_mode_pill_is_formula_for_leading_equals() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Formula);
    }

    #[test]
    fn entry_mode_pill_is_text_for_leading_apostrophe() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "'42");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Text);
    }

    #[test]
    fn entry_mode_pill_is_value_for_literal_cell_entry() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "42.5");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Value);
    }

    #[test]
    fn result_class_pill_is_none_for_empty_and_pending() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.result_class_pill.is_none());

        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.result_class_pill.is_none());
    }

    #[test]
    fn result_class_pill_is_number_for_number_display() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Number));
    }

    #[test]
    fn result_class_pill_is_error_for_diagnostic_or_blocked() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Error));
    }

    #[test]
    fn result_class_pill_is_array_for_array_result() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Array));
    }

    #[test]
    fn result_class_pill_is_text_for_literal_text_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "hello");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Text));
    }

    #[test]
    fn result_class_pill_is_number_for_literal_number_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "42");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Number));
    }

    // -----------------------------------------------------------------
    // Syntax overlay runs
    // -----------------------------------------------------------------

    #[test]
    fn syntax_runs_empty_without_editor_document() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.syntax_runs.is_empty());
    }

    #[test]
    fn syntax_runs_populated_when_document_matches_raw_text() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(!vm.syntax_runs.is_empty());
        assert_eq!(vm.syntax_runs.first().map(|run| run.text.as_str()), Some("="));
    }

    #[test]
    fn syntax_runs_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document carries a different (older) source text — stale snapshot.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.syntax_runs.is_empty());
    }

    // -----------------------------------------------------------------
    // Diagnostic squiggles
    // -----------------------------------------------------------------

    #[test]
    fn diagnostic_squiggles_empty_without_editor_document() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.diagnostic_squiggles.is_empty());
    }

    #[test]
    fn diagnostic_squiggles_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.diagnostic_squiggles.is_empty());
    }

    #[test]
    fn diagnostic_squiggles_carry_message_severity_and_span() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.diagnostic_squiggles.len(), 1);
        let squiggle = &vm.diagnostic_squiggles[0];
        assert_eq!(squiggle.message, "Missing trailing argument");
        assert_eq!(squiggle.severity, SquiggleSeverity::Error);
        assert!(squiggle.span_len >= 1);
    }

    #[test]
    fn diagnostic_squiggles_sort_and_dedup_overlaps() {
        use crate::adapters::oxfml::LiveDiagnosticSnapshot;
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        let mut document = sample_editor_document("=SUM(1,2,3)");
        document.live_diagnostics = LiveDiagnosticSnapshot {
            formula_stable_id: "f1".into(),
            formula_token: "f1".into(),
            diagnostics: vec![
                // Out of order: late then early.
                make_diag("d-late", "later", 8, 2, LiveDiagnosticSeverity::Warning),
                make_diag("d-early", "earlier", 1, 3, LiveDiagnosticSeverity::Error),
                // Overlaps with d-early — should be dropped.
                make_diag("d-overlap", "overlap", 2, 2, LiveDiagnosticSeverity::Error),
            ],
        };
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        // Sorted by span_start ascending; overlap dropped, leaving 2.
        assert_eq!(vm.diagnostic_squiggles.len(), 2);
        assert_eq!(vm.diagnostic_squiggles[0].diagnostic_id, "d-early");
        assert_eq!(vm.diagnostic_squiggles[0].severity, SquiggleSeverity::Error);
        assert_eq!(vm.diagnostic_squiggles[1].diagnostic_id, "d-late");
        assert_eq!(vm.diagnostic_squiggles[1].severity, SquiggleSeverity::Warning);
    }

    fn make_diag(
        id: &str,
        message: &str,
        start: usize,
        len: usize,
        severity: LiveDiagnosticSeverity,
    ) -> crate::adapters::oxfml::LiveDiagnostic {
        use crate::adapters::oxfml::{FormulaTextSpan, LiveDiagnostic, LiveDiagnosticStage};
        LiveDiagnostic {
            diagnostic_id: id.to_string(),
            severity,
            stage: LiveDiagnosticStage::Bind,
            message: message.to_string(),
            primary_span: FormulaTextSpan { start, len },
            related_spans: Vec::new(),
            code: None,
            suggested_fix_kind: None,
        }
    }

    // -----------------------------------------------------------------
    // Foot chips
    // -----------------------------------------------------------------

    #[test]
    fn editor_metrics_zero_without_document() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.editor_metrics.token_count, 0);
        assert_eq!(vm.editor_metrics.function_count, 0);
        assert_eq!(vm.editor_metrics.diagnostic_count, 0);
    }

    #[test]
    fn editor_metrics_count_tokens_functions_and_diagnostics() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        // sample_editor_document for "=SUM(1,2)" emits 7 tokens, has 1
        // diagnostic ('sample diagnostic'), and SUM is a function token.
        assert_eq!(vm.editor_metrics.token_count, 7);
        assert!(vm.editor_metrics.function_count >= 1);
        assert_eq!(vm.editor_metrics.diagnostic_count, 1);
    }

    #[test]
    fn result_context_defaults_to_seam_pending_locale_and_format_with_live_policy() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_context.locale.value(), "en-US");
        assert_eq!(
            vm.result_context.locale.seam_id(),
            Some("SEAM-OXFUNC-LOCALE-EXPAND")
        );
        assert_eq!(vm.result_context.format.value(), "GENERAL");
        assert_eq!(
            vm.result_context.format.seam_id(),
            Some("SEAM-OXFUNC-FORMAT-GENERAL")
        );
        assert_eq!(vm.result_context.policy.value(), "deterministic");
        assert_eq!(vm.result_context.policy.seam_id(), None);
    }

    // -----------------------------------------------------------------
    // Status foot
    // -----------------------------------------------------------------

    #[test]
    fn status_live_when_live_backed_with_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Live);
        assert_eq!(vm.status.green_tree_key.as_deref(), Some("green-1"));
    }

    #[test]
    fn status_stale_when_local_fallback() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LocalFallback;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
    }

    #[test]
    fn status_stale_when_live_backed_but_no_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // LiveBacked but no editor_document => no green-tree key => stale.
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
        assert!(vm.status.green_tree_key.is_none());
    }
}
