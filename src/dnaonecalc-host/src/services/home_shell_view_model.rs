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
use crate::services::completion_popup::{CompletionPopupKind, CompletionPopupState};
use crate::state::{FormulaSpaceState, OneCalcHostState, ProjectionTruthSource, ViewMode};
use crate::ui::editor::geometry::caret_box_for_offset;
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
    /// Completion popup overlay. `None` when the popup is `Hidden` OR
    /// when the editor-box metrics have not yet been measured (the
    /// browser adapter populates them on the first input event; until
    /// then the popup cannot be positioned and is suppressed).
    pub completion_popup: Option<CompletionPopupView>,
    /// Signature-help line rendered ABOVE the caret while the caret
    /// sits inside an open function call. `None` when:
    ///   * the editor document does not carry a `signature_help` from
    ///     the bridge,
    ///   * the document is stale (its `source_text` does not match
    ///     `raw_entered_cell_text`), or
    ///   * the completion popup is already `Open` at the same caret
    ///     (popup wins to avoid double-stacking; signature help
    ///     re-appears the moment the popup dismisses).
    pub signature_help: Option<SignatureHelpView>,
    /// Function-help card rendered as a hover tooltip on the matching
    /// function-token in the syntax overlay. `None` when:
    ///   * the editor document does not carry a `function_help` from
    ///     the bridge (no function context for the current caret), or
    ///   * the document is stale.
    /// Visibility is gated by component-local hover state — the
    /// view-model only carries the *content*; the actual tooltip is
    /// shown after a 400 ms hover over the matching `.syn-fn` span.
    pub function_help_card: Option<FunctionHelpCardView>,
    /// First progressive-disclosure drill-down: the formula
    /// walk-tree panel rendered between the editor-foot and the
    /// result-caption when the user toggles it open with Ctrl+D.
    /// Always present (so the toggle row is rendered consistently
    /// whether the panel is open or closed); the `expanded` flag
    /// drives whether the panel body is visible.
    pub formula_drill: FormulaDrillView,
    /// Workspace-level reading-audience preference. Components
    /// branch their rendering based on this — User mode hides
    /// phase chips, state slugs, and SEAM markers; Developer mode
    /// surfaces them.
    pub view_mode: ViewMode,
    pub result_view: ResultView,
    pub status: StatusView,
    /// Titlebar scenario breadcrumb + dropdown projection. The
    /// breadcrumb is always rendered when there is an active
    /// formula space; the dropdown's `is_open` flag drives
    /// visibility of the menu body.
    pub scenario_breadcrumb: ScenarioBreadcrumbView,
}

/// View-model shape for the titlebar scenario breadcrumb + its
/// dropdown menu. Built from the active formula space (for the
/// label + dirty marker) and from `WorkspaceShellState` for the
/// Recent / Pinned lists. Actions are a fixed shape — handlers
/// are wired in the renderer.
///
/// Phase A: the dropdown surfaces the data already living in
/// state. File I/O for `.dnascenario` is out of scope here and
/// will land behind `SEAM-ONECALC-SCENARIO-PERSIST`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbView {
    /// Human-readable breadcrumb label. `"unsaved"` when the
    /// scenario has not been named (the default `scenario_label`
    /// equals the synthetic `formula_space_id.as_str()`); the
    /// stored label otherwise. Renderer puts this inside the
    /// breadcrumb button next to the dropdown caret.
    pub active_label: String,
    /// True when the live editor text has uncommitted changes
    /// (live state is `EditingLive`). Renderer surfaces this
    /// with a small dot before the label per the WS-14 mockup
    /// (`data-dirty="true"` selector).
    pub is_dirty: bool,
    /// Lifecycle for the dropdown menu. Mirrors
    /// `global_ui_chrome.scenario_breadcrumb_open`.
    pub is_open: bool,
    /// Top-N most recently active formula spaces. Includes the
    /// active formula (first entry) followed by the workspace
    /// shell's `recent_formula_space_order`, deduplicated. Empty
    /// when there is no active space and no recents.
    pub recent: Vec<ScenarioBreadcrumbEntry>,
    /// Pinned formula spaces, in stable id order from the
    /// underlying `BTreeSet`. Empty when no pins exist.
    pub pinned: Vec<ScenarioBreadcrumbEntry>,
    /// Fixed action list rendered under the dropdown's
    /// `Actions` heading. Each action has a stable identifier
    /// the renderer dispatches on.
    pub actions: Vec<ScenarioBreadcrumbAction>,
}

/// One row in the Recent or Pinned section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbEntry {
    pub formula_space_id: String,
    pub display_name: String,
    /// One-line meta string rendered to the right of the name.
    /// Today: `"active"` for the active row, `"pinned"` for
    /// pinned rows, otherwise the recent entry's last-active
    /// mode label. Future bead: timestamp once persistence
    /// lands.
    pub meta: String,
    pub is_active: bool,
    pub is_pinned: bool,
}

/// Stable identifiers for the dropdown's Actions section. The
/// renderer maps each to a click handler; today most are no-ops
/// gated behind `SEAM-ONECALC-SCENARIO-PERSIST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioBreadcrumbActionId {
    NewScenario,
    SaveAs,
    Open,
    Duplicate,
    ManageScenarios,
}

impl ScenarioBreadcrumbActionId {
    pub fn slug(self) -> &'static str {
        match self {
            Self::NewScenario => "new-scenario",
            Self::SaveAs => "save-as",
            Self::Open => "open",
            Self::Duplicate => "duplicate",
            Self::ManageScenarios => "manage-scenarios",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbAction {
    pub action_id: ScenarioBreadcrumbActionId,
    pub label: &'static str,
    /// Keyboard chord rendered to the right of the action label
    /// in the dropdown. Empty string when no chord is bound today.
    pub chord_label: &'static str,
    /// Optional SEAM identifier surfaced in the renderer's
    /// `aria-describedby` so unimplemented actions are honest
    /// about the missing backend. `None` for actions that work
    /// fully today (just opening the dropdown / etc.).
    pub seam_id: Option<&'static str>,
}

/// View-model shape for the formula walk-tree drill-down. Always
/// emitted — the toggle row in the editor-foot needs to render
/// regardless of expansion state, so the `expanded` flag drives
/// visibility of the panel body and the chevron rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillView {
    pub expanded: bool,
    /// Walk tree from `editor_document.formula_walk`, projected
    /// into render-friendly nodes. Empty when the document is
    /// missing or stale.
    pub tree: Vec<FormulaDrillNode>,
    /// Bottom-strip phase chips: `parse: <status> · bind: <vars>
    /// vars · eval: <steps> steps`. Pulled from the document's
    /// parse_summary / bind_summary / eval_summary fields. Empty
    /// when the document is missing or stale.
    pub phase_summaries: Vec<FormulaDrillPhaseChip>,
    /// True iff the document is present and matches
    /// `raw_entered_cell_text`. The component shows a "(loading)"
    /// indicator when `expanded` is true but `document_is_fresh`
    /// is false — gives the user feedback during the brief stale
    /// window between keystroke and bridge round-trip.
    pub document_is_fresh: bool,
}

/// One row in the formula walk-tree panel. Mirrors
/// [`crate::adapters::oxfml::FormulaWalkNode`] but flattened by
/// the projector so the component renders without recursion
/// helpers — `depth` carries the indent level and the projector
/// emits parent rows before child rows in the order the user
/// reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillNode {
    pub node_id: String,
    pub label: String,
    pub value_preview: Option<String>,
    pub state: crate::adapters::oxfml::FormulaWalkNodeState,
    pub depth: usize,
    pub has_children: bool,
}

/// Phase-strip chip. Renders `label · detail` with a state
/// attribute (`ok` / `pending` / `blocked`) so the corpus can
/// pin the colour and content separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillPhaseChip {
    pub label: &'static str,
    pub detail: String,
    pub state: FormulaDrillPhaseState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaDrillPhaseState {
    Ok,
    Pending,
    Blocked,
}

impl FormulaDrillPhaseState {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
        }
    }
}

/// View-model shape for the function-help hover tooltip.
///
/// Sourced from `editor_document.function_help: FunctionHelpPacket`.
/// The bridge populates this packet for the caret-adjacent function;
/// hover help on an arbitrary token in the formula would require a
/// separate bridge call and is deferred to a future bead. For now,
/// the hover only fires when the user hovers over a function token
/// whose name matches `lookup_key` (case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHelpCardView {
    /// The bridge's lookup key for the function (uppercase canonical
    /// name, e.g. "SUM"). The renderer uses this to gate which
    /// `.syn-fn` span can trigger the tooltip.
    pub lookup_key: String,
    /// Display name for the heading line (typically the same as
    /// `lookup_key` but may differ for localised function catalogues).
    pub display_name: String,
    /// First signature form's `display_signature`, e.g.
    /// `SUM(number1, number2, ...)`. Multi-form / overload navigation
    /// is a future bead — first form only here.
    pub signature: Option<String>,
    /// One-line description from the function-help packet. Optional
    /// because not every catalogue entry carries a description.
    pub short_description: Option<String>,
    /// Availability summary from the packet, surfaced when present so
    /// users can see why a deferred / profile-limited function might
    /// not be evaluating.
    pub availability_summary: Option<String>,
    /// True when the function-help packet flags the function as
    /// deferred or restricted by the active capability profile. The
    /// renderer styles this state so the user knows the help is for
    /// a function that won't fully evaluate today.
    pub deferred_or_profile_limited: bool,
}

/// View-model shape for the signature-help line. Mirrors the
/// completion-popup geometry primitives so the renderer uses one
/// shared positioning convention; the difference is purely that
/// the help line sits ABOVE the caret instead of below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpView {
    /// Display text of the function being called (e.g. `SUM`). Comes
    /// straight from the upstream `SignatureHelpContext`.
    pub callee_text: String,
    /// Pixel anchor of the caret-box top-left, relative to the editor
    /// frame's origin. The renderer offsets `top_px` UPWARD by the
    /// signature-help line's own height so the help sits above the
    /// line the caret occupies, not on top of it.
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    /// Caret line height in pixels — used for the BELOW-caret fallback
    /// when the help line would clip the top of the editor frame.
    pub line_height_px: usize,
    /// Parameter list rendered with the active argument bolded. Built
    /// from `function_help.argument_help`; an empty vec is rendered
    /// as just the callee name with bare parens.
    pub parameters: Vec<SignatureHelpParameter>,
    /// Active-parameter index, clamped to `parameters.len()`. `None`
    /// when the bridge's index is out of range (caret is past the
    /// last parameter, e.g. one extra trailing comma) — the renderer
    /// shows the parameter list with no bolded entry in that case.
    pub active_parameter: Option<usize>,
}

/// One parameter in the signature-help line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpParameter {
    pub name: String,
    pub is_active: bool,
}

/// View-model shape for the completion popup. Carries the anchor pixel
/// position computed from the bridge's caret offset + the browser-
/// measured char-box metrics, plus the rendered item list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopupView {
    /// Pixel anchor of the popup's top-left corner, relative to the
    /// editor frame's origin. The popup itself sits below the line the
    /// caret occupies, so the renderer offsets `top_px` by
    /// `line_height_px` when positioning.
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    pub line_height_px: usize,
    pub items: Vec<CompletionPopupItemView>,
    /// Index into `items` of the row to highlight. Always in
    /// `0..items.len()`.
    pub selected_index: usize,
}

/// One row of the popup. Carries the rendering payload (display text,
/// kind glyph, is_selected) and the proposal id so click handlers can
/// look up the original proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopupItemView {
    pub proposal_id: String,
    pub display_text: String,
    pub kind_glyph: char,
    pub kind_label: &'static str,
    pub is_selected: bool,
    pub documentation_ref: Option<String>,
}

impl CompletionPopupItemView {
    fn glyph_for_kind(kind: CompletionPopupKind) -> char {
        match kind {
            CompletionPopupKind::Function => 'ƒ',
            CompletionPopupKind::DefinedName => 'N',
            CompletionPopupKind::TableName => 'T',
            CompletionPopupKind::TableColumn => '⫶',
            CompletionPopupKind::StructuredSelector => '#',
            CompletionPopupKind::SyntaxAssist => '·',
        }
    }

    fn label_for_kind(kind: CompletionPopupKind) -> &'static str {
        match kind {
            CompletionPopupKind::Function => "Function",
            CompletionPopupKind::DefinedName => "Defined name",
            CompletionPopupKind::TableName => "Table",
            CompletionPopupKind::TableColumn => "Column",
            CompletionPopupKind::StructuredSelector => "Selector",
            CompletionPopupKind::SyntaxAssist => "Syntax",
        }
    }
}

/// Editor-foot live-metrics chip projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMetricsChip {
    pub token_count: usize,
    pub function_count: usize,
    pub diagnostic_count: usize,
    /// First diagnostic's user-facing message, when one exists.
    /// Surfaced on the User-mode editor-foot chip so the user sees
    /// "1 issue: unmatched '('" instead of the developer-mode raw
    /// counts. None when `diagnostic_count == 0`.
    pub first_diagnostic_message: Option<String>,
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
///
/// Per OxFml W067 (`docs/upstream/NOTES_FOR_DNAONECALC.md` §10), this
/// view-model passes through three additional upstream-owned fields:
///
/// * `code` — stable classification key (`unknown_function`,
///   `unknown_name`, `function_arity_mismatch`,
///   `known_symbol_not_callable`, `function_gated_or_unavailable`,
///   etc.). Used by the test corpus and by the eventual UI grouping
///   surface; never inferred host-side.
/// * `stage` — `Syntax` / `Bind` / `SemanticPlan`, the upstream phase
///   that emitted the diagnostic. Surfaced as a data attribute so the
///   corpus and X-Ray can read it without collapsing the distinction.
/// * `worksheet_error_class` — the worksheet-visible error consequence
///   when OxFml already knows it (e.g. `"#NAME?"`). Lets the UI render
///   the squiggle alongside the same error glyph Excel would show; the
///   host never derives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSquiggle {
    pub diagnostic_id: String,
    pub message: String,
    pub severity: SquiggleSeverity,
    pub stage: DiagnosticStage,
    pub code: Option<String>,
    pub worksheet_error_class: Option<String>,
    pub span_start: usize,
    pub span_len: usize,
}

/// Mirror of `LiveDiagnosticStage` lifted to the view-model layer so
/// the renderer doesn't depend on the upstream enum directly. Three
/// values: `Syntax`, `Bind`, `SemanticPlan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStage {
    Syntax,
    Bind,
    SemanticPlan,
}

impl DiagnosticStage {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Bind => "bind",
            Self::SemanticPlan => "semantic-plan",
        }
    }

    fn from_upstream(stage: crate::adapters::oxfml::LiveDiagnosticStage) -> Self {
        match stage {
            crate::adapters::oxfml::LiveDiagnosticStage::Syntax => Self::Syntax,
            crate::adapters::oxfml::LiveDiagnosticStage::Bind => Self::Bind,
            crate::adapters::oxfml::LiveDiagnosticStage::SemanticPlan => Self::SemanticPlan,
        }
    }
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
    /// Scenario label rendered in the status-foot's centre band
    /// (`scenario · <label>`). Same fallback rule as the
    /// breadcrumb: synthetic-default labels render as `"unsaved"`
    /// rather than leaking the synthetic id. Always populated when
    /// there is an active formula space; the renderer always shows
    /// the chip so the user always knows which scenario they are
    /// editing.
    pub scenario_label: String,
    /// Persistence-loader warnings for the active formula space.
    /// Empty when the file was loaded with full fidelity (or no
    /// file has been loaded). Slice 3 of the persistence ladder
    /// surfaces a warning chip while this is non-empty; the next
    /// save clears it.
    pub load_diagnostics: Vec<crate::persistence::LoadDiagnostic>,
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
    Some(project_formula_space(formula_space, state))
}

fn project_formula_space(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
) -> HomeShellViewModel {
    let view_mode = state.view_mode;
    let result_view = project_result_view(formula_space);
    let entry_mode_pill = EntryModePill::from_entry_mode(EditorEntryMode::classify(
        &formula_space.raw_entered_cell_text,
    ));
    let result_class_pill = ResultClassPill::from_result_view(&result_view);
    let syntax_runs = project_syntax_runs(formula_space);
    let diagnostic_squiggles = project_diagnostic_squiggles(formula_space);
    let editor_metrics = project_editor_metrics(formula_space, &syntax_runs);
    let result_context = project_result_context(formula_space);
    let completion_popup = project_completion_popup(formula_space);
    let signature_help = project_signature_help(formula_space, completion_popup.is_some());
    let function_help_card = project_function_help_card(formula_space);
    let formula_drill = project_formula_drill(formula_space);
    let scenario_breadcrumb = project_scenario_breadcrumb(formula_space, state);
    HomeShellViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        editor_surface_state: formula_space.editor_surface_state.clone(),
        entry_mode_pill,
        result_class_pill,
        syntax_runs,
        diagnostic_squiggles,
        editor_metrics,
        result_context,
        completion_popup,
        signature_help,
        function_help_card,
        formula_drill,
        view_mode,
        result_view,
        status: project_status_view(formula_space),
        scenario_breadcrumb,
    }
}

/// Project the formula walk tree + phase summaries into the
/// drill-down view model. Always returns a `FormulaDrillView` —
/// the `expanded` flag follows the formula space's
/// `formula_drill_open`, and the `tree` / `phase_summaries`
/// vectors are empty when the document is missing or stale.
///
/// `document_is_fresh` lets the component distinguish "panel
/// open but bridge round-trip pending" (show a loading state)
/// from "panel open and tree ready".
fn project_formula_drill(formula_space: &FormulaSpaceState) -> FormulaDrillView {
    let document = formula_space.editor_document.as_ref();
    let document_is_fresh = document
        .map(|doc| doc.source_text == formula_space.raw_entered_cell_text)
        .unwrap_or(false);

    let mut tree = Vec::new();
    if document_is_fresh {
        if let Some(document) = document {
            for node in &document.formula_walk {
                flatten_walk_node(node, 0, &mut tree);
            }
        }
    }

    let mut phase_summaries = Vec::new();
    if document_is_fresh {
        if let Some(document) = document {
            if let Some(parse) = &document.parse_summary {
                let state = if parse.status == "Valid" {
                    FormulaDrillPhaseState::Ok
                } else {
                    FormulaDrillPhaseState::Pending
                };
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "parse",
                    detail: format!("{} ({} tokens)", parse.status, parse.token_count),
                    state,
                });
            }
            if let Some(bind) = &document.bind_summary {
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "bind",
                    detail: format!(
                        "{} vars · {} refs",
                        bind.variable_count, bind.reference_count
                    ),
                    state: FormulaDrillPhaseState::Ok,
                });
            }
            if let Some(eval) = &document.eval_summary {
                let blocked = document
                    .provenance_summary
                    .as_ref()
                    .and_then(|p| p.blocked_reason.as_ref())
                    .is_some();
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "eval",
                    detail: format!("{} step{} · {}", eval.step_count,
                        if eval.step_count == 1 { "" } else { "s" }, eval.duration_text),
                    state: if blocked {
                        FormulaDrillPhaseState::Blocked
                    } else {
                        FormulaDrillPhaseState::Ok
                    },
                });
            }
        }
    }

    FormulaDrillView {
        expanded: formula_space.formula_drill_open,
        tree,
        phase_summaries,
        document_is_fresh,
    }
}

/// Flatten a `FormulaWalkNode` recursively in pre-order (parent
/// before children) into the target vec. `depth` is the indent
/// level; root nodes have depth 0.
fn flatten_walk_node(
    node: &crate::adapters::oxfml::FormulaWalkNode,
    depth: usize,
    out: &mut Vec<FormulaDrillNode>,
) {
    out.push(FormulaDrillNode {
        node_id: node.node_id.clone(),
        label: node.label.clone(),
        value_preview: node.value_preview.clone(),
        state: node.state,
        depth,
        has_children: !node.children.is_empty(),
    });
    for child in &node.children {
        flatten_walk_node(child, depth + 1, out);
    }
}

/// Project the bridge's `FunctionHelpPacket` into the view-model
/// shape consumed by the hover-help tooltip. Returns `None` when:
///   * the editor document is missing or stale, or
///   * the bridge did not produce a function_help (no function
///     context for the current caret position).
///
/// The component decides WHEN to render the tooltip (based on hover
/// state); the view-model only carries the *content* and the
/// `lookup_key` that gates which `.syn-fn` span is eligible.
fn project_function_help_card(
    formula_space: &FormulaSpaceState,
) -> Option<FunctionHelpCardView> {
    let document = formula_space.editor_document.as_ref()?;
    if document.source_text != formula_space.raw_entered_cell_text {
        return None;
    }
    let packet = document.function_help.as_ref()?;
    let signature = packet
        .signature_forms
        .first()
        .map(|form| form.display_signature.clone());
    Some(FunctionHelpCardView {
        lookup_key: packet.lookup_key.clone(),
        display_name: packet.display_name.clone(),
        signature,
        short_description: packet.short_description.clone(),
        availability_summary: packet.availability_summary.clone(),
        deferred_or_profile_limited: packet.deferred_or_profile_limited,
    })
}

/// Project the bridge's signature-help context into the home shell's
/// renderable view-model.
///
/// Returns `None` when:
///   * the editor document is missing or stale (`source_text !=
///     raw_entered_cell_text`),
///   * the bridge did not produce a `signature_help` for the current
///     caret position (the user is not inside an open function call),
///   * the caret-box metrics have not yet been measured (without
///     metrics the anchor cannot be placed; same gate as the
///     completion popup), or
///   * the completion popup is already `Open` at the same caret —
///     popup wins to avoid stacking two overlays at the same spot.
///
/// The parameter list is sourced from the matching function-help
/// packet's `argument_help` rather than parsing the formatted
/// `signature_form.display_signature` string. If the function-help
/// packet is missing, fall back to a single empty parameter list so
/// the callee name still renders.
fn project_signature_help(
    formula_space: &FormulaSpaceState,
    completion_popup_open: bool,
) -> Option<SignatureHelpView> {
    if completion_popup_open {
        return None;
    }

    let document = formula_space.editor_document.as_ref()?;
    if document.source_text != formula_space.raw_entered_cell_text {
        return None;
    }

    let signature_help_context = document.signature_help.as_ref()?;
    let metrics = formula_space.editor_box_metrics?;

    let parameters: Vec<SignatureHelpParameter> = document
        .function_help
        .as_ref()
        .map(|packet| {
            packet
                .argument_help
                .iter()
                .enumerate()
                .map(|(index, name)| SignatureHelpParameter {
                    name: name.clone(),
                    is_active: index == signature_help_context.active_argument_index,
                })
                .collect()
        })
        .unwrap_or_default();

    let active_parameter = if signature_help_context.active_argument_index < parameters.len() {
        Some(signature_help_context.active_argument_index)
    } else {
        None
    };

    let caret_offset = formula_space.editor_surface_state.caret.offset;
    let anchor = caret_box_for_offset(
        &formula_space.raw_entered_cell_text,
        caret_offset,
        metrics,
    );

    Some(SignatureHelpView {
        callee_text: signature_help_context.callee_text.clone(),
        anchor_left_px: anchor.left_px,
        anchor_top_px: anchor.top_px,
        line_height_px: metrics.line_height_px.max(1),
        parameters,
        active_parameter,
    })
}

/// Project the completion popup state into a renderable view-model.
/// Returns `None` when:
///   * the popup is in `Hidden` state, or
///   * `editor_box_metrics` is `None` (the browser adapter has not yet
///     measured the textarea — without metrics the anchor cannot be
///     placed, so the popup is suppressed for one frame).
///
/// When both gates pass, the anchor is computed via
/// [`caret_box_for_offset`] from the popup's `anchor_offset` (which the
/// reducer sourced from the proposal's `replacement_span.start` or the
/// caret offset). Each item maps to a `CompletionPopupItemView` with
/// `is_selected` set on the popup's `selected_index`.
fn project_completion_popup(formula_space: &FormulaSpaceState) -> Option<CompletionPopupView> {
    let CompletionPopupState::Open {
        anchor_offset,
        items,
        selected_index,
    } = &formula_space.completion_popup
    else {
        return None;
    };
    let metrics = formula_space.editor_box_metrics?;
    let anchor = caret_box_for_offset(
        &formula_space.raw_entered_cell_text,
        *anchor_offset,
        metrics,
    );
    let item_views: Vec<CompletionPopupItemView> = items
        .iter()
        .enumerate()
        .map(|(index, item)| CompletionPopupItemView {
            proposal_id: item.proposal_id.clone(),
            display_text: item.display_text.clone(),
            kind_glyph: CompletionPopupItemView::glyph_for_kind(item.kind),
            kind_label: CompletionPopupItemView::label_for_kind(item.kind),
            is_selected: index == *selected_index,
            documentation_ref: item.documentation_ref.clone(),
        })
        .collect();
    Some(CompletionPopupView {
        anchor_left_px: anchor.left_px,
        anchor_top_px: anchor.top_px,
        line_height_px: metrics.line_height_px.max(1),
        items: item_views,
        selected_index: *selected_index,
    })
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
                first_diagnostic_message: None,
            }
        }
    };
    let function_count = syntax_runs
        .iter()
        .filter(|run| run.role == SyntaxTokenRole::Function)
        .count();
    let first_diagnostic_message = document
        .live_diagnostics
        .diagnostics
        .first()
        .map(|diagnostic| diagnostic.message.clone());
    EditorMetricsChip {
        token_count: document.editor_syntax_snapshot.tokens.len(),
        function_count,
        diagnostic_count: document.live_diagnostics.diagnostics.len(),
        first_diagnostic_message,
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
            stage: DiagnosticStage::from_upstream(diag.stage),
            code: diag.code.clone(),
            worksheet_error_class: diag.worksheet_error_class.clone(),
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

/// Project the active scenario's titlebar breadcrumb + dropdown.
///
/// The label falls back to `"unsaved"` when the scenario carries
/// the synthetic default label (the `formula_space_id.as_str()`
/// auto-set by `FormulaSpaceState::new`); a user-named scenario
/// shows its name verbatim. The dirty marker tracks live edits
/// against the last commit point. Recent always lists the active
/// scenario first; the workspace's `recent_formula_space_order`
/// follows, deduplicated against the active id. Pinned reads
/// from the workspace's stable id-ordered set.
///
/// Actions are a fixed list. Most carry a `SEAM-ONECALC-SCENARIO-PERSIST`
/// id today — the dropdown surfaces them as honest stubs until
/// file I/O lands. `NewScenario` is wired to a real reducer
/// helper so the slice is not entirely no-op.
fn project_scenario_breadcrumb(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
) -> ScenarioBreadcrumbView {
    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let active_label = if synthetic_default_label {
        "unsaved".to_string()
    } else {
        formula_space.context.scenario_label.clone()
    };
    let is_dirty = matches!(
        formula_space.live_state(),
        crate::ui::editor::state::EditorLiveState::EditingLive
            | crate::ui::editor::state::EditorLiveState::ProofedScratch
    );
    let active_entry = ScenarioBreadcrumbEntry {
        formula_space_id: formula_space.formula_space_id.as_str().to_string(),
        display_name: active_label.clone(),
        meta: "active".to_string(),
        is_active: true,
        is_pinned: state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&formula_space.formula_space_id),
    };
    let active_id = formula_space.formula_space_id.clone();
    let mut recent = vec![active_entry];
    for recent_id in &state.workspace_shell.recent_formula_space_order {
        if recent_id == &active_id {
            continue;
        }
        let display_name = state
            .workspace_shell
            .recent_formula_spaces
            .get(recent_id)
            .map(|record| record.formula_space.context.scenario_label.clone())
            .unwrap_or_else(|| recent_id.as_str().to_string());
        let synthetic_default = display_name == recent_id.as_str();
        recent.push(ScenarioBreadcrumbEntry {
            formula_space_id: recent_id.as_str().to_string(),
            display_name: if synthetic_default {
                "unsaved".to_string()
            } else {
                display_name
            },
            meta: "recent".to_string(),
            is_active: false,
            is_pinned: state
                .workspace_shell
                .pinned_formula_space_ids
                .contains(recent_id),
        });
        if recent.len() >= 5 {
            break;
        }
    }
    let pinned: Vec<ScenarioBreadcrumbEntry> = state
        .workspace_shell
        .pinned_formula_space_ids
        .iter()
        .map(|pinned_id| {
            let display_name = state
                .formula_spaces
                .get(pinned_id)
                .map(|space| space.context.scenario_label.clone())
                .or_else(|| {
                    state
                        .workspace_shell
                        .recent_formula_spaces
                        .get(pinned_id)
                        .map(|record| record.formula_space.context.scenario_label.clone())
                })
                .unwrap_or_else(|| pinned_id.as_str().to_string());
            let synthetic_default = display_name == pinned_id.as_str();
            ScenarioBreadcrumbEntry {
                formula_space_id: pinned_id.as_str().to_string(),
                display_name: if synthetic_default {
                    "unsaved".to_string()
                } else {
                    display_name
                },
                meta: "pinned".to_string(),
                is_active: pinned_id == &active_id,
                is_pinned: true,
            }
        })
        .collect();

    // User-facing labels say "formula" (per docs/APP_UX_BRIEF.md
    // §1A); internal type / action-id slugs continue to say
    // `scenario`.
    //
    // Seam policy: NewScenario / Duplicate use existing in-memory
    // reducers; SaveAs and Open are wired through the browser-host
    // download / file-input adapter (slice 1b — see
    // `persistence/browser_file_io.rs`). ManageScenarios still
    // pends its full-screen page UI, hence the SEAM. Tauri-host
    // file IO is a separate later slice; the per-host adapter
    // lives below the action layer, so the breadcrumb sees the
    // same dispatch surface either way.
    let actions = vec![
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::NewScenario,
            label: "New formula",
            chord_label: "Ctrl+N",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::SaveAs,
            label: "Save as…",
            chord_label: "Ctrl+Shift+S",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::Open,
            label: "Open…",
            chord_label: "Ctrl+O",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::Duplicate,
            label: "Duplicate",
            chord_label: "",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::ManageScenarios,
            label: "Manage formulas…",
            chord_label: "",
            seam_id: Some("SEAM-ONECALC-SCENARIO-PERSIST"),
        },
    ];

    ScenarioBreadcrumbView {
        active_label,
        is_dirty,
        is_open: state.global_ui_chrome.scenario_breadcrumb_open,
        recent,
        pinned,
        actions,
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

    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let scenario_label = if synthetic_default_label {
        "unsaved".to_string()
    } else {
        formula_space.context.scenario_label.clone()
    };

    StatusView {
        bridge_health,
        truth_source,
        green_tree_key,
        scenario_label,
        load_diagnostics: formula_space.load_diagnostics.clone(),
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
    use crate::state::{
        AppMode, ClosedFormulaSpaceRecord, FormulaArrayPreviewState, FormulaSpaceState,
    };
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
            worksheet_error_class: None,
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
    // Completion popup view-model projection (bead dno-xcq.24)
    // -----------------------------------------------------------------

    fn open_popup_state() -> CompletionPopupState {
        use crate::adapters::oxfml::FormulaTextSpan;
        use crate::services::completion_popup::{CompletionPopupItem, CompletionPopupKind};
        CompletionPopupState::Open {
            anchor_offset: 1,
            items: vec![
                CompletionPopupItem {
                    proposal_id: "p-1".to_string(),
                    display_text: "SUM".to_string(),
                    insert_text: "SUM(".to_string(),
                    kind: CompletionPopupKind::Function,
                    replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                    documentation_ref: Some("doc:sum".to_string()),
                },
                CompletionPopupItem {
                    proposal_id: "p-2".to_string(),
                    display_text: "SUMIF".to_string(),
                    insert_text: "SUMIF(".to_string(),
                    kind: CompletionPopupKind::Function,
                    replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                    documentation_ref: None,
                },
            ],
            selected_index: 1,
        }
    }

    fn synthetic_metrics() -> crate::adapters::oxfml::FormulaTextSpan {
        // Returning a span isn't quite right; replace below.
        crate::adapters::oxfml::FormulaTextSpan { start: 0, len: 0 }
    }

    #[test]
    fn completion_popup_view_is_none_when_state_hidden() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        // popup defaults to Hidden, even when metrics are populated.
        let mut formula_space = formula_space;
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.completion_popup.is_none());
        let _ = synthetic_metrics(); // silence unused-helper warning
    }

    #[test]
    fn completion_popup_view_is_none_when_metrics_unmeasured() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        // Metrics deliberately None — adapter hasn't run yet.
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(
            vm.completion_popup.is_none(),
            "popup view suppressed until measurement lands",
        );
    }

    #[test]
    fn completion_popup_view_is_some_when_open_and_measured() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        // Anchor at offset 1 with char_width 9 -> left = 9.
        assert_eq!(popup.anchor_left_px, 9);
        assert_eq!(popup.anchor_top_px, 0);
        assert_eq!(popup.line_height_px, 22);
        assert_eq!(popup.items.len(), 2);
        assert_eq!(popup.selected_index, 1);
    }

    #[test]
    fn completion_popup_view_marks_selected_item_only() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        assert_eq!(popup.items[0].is_selected, false);
        assert_eq!(popup.items[1].is_selected, true);
    }

    #[test]
    fn completion_popup_view_kind_glyph_and_label_cover_all_variants() {
        use crate::services::completion_popup::CompletionPopupKind as Kind;
        for (kind, expected_glyph, expected_label) in [
            (Kind::Function, 'ƒ', "Function"),
            (Kind::DefinedName, 'N', "Defined name"),
            (Kind::TableName, 'T', "Table"),
            (Kind::TableColumn, '⫶', "Column"),
            (Kind::StructuredSelector, '#', "Selector"),
            (Kind::SyntaxAssist, '·', "Syntax"),
        ] {
            assert_eq!(CompletionPopupItemView::glyph_for_kind(kind), expected_glyph);
            assert_eq!(CompletionPopupItemView::label_for_kind(kind), expected_label);
        }
    }

    #[test]
    fn completion_popup_view_anchor_uses_replacement_span_start_via_state_offset() {
        // The reducer auto-sync sets anchor_offset to the proposal's
        // replacement_span.start; here we set it explicitly to verify
        // the projector consumes that field rather than the caret.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM");
        // Pretend the popup anchored at offset 1 (start of "SUM") even
        // though the caret has advanced to offset 4 (end of text).
        formula_space.editor_surface_state.caret.offset = 4;
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        // Anchor offset = 1; left_px = 1 * 9 = 9. NOT 4 * 9 = 36.
        assert_eq!(popup.anchor_left_px, 9);
    }

    // -----------------------------------------------------------------
    // Signature help
    // -----------------------------------------------------------------

    fn metrics_9x22() -> crate::ui::editor::geometry::TextareaMeasurementMetrics {
        crate::ui::editor::geometry::TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        }
    }

    fn document_with_signature_help_for_sum(
        source_text: &str,
        active_argument_index: usize,
    ) -> crate::adapters::oxfml::EditorDocument {
        use oxfml_core::syntax::green::SyntaxKind;
        let mut document = sample_editor_document(source_text);
        document.signature_help =
            Some(crate::adapters::oxfml::SignatureHelpContext {
                callee_text: "SUM".to_string(),
                call_span: crate::adapters::oxfml::FormulaTextSpan {
                    start: 1,
                    len: source_text.chars().count().saturating_sub(1),
                },
                active_argument_index,
                invocation_kind: SyntaxKind::CallExpr,
            });
        document.function_help =
            Some(crate::adapters::oxfml::FunctionHelpPacket {
                lookup_key: "SUM".to_string(),
                library_context_snapshot_ref: None,
                display_name: "SUM".to_string(),
                signature_forms: vec![crate::adapters::oxfml::FunctionHelpSignatureForm {
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
            });
        document
    }

    #[test]
    fn signature_help_view_built_from_editor_document() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 5;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.callee_text, "SUM");
        assert_eq!(help.parameters.len(), 3);
        assert_eq!(help.parameters[0].name, "number1");
        assert!(help.parameters[0].is_active);
        assert!(!help.parameters[1].is_active);
        assert_eq!(help.active_parameter, Some(0));
        // Anchor at caret offset 5 with char_width 9 → left 45.
        assert_eq!(help.anchor_left_px, 45);
        assert_eq!(help.line_height_px, 22);
    }

    #[test]
    fn signature_help_view_active_argument_advances_after_comma() {
        // After typing `=SUM(1,` the bridge bumps active_argument_index
        // to 1 — the second parameter is now the active one.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(1,", 1));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 7;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.active_parameter, Some(1));
        assert!(!help.parameters[0].is_active);
        assert!(help.parameters[1].is_active);
        assert!(!help.parameters[2].is_active);
    }

    #[test]
    fn signature_help_view_active_argument_clamps_when_out_of_range() {
        // Bridge reports active_argument_index = 5 but argument_help
        // has 3 entries. Clamp to None (no parameter bolded) rather
        // than panic or wrap.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3,4,5,");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(1,2,3,4,5,", 5));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 15;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.active_parameter, None);
        assert!(help.parameters.iter().all(|p| !p.is_active));
    }

    #[test]
    fn signature_help_view_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2");
        // Document still reflects the pre-`,2` state — stale by one keystroke.
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_empty_when_no_signature_help_in_document() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // Plain sample document carries function_help but no
        // signature_help (sample_editor_document populates the help
        // packet but signature_help only when explicitly attached).
        let mut document = sample_editor_document("=SUM(1,2)");
        document.signature_help = None;
        formula_space.editor_document = Some(document);
        formula_space.editor_box_metrics = Some(metrics_9x22());

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_empty_when_metrics_unmeasured() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(", 0));
        // editor_box_metrics deliberately None — geometry can't anchor yet.

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_suppressed_when_completion_popup_open() {
        // Popup wins; signature help hides until the popup dismisses.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(s");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(s", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_surface_state.caret.offset = 6;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.completion_popup.is_some());
        assert!(
            vm.signature_help.is_none(),
            "signature help must be suppressed while the completion popup is open",
        );
    }

    #[test]
    fn signature_help_view_renders_callee_only_when_function_help_packet_missing() {
        // Defensive: bridge gives signature_help but no function_help
        // (theoretically possible during a brief stale-document tick).
        // The view-model still renders the callee — empty parameter list.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        let mut document = document_with_signature_help_for_sum("=SUM(", 0);
        document.function_help = None;
        formula_space.editor_document = Some(document);
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 5;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.callee_text, "SUM");
        assert!(help.parameters.is_empty());
        assert_eq!(help.active_parameter, None);
    }

    // -----------------------------------------------------------------
    // Function-help card
    // -----------------------------------------------------------------

    #[test]
    fn function_help_card_built_from_editor_document_packet() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // sample_editor_document populates a function_help packet
        // for SUM with three arg names + short description.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let card = vm.function_help_card.expect("card projected");
        assert_eq!(card.lookup_key, "SUM");
        assert_eq!(card.display_name, "SUM");
        assert_eq!(
            card.signature.as_deref(),
            Some("SUM(number1, number2, ...)")
        );
        assert_eq!(
            card.short_description.as_deref(),
            Some("Adds numbers together.")
        );
        assert_eq!(card.availability_summary.as_deref(), Some("supported"));
        assert!(!card.deferred_or_profile_limited);
    }

    #[test]
    fn function_help_card_is_none_when_packet_absent() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        document.function_help = None;
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.function_help_card.is_none());
    }

    #[test]
    fn function_help_card_is_none_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document still reflects the pre-`,3` state.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.function_help_card.is_none());
    }

    #[test]
    fn function_help_card_signature_is_none_when_signature_forms_empty() {
        // Defensive: bridge populates function_help but the packet has
        // no signature forms. The card still renders display_name and
        // description; the signature line is just absent.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        if let Some(ref mut packet) = document.function_help {
            packet.signature_forms.clear();
        }
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let card = vm.function_help_card.expect("card projected");
        assert_eq!(card.lookup_key, "SUM");
        assert!(card.signature.is_none());
    }

    // -----------------------------------------------------------------
    // View mode
    // -----------------------------------------------------------------

    #[test]
    fn view_mode_defaults_to_user_in_view_model() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert_eq!(vm.view_mode, ViewMode::User);
    }

    #[test]
    fn view_mode_developer_propagates_into_view_model() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let mut state = host_state_with(formula_space);
        state.view_mode = ViewMode::Developer;
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert_eq!(vm.view_mode, ViewMode::Developer);
    }

    // -----------------------------------------------------------------
    // Formula drill-down
    // -----------------------------------------------------------------

    #[test]
    fn formula_drill_default_collapsed_with_empty_tree() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(!vm.formula_drill.expanded);
        assert!(vm.formula_drill.tree.is_empty());
        assert!(vm.formula_drill.phase_summaries.is_empty());
        assert!(!vm.formula_drill.document_is_fresh);
    }

    #[test]
    fn formula_drill_expanded_flag_follows_state_field() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.formula_drill_open = true;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.formula_drill.expanded);
    }

    #[test]
    fn formula_drill_flattens_walk_tree_in_preorder_with_depth() {
        use crate::adapters::oxfml::{FormulaWalkNode, FormulaWalkNodeState};
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=LET(x,1,x)");
        let mut document = sample_editor_document("=LET(x,1,x)");
        document.formula_walk = vec![FormulaWalkNode {
            node_id: "let".to_string(),
            label: "LET".to_string(),
            value_preview: Some("1".to_string()),
            state: FormulaWalkNodeState::Evaluated,
            children: vec![
                FormulaWalkNode {
                    node_id: "x-bind".to_string(),
                    label: "x".to_string(),
                    value_preview: Some("1".to_string()),
                    state: FormulaWalkNodeState::Bound,
                    children: vec![],
                },
                FormulaWalkNode {
                    node_id: "x-use".to_string(),
                    label: "x".to_string(),
                    value_preview: Some("1".to_string()),
                    state: FormulaWalkNodeState::Evaluated,
                    children: vec![],
                },
            ],
        }];
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let nodes = &vm.formula_drill.tree;
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].node_id, "let");
        assert_eq!(nodes[0].depth, 0);
        assert!(nodes[0].has_children);
        assert_eq!(nodes[1].node_id, "x-bind");
        assert_eq!(nodes[1].depth, 1);
        assert!(!nodes[1].has_children);
        assert_eq!(nodes[2].node_id, "x-use");
        assert_eq!(nodes[2].depth, 1);
    }

    #[test]
    fn formula_drill_phase_summaries_emit_parse_bind_eval() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let labels: Vec<&str> = vm
            .formula_drill
            .phase_summaries
            .iter()
            .map(|p| p.label)
            .collect();
        assert_eq!(labels, vec!["parse", "bind", "eval"]);
        assert!(vm
            .formula_drill
            .phase_summaries
            .iter()
            .all(|p| p.state == FormulaDrillPhaseState::Ok));
    }

    #[test]
    fn formula_drill_eval_phase_blocked_when_provenance_carries_blocked_reason() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        formula_space.editor_document =
            Some(blocked_editor_document("=XLOOKUP(...)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let eval = vm
            .formula_drill
            .phase_summaries
            .iter()
            .find(|p| p.label == "eval")
            .expect("eval chip emitted");
        assert_eq!(eval.state, FormulaDrillPhaseState::Blocked);
    }

    #[test]
    fn formula_drill_tree_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document still reflects the pre-`,3` state.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.formula_drill_open = true;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.formula_drill.expanded);
        assert!(vm.formula_drill.tree.is_empty());
        assert!(vm.formula_drill.phase_summaries.is_empty());
        assert!(!vm.formula_drill.document_is_fresh);
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

    // -----------------------------------------------------------------------
    // Scenario breadcrumb projection
    // -----------------------------------------------------------------------

    #[test]
    fn breadcrumb_label_falls_back_to_unsaved_for_synthetic_scenario_label() {
        // FormulaSpaceState::new auto-sets scenario_label = formula_space_id;
        // the breadcrumb projects that to "unsaved" rather than leaking the
        // synthetic id into the titlebar.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.active_label, "unsaved");
    }

    #[test]
    fn breadcrumb_label_uses_user_assigned_scenario_label() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        formula_space.context.scenario_label = "invoice-eu-tax".to_string();
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.active_label, "invoice-eu-tax");
    }

    #[test]
    fn breadcrumb_dirty_when_live_text_differs_from_committed() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        formula_space.committed_cell_text = Some("=2".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert!(vm.scenario_breadcrumb.is_dirty);
    }

    #[test]
    fn breadcrumb_clean_when_live_text_matches_committed() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        formula_space.committed_cell_text = Some("=1".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert!(!vm.scenario_breadcrumb.is_dirty);
    }

    #[test]
    fn breadcrumb_dropdown_open_mirrors_global_chrome_state() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let mut state = host_state_with(formula_space);
        assert!(!build_home_shell_view_model(&state).unwrap().scenario_breadcrumb.is_open);
        state.global_ui_chrome.scenario_breadcrumb_open = true;
        assert!(build_home_shell_view_model(&state).unwrap().scenario_breadcrumb.is_open);
    }

    #[test]
    fn breadcrumb_recent_starts_with_active_scenario_marked_active() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        let recent = &vm.scenario_breadcrumb.recent;
        assert!(!recent.is_empty(), "recent must include the active scenario");
        assert!(recent[0].is_active);
        assert_eq!(recent[0].formula_space_id, "space-active");
        assert_eq!(recent[0].meta, "active");
    }

    #[test]
    fn breadcrumb_recent_caps_at_five_entries() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        // Seed 6 closed records; only the first 4 (plus the active one)
        // should make it into the breadcrumb's Recent list.
        for i in 0..6 {
            let id = FormulaSpaceId::new(format!("recent-{i}"));
            state
                .workspace_shell
                .recent_formula_space_order
                .push(id.clone());
            state.workspace_shell.recent_formula_spaces.insert(
                id.clone(),
                ClosedFormulaSpaceRecord {
                    formula_space: FormulaSpaceState::new(id, ""),
                    last_active_mode: AppMode::Explore,
                },
            );
        }
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.recent.len(), 5);
        // First entry is the active one; remaining four are recent-0..recent-3.
        assert!(vm.scenario_breadcrumb.recent[0].is_active);
        for entry in &vm.scenario_breadcrumb.recent[1..] {
            assert!(!entry.is_active);
            assert_eq!(entry.meta, "recent");
        }
    }

    #[test]
    fn breadcrumb_recent_dedupes_active_scenario_from_recent_list() {
        // If the workspace has the active id ALSO listed in
        // recent_formula_space_order (e.g. it was previously closed
        // and re-opened), the active entry must appear once, not
        // twice.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        state
            .workspace_shell
            .recent_formula_space_order
            .push(FormulaSpaceId::new("space-active"));
        let vm = build_home_shell_view_model(&state).expect("active");
        let active_count = vm
            .scenario_breadcrumb
            .recent
            .iter()
            .filter(|entry| entry.formula_space_id == "space-active")
            .count();
        assert_eq!(active_count, 1, "active scenario must appear only once");
    }

    #[test]
    fn breadcrumb_pinned_lists_pinned_ids_and_marks_active_when_active_is_pinned() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(FormulaSpaceId::new("space-active"));
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(FormulaSpaceId::new("space-other-pin"));
        let vm = build_home_shell_view_model(&state).expect("active");
        let pinned = &vm.scenario_breadcrumb.pinned;
        assert_eq!(pinned.len(), 2);
        let active_pin = pinned
            .iter()
            .find(|entry| entry.formula_space_id == "space-active")
            .expect("active pin row");
        assert!(active_pin.is_active);
        assert!(active_pin.is_pinned);
        let other_pin = pinned
            .iter()
            .find(|entry| entry.formula_space_id == "space-other-pin")
            .expect("other pin row");
        assert!(!other_pin.is_active);
        assert!(other_pin.is_pinned);
    }

    #[test]
    fn breadcrumb_actions_carry_stable_ids_and_seam_markers() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        let action_ids: Vec<_> = vm
            .scenario_breadcrumb
            .actions
            .iter()
            .map(|action| action.action_id)
            .collect();
        assert_eq!(
            action_ids,
            vec![
                ScenarioBreadcrumbActionId::NewScenario,
                ScenarioBreadcrumbActionId::SaveAs,
                ScenarioBreadcrumbActionId::Open,
                ScenarioBreadcrumbActionId::Duplicate,
                ScenarioBreadcrumbActionId::ManageScenarios,
            ],
        );
        // After slice 1b, the only action that still carries a SEAM
        // marker is `ManageScenarios` (the manage-formulas page is
        // not yet built). NewScenario / Duplicate use existing
        // in-memory reducers; SaveAs / Open are wired through
        // `persistence/browser_file_io.rs`.
        for action in &vm.scenario_breadcrumb.actions {
            match action.action_id {
                ScenarioBreadcrumbActionId::ManageScenarios => assert_eq!(
                    action.seam_id,
                    Some("SEAM-ONECALC-SCENARIO-PERSIST"),
                ),
                _ => assert_eq!(action.seam_id, None, "{:?}", action.action_id),
            }
        }
    }
}
