use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::adapters::oxfml::EditorDocument;
use crate::domain::ids::FormulaSpaceId;
use crate::extensions::{
    default_extension_provider_catalog, default_extension_rtd_host,
    default_extension_upstream_pressure_register, frozen_extension_host_contract,
    ExtensionProviderCatalog, ExtensionRtdHostState, ExtensionUpstreamPressureRegister,
    FrozenExtensionHostContract,
};
use crate::services::completion_popup::CompletionPopupState;
use crate::services::programmatic_testing::{
    ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
};
use crate::services::spreadsheet_xml::SpreadsheetXmlCellExtraction;
use crate::services::verification_bundle::{
    OxReplayExplainRecord, OxReplayMismatchRecord, VerificationObservationGapReport,
};
use crate::ui::editor::geometry::{EditorOverlayGeometrySnapshot, TextareaMeasurementMetrics};
use crate::ui::editor::state::{EditorLiveState, EditorSettings, EditorSurfaceState};

#[derive(Debug, Clone)]
pub struct OneCalcHostState {
    pub workspace_shell: WorkspaceShellState,
    pub formula_spaces: FormulaSpaceCollectionState,
    pub active_formula_space_view: ActiveFormulaSpaceViewState,
    pub retained_artifacts: RetainedArtifactOpenState,
    pub capability_and_environment: CapabilityAndEnvironmentState,
    pub extension_surface: ExtensionSurfaceState,
    pub global_ui_chrome: GlobalUiChromeState,
    /// Workspace-level reading-audience preference. Default
    /// [`ViewMode::User`] — Excel-user-friendly chrome with phase
    /// chips, state slugs, and SEAM markers hidden. Toggled to
    /// [`ViewMode::Developer`] via Ctrl+Alt+D for the full
    /// engineering surface. Preference is per-session (not yet
    /// persisted across reloads — that is a separate seam).
    pub view_mode: ViewMode,
    /// Workspace-level "ambient app context" — the background
    /// preferences that drive *defaults* for things like the
    /// presentation-hint format codes. Modelled after Excel reading
    /// the Windows regional settings: not the formula's own state,
    /// not strictly the locale either, but the user's machine /
    /// app preference for what `=NOW()` and `=TODAY()` should look
    /// like *when the formula author hasn't picked a format
    /// explicitly*. Per-formula format codes always trump this.
    pub ambient_app_context: AmbientAppContext,
}

impl Default for OneCalcHostState {
    fn default() -> Self {
        Self {
            workspace_shell: WorkspaceShellState::default(),
            formula_spaces: FormulaSpaceCollectionState::default(),
            active_formula_space_view: ActiveFormulaSpaceViewState::default(),
            retained_artifacts: RetainedArtifactOpenState::default(),
            capability_and_environment: CapabilityAndEnvironmentState::default(),
            extension_surface: ExtensionSurfaceState::default(),
            global_ui_chrome: GlobalUiChromeState::default(),
            view_mode: ViewMode::default(),
            ambient_app_context: AmbientAppContext::default(),
        }
    }
}

/// Workspace-level format defaults — what `=NOW()` / `=TODAY()` etc.
/// should look like when the formula author hasn't picked a format
/// explicitly. Excel sources these from the Windows regional
/// settings; in DnaOneCalc they come from a dedicated host state
/// slot so the user can also override them through workspace
/// preferences (UI knob is a follow-up — the slot is the right
/// architectural home today).
///
/// Initial values are populated by
/// [`AmbientAppContext::detect_for_platform`] which makes a
/// best-effort guess from the browser's `navigator.language` (on
/// wasm) or falls back to ISO defaults (on non-wasm SSR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmbientAppContext {
    /// BCP-47 language tag of the active workspace locale preset
    /// (e.g. `"en-US"`, `"de-DE"`). Drives the format-code triple
    /// below. Surface-only today: month / weekday name tables and
    /// numeric separators stay en-US until OxFml's locale-expansion
    /// chain (`SEAM-OXFML-LOCALE-EXPAND`, blocked on OxFunc W094)
    /// lands. The user can still switch this — the result-hero
    /// format defaults follow immediately, the runtime locale
    /// tables follow when upstream lands.
    pub language_tag: String,
    /// Excel format-string-text used as the default for the
    /// `DateLike` presentation hint **without** a time component
    /// (e.g. `=TODAY()`). User-overridable.
    pub date_format_code: String,
    /// Excel format-string-text used as the default for the
    /// `DateLike` presentation hint **with** a time component
    /// (e.g. `=NOW()`). User-overridable.
    pub datetime_format_code: String,
    /// Excel format-string-text for time-only defaults. Reserved
    /// for functions that return a pure time value once OxFunc
    /// surfaces a `TimeLike` hint — for now no built-in function
    /// emits this hint, but the slot stays so the same indirection
    /// covers it when it arrives.
    pub time_format_code: String,
}

impl Default for AmbientAppContext {
    fn default() -> Self {
        // ISO fallback — safe to read for any audience, deliberately
        // *not* what most users see in Excel. The
        // `detect_for_platform` initialiser overrides this with a
        // platform-derived guess at startup.
        Self {
            language_tag: String::new(),
            date_format_code: "yyyy-mm-dd".to_string(),
            datetime_format_code: "yyyy-mm-dd HH:mm:ss".to_string(),
            time_format_code: "HH:mm:ss".to_string(),
        }
    }
}

/// Reading-audience switch for the home shell. Two values today:
/// `User` for the Excel-user-friendly chrome (phase chips, state
/// slugs, SEAM markers all hidden), and `Developer` for the full
/// engineering surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    User,
    Developer,
}

impl ViewMode {
    pub fn slug(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Developer => "developer",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::User => Self::Developer,
            Self::Developer => Self::User,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceShellState {
    pub active_formula_space_id: Option<FormulaSpaceId>,
    pub open_formula_space_order: Vec<FormulaSpaceId>,
    pub pinned_formula_space_ids: BTreeSet<FormulaSpaceId>,
    pub recent_formula_space_order: Vec<FormulaSpaceId>,
    pub recent_formula_spaces: BTreeMap<FormulaSpaceId, ClosedFormulaSpaceRecord>,
    pub formula_space_modes: BTreeMap<FormulaSpaceId, AppMode>,
    pub navigation_selection: WorkspaceNavigationSelection,
}

impl Default for WorkspaceShellState {
    fn default() -> Self {
        Self {
            active_formula_space_id: None,
            open_formula_space_order: Vec::new(),
            pinned_formula_space_ids: BTreeSet::new(),
            recent_formula_space_order: Vec::new(),
            recent_formula_spaces: BTreeMap::new(),
            formula_space_modes: BTreeMap::new(),
            navigation_selection: WorkspaceNavigationSelection::Overview,
        }
    }
}

// `Eq` cannot be derived: the upstream `EvalValue` carried inside
// `FormulaValuePresentation.published_value` contains `f64` (not `Eq`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormulaSpaceCollectionState {
    pub spaces: BTreeMap<FormulaSpaceId, FormulaSpaceState>,
}

impl FormulaSpaceCollectionState {
    pub fn insert(&mut self, formula_space: FormulaSpaceState) {
        self.spaces
            .insert(formula_space.formula_space_id.clone(), formula_space);
    }

    pub fn get(&self, formula_space_id: &FormulaSpaceId) -> Option<&FormulaSpaceState> {
        self.spaces.get(formula_space_id)
    }

    pub fn get_mut(&mut self, formula_space_id: &FormulaSpaceId) -> Option<&mut FormulaSpaceState> {
        self.spaces.get_mut(formula_space_id)
    }
}

// `Eq` cannot be derived: `editor_document.value_presentation.published_value`
// is an upstream `EvalValue` containing `f64` (not `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaSpaceState {
    pub formula_space_id: FormulaSpaceId,
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub editor_overlay_geometry: Option<EditorOverlayGeometrySnapshot>,
    /// Browser-measured textarea metrics (char-box width / line-height /
    /// scroll position). `None` until the home-shell mount runs the
    /// caret-box measurement adapter for the first time. The popup
    /// view-model treats `None` as "geometry not yet available" and
    /// suppresses caret-anchored surfaces until measurement lands.
    pub editor_box_metrics: Option<TextareaMeasurementMetrics>,
    /// Monotonic counter that increments every time the editor box
    /// metrics are re-measured (mount, window resize, textarea resize).
    /// Surfaced as a `data-measure-tick` attribute for browser tests so
    /// they can detect re-measurement without comparing pixel values
    /// (which may be identical after a no-op resize).
    pub editor_box_metrics_tick: u64,
    pub editor_document: Option<EditorDocument>,
    pub completion_help: CompletionHelpState,
    /// Completion popup lifecycle for this formula space. Default
    /// [`CompletionPopupState::Hidden`]. The reducer auto-opens this
    /// when the bridge returns a non-empty proposals list and
    /// auto-closes it when the proposals empty out.
    pub completion_popup: CompletionPopupState,
    /// Set true by `accept_selected_completion` to suppress the
    /// auto-open hook on the IMMEDIATELY-following bridge refresh.
    /// Without this, accepting a proposal then re-running the bridge
    /// (which is what the synthetic input event after acceptance
    /// triggers) would re-populate the popup with proposals matching
    /// the newly-inserted name, producing a UX wart where the popup
    /// re-appears the moment the user accepts. The sync hook clears
    /// this flag after consuming it once.
    pub completion_popup_suppressed_until_next_input: bool,
    pub latest_evaluation_summary: Option<String>,
    pub effective_display_summary: Option<String>,
    pub context: FormulaSpaceContextState,
    pub array_preview: Option<FormulaArrayPreviewState>,
    pub committed_cell_text: Option<String>,
    pub proofed_cell_text: Option<String>,
    pub expanded_editor: bool,
    /// First progressive-disclosure drill-down. `true` when the
    /// formula walk-tree panel is expanded between the editor-foot
    /// and the result-caption. Toggled by Ctrl+D and by the
    /// editor-foot trigger row. Default false.
    pub formula_drill_open: bool,
    /// Collapsible state for the formatting panel that sits above
    /// the result section (WS-14 plan §5.3, item 8). When `false`
    /// the panel renders as a one-line summary chip ("format ▸
    /// General") that the user can click to expand; when `true`
    /// the full formatting controls row is rendered. Default
    /// false — the panel starts collapsed so the eye runs editor
    /// → result without a visual detour.
    pub formatting_panel_open: bool,
    /// Diagnostics surfaced from the persistence loader (slice 3 of
    /// the persistence ladder). Empty by default; populated when the
    /// user opens a `.dnafml` / `.xml` file that lacked the
    /// `<dna:Formula>` extension and was loaded through the
    /// Excel-only fallback path. The status-foot renders a warning
    /// chip while this list is non-empty; the next save clears it.
    pub load_diagnostics: Vec<crate::persistence::LoadDiagnostic>,
    /// Editable formatting state that drives the result hero's
    /// rendering AND gets persisted into the `.dnafml`'s
    /// `<dna:PublicationContext>` + native `<Styles>` block.
    /// Populated at load time from the saved scenario; mutated by
    /// the UI's formatting-controls row; serialised back at save
    /// time via `scenario_projection::formula_space_to_scenario`.
    pub formatting: FormulaFormattingState,
}

/// Live editable formatting settings for the active formula.
/// Mirrors a subset of `persistence::PublicationContext` plus the
/// locale's `date1904` flag — the fields the UI's formatting-
/// controls row exposes today.
///
/// Fields that aren't yet UI-editable but are part of the persisted
/// schema (style hierarchy, host profile) live in their own future
/// home; this struct carries the surfaces that appear in the
/// formatting panel today.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingState {
    /// Excel format-string-text (e.g. `"0.00"`, `"$#,##0.00"`,
    /// `"yyyy-mm-dd"`). Empty = inherit / `General`.
    pub number_format_code: String,
    /// Hex `#RRGGBB`. Empty = inherit / default.
    pub font_color: String,
    /// Hex `#RRGGBB`. Empty = inherit / default.
    pub fill_color: String,
    /// Locale's date-system flag. False = 1900 (default), true =
    /// 1904 (Mac legacy).
    pub date1904: bool,
    /// Calc-options scenario policy for this formula. `Deterministic`
    /// pins `now_serial` and `random_value` to fixed seeds so a
    /// formula re-runs identically on every keystroke — what the
    /// user wants while authoring. `LiveRecalc` lets each bridge
    /// invocation pick fresh values, so `=NOW()` advances and
    /// `=RAND()` rolls a new value per round-trip. The bridge reads
    /// this and populates `RuntimeFormulaRequest`'s typed-context
    /// bundle accordingly. Default `Deterministic` — that's the
    /// authoring-friendly mode and the only mode the OxFml runtime
    /// exposed pre-MVP.
    pub scenario_policy: crate::persistence::ScenarioPolicy,
    /// Conditional-formatting rules for this formula's result hero.
    /// Each rule renders as a row inside the formatting panel and
    /// flows through `VerificationPublicationContext.conditional_formatting_rules`
    /// to OxFml's publication surface. Empty by default.
    pub conditional_formatting_rules: Vec<FormulaConditionalFormattingRule>,
}

/// Host-side mirror of `oxfml_core`'s `VerificationConditionalFormattingRule`.
/// One entry per CF rule the user has authored on this formula's
/// result hero. The bridge translates this list into the upstream
/// publication context so OxFml can apply / order / fall back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaConditionalFormattingRule {
    /// Rule kind label. Live kinds (post-W070):
    /// `"cell_value"` (operator-driven comparisons),
    /// `"text"` (operator-driven text predicates),
    /// `"dates"` (relative-date predicate; `thresholds[0]` carries
    /// the relative-date keyword like `"today"` or `"last7Days"`),
    /// `"blanks"` / `"noBlanks"` / `"errors"` / `"noErrors"`
    /// (kind-only predicates, no operator / threshold needed),
    /// `"expression"` (free-form formula evaluated as boolean).
    /// Free-form string at the host boundary; tightens when OxFml
    /// surfaces an enum.
    pub rule_kind: String,
    /// Comparison operator name in canonical Excel CF form
    /// (`"greaterThan"`, `"greaterThanOrEqual"`, `"lessThan"`,
    /// `"lessThanOrEqual"`, `"equal"`, `"notEqual"`, `"between"`,
    /// `"notBetween"`, `"containsText"`, `"notContainsText"`,
    /// `"beginsWith"`, `"endsWith"`). OxFml's
    /// `evaluate_operator_rule` strips non-alphanumerics and
    /// lowercases before matching, so casing is forgiving but the
    /// operator *name* must match — abbreviated forms like `"gt"`
    /// silently never fire. `None` when the rule kind doesn't
    /// take an operator (`dates`, `blanks`, `errors`, …).
    pub operator: Option<String>,
    /// Threshold values, in operator order. For `between` two
    /// thresholds; for single-operand operators one threshold;
    /// for `dates` rule kind one entry holding the relative-date
    /// keyword (`"today"`, `"yesterday"`, `"last7Days"`, …);
    /// empty for predicate-only kinds.
    pub thresholds: Vec<String>,
    /// Hex `#RRGGBB` font colour to apply when the rule fires.
    pub font_color: Option<String>,
    /// Hex `#RRGGBB` fill colour to apply when the rule fires.
    pub fill_color: Option<String>,
    /// Optional **typed** rule payload — host-side mirror of
    /// `oxfml_core::publication::ConditionalFormattingTypedRule`
    /// landed in OxFml W073 (`HANDOFF-DNAONECALC-012`). When set,
    /// OxFml prefers this typed payload over the bounded-string
    /// `thresholds` convention. Leaving it `None` keeps the W072
    /// bounded-string fallback active so older callers that only
    /// author through `thresholds` keep working.
    pub typed_rule: Option<FormulaConditionalFormattingTypedRule>,
}

/// Typed CF rule payload — host-side mirror of upstream
/// `ConditionalFormattingTypedRule`. Each sub-options field is
/// `Option<...>`; for a given rule_kind exactly one of them is
/// populated. Empty rule (`Default`) round-trips as a no-op,
/// which lets the seed helper attach a typed rule even when no
/// kind-specific data has been authored yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaConditionalFormattingTypedRule {
    pub color_scale: Option<FormulaColorScaleRuleOptions>,
    pub data_bar: Option<FormulaDataBarRuleOptions>,
    pub icon_set: Option<FormulaIconSetRuleOptions>,
    pub rank: Option<FormulaRankRuleOptions>,
    pub average: Option<FormulaAverageRuleOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaColorScaleRuleOptions {
    pub stops: Vec<FormulaColorScaleStop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaColorScaleStop {
    pub position: FormulaConditionalFormattingThreshold,
    /// Hex `#RRGGBB`.
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaDataBarRuleOptions {
    pub minimum: Option<FormulaConditionalFormattingThreshold>,
    pub maximum: Option<FormulaConditionalFormattingThreshold>,
    /// Hex `#RRGGBB`. `None` means OxFml falls back to
    /// `VerificationConditionalFormattingRule.fill_color`.
    pub bar_color: Option<String>,
    pub direction: Option<FormulaDataBarDirection>,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaDataBarDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaIconSetRuleOptions {
    /// Excel's published icon-set kind, e.g. `"3Arrows"`,
    /// `"3TrafficLights1"`, `"4Rating"`, `"5Quarters"`.
    pub set_kind: String,
    /// `n - 1` thresholds for an `n`-icon set, in ascending order.
    pub thresholds: Vec<FormulaConditionalFormattingThreshold>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaRankRuleOptions {
    pub rank: FormulaConditionalFormattingRank,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormulaConditionalFormattingRank {
    Count(usize),
    /// 0..100 percentile.
    Percent(f64),
}

impl Eq for FormulaConditionalFormattingRank {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormulaAverageRuleOptions {
    /// When true, cells equal to the mean also fire (Excel's
    /// `equalAverage` flag).
    pub include_equal: bool,
    /// Optional stddev offset multiplier (e.g. `1.0` for "above
    /// 1 stddev").
    pub stddev_multiplier: Option<f64>,
}

impl Eq for FormulaAverageRuleOptions {}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FormulaConditionalFormattingThreshold {
    #[default]
    Min,
    Mid,
    Max,
    /// 0..100.
    Percent(f64),
    /// 0..100.
    Percentile(f64),
    Number(f64),
}

impl Eq for FormulaConditionalFormattingThreshold {}

impl FormulaSpaceState {
    pub fn new(formula_space_id: FormulaSpaceId, raw_entered_cell_text: impl Into<String>) -> Self {
        let raw_entered_cell_text = raw_entered_cell_text.into();
        let scenario_label = formula_space_id.as_str().to_string();
        Self {
            formula_space_id,
            raw_entered_cell_text: raw_entered_cell_text.clone(),
            editor_surface_state: EditorSurfaceState::for_text(&raw_entered_cell_text),
            editor_overlay_geometry: None,
            editor_box_metrics: None,
            editor_box_metrics_tick: 0,
            editor_document: None,
            completion_help: CompletionHelpState::default(),
            completion_popup: CompletionPopupState::default(),
            completion_popup_suppressed_until_next_input: false,
            latest_evaluation_summary: None,
            effective_display_summary: None,
            context: FormulaSpaceContextState {
                scenario_label,
                ..Default::default()
            },
            array_preview: None,
            committed_cell_text: None,
            proofed_cell_text: None,
            expanded_editor: false,
            formula_drill_open: false,
            formatting_panel_open: false,
            load_diagnostics: Vec::new(),
            formatting: FormulaFormattingState::default(),
        }
    }

    pub fn live_state(&self) -> EditorLiveState {
        if self.raw_entered_cell_text.is_empty() && self.committed_cell_text.is_none() {
            return EditorLiveState::Idle;
        }
        if self
            .committed_cell_text
            .as_deref()
            .is_some_and(|text| text == self.raw_entered_cell_text)
        {
            return EditorLiveState::Committed;
        }
        if self
            .proofed_cell_text
            .as_deref()
            .is_some_and(|text| text == self.raw_entered_cell_text)
        {
            return EditorLiveState::ProofedScratch;
        }
        EditorLiveState::EditingLive
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionTruthSource {
    LiveBacked,
    LocalFallback,
}

impl ProjectionTruthSource {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectionTruthSource::LiveBacked => "live-backed",
            ProjectionTruthSource::LocalFallback => "local-fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSpaceContextState {
    pub scenario_label: String,
    pub host_profile: String,
    pub packet_kind: String,
    pub capability_floor: String,
    pub mode_availability: String,
    pub truth_source: ProjectionTruthSource,
    pub trace_summary: Option<String>,
    pub blocked_reason: Option<String>,
}

impl Default for FormulaSpaceContextState {
    fn default() -> Self {
        Self {
            scenario_label: "untitled".to_string(),
            host_profile: "host pending".to_string(),
            packet_kind: "packet pending".to_string(),
            capability_floor: "pending".to_string(),
            mode_availability: "explore / inspect / workbench".to_string(),
            truth_source: ProjectionTruthSource::LocalFallback,
            trace_summary: None,
            blocked_reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaArrayPreviewState {
    pub label: String,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionHelpState {
    pub completion_count: usize,
    pub has_signature_help: bool,
    pub function_help_lookup_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFormulaSpaceViewState {
    pub active_mode: AppMode,
    pub selected_formula_space_id: Option<FormulaSpaceId>,
}

impl Default for ActiveFormulaSpaceViewState {
    fn default() -> Self {
        Self {
            active_mode: AppMode::Explore,
            selected_formula_space_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetainedArtifactOpenState {
    pub open_artifact_id: Option<String>,
    pub catalog: BTreeMap<String, RetainedArtifactRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedArtifactRecord {
    pub artifact_id: String,
    pub case_id: String,
    pub formula_space_id: FormulaSpaceId,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
    pub discrepancy_summary: Option<String>,
    pub bundle_report_path: Option<String>,
    pub case_output_dir: Option<String>,
    pub xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    pub upstream_gap_report: Option<VerificationObservationGapReport>,
    pub oxfml_comparison_value: Option<Value>,
    pub excel_comparison_value: Option<Value>,
    pub value_match: Option<bool>,
    pub display_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub replay_mismatch_records: Vec<OxReplayMismatchRecord>,
    pub replay_explain_records: Vec<OxReplayExplainRecord>,
    pub oxfml_effective_display_summary: Option<String>,
    pub excel_effective_display_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityAndEnvironmentState {
    pub selected_diff_target: CapabilityDiffTarget,
}

impl Default for CapabilityAndEnvironmentState {
    fn default() -> Self {
        Self {
            selected_diff_target: CapabilityDiffTarget::WorkspaceBaseline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSurfaceState {
    pub admitted_contract: FrozenExtensionHostContract,
    pub provider_catalog: ExtensionProviderCatalog,
    pub rtd_host: ExtensionRtdHostState,
    pub upstream_pressure: ExtensionUpstreamPressureRegister,
}

impl Default for ExtensionSurfaceState {
    fn default() -> Self {
        let admitted_contract = frozen_extension_host_contract();
        Self {
            provider_catalog: default_extension_provider_catalog(),
            rtd_host: default_extension_rtd_host(),
            upstream_pressure: default_extension_upstream_pressure_register(&admitted_contract),
            admitted_contract,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalUiChromeState {
    pub editor_settings: EditorSettings,
    pub editor_settings_popover_open: bool,
    pub configure_drawer_open: bool,
    /// Titlebar scenario-breadcrumb dropdown lifecycle. `false` is
    /// closed; toggled by clicking the breadcrumb button. Outside
    /// clicks and Esc force `false` via the dedicated reducer
    /// helper. Default closed.
    pub scenario_breadcrumb_open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceNavigationSelection {
    Overview,
    Recent,
    Pinned,
    FormulaSpace(FormulaSpaceId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Explore,
    Inspect,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDiffTarget {
    WorkspaceBaseline,
    OpenFormulaSpace(FormulaSpaceId),
    RecentFormulaSpace(FormulaSpaceId),
}

impl CapabilityDiffTarget {
    pub fn slug(&self) -> String {
        match self {
            Self::WorkspaceBaseline => "workspace-baseline".to_string(),
            Self::OpenFormulaSpace(formula_space_id) => {
                format!("open:{}", formula_space_id.as_str())
            }
            Self::RecentFormulaSpace(formula_space_id) => {
                format!("recent:{}", formula_space_id.as_str())
            }
        }
    }

    pub fn parse(slug: &str) -> Option<Self> {
        if slug == "workspace-baseline" {
            return Some(Self::WorkspaceBaseline);
        }
        if let Some(rest) = slug.strip_prefix("open:") {
            return Some(Self::OpenFormulaSpace(FormulaSpaceId::new(
                rest.to_string(),
            )));
        }
        slug.strip_prefix("recent:")
            .map(|rest| Self::RecentFormulaSpace(FormulaSpaceId::new(rest.to_string())))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosedFormulaSpaceRecord {
    pub formula_space: FormulaSpaceState,
    pub last_active_mode: AppMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFormulaSpaceRecord {
    pub formula_space_id: FormulaSpaceId,
    pub is_pinned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_surface_state_defaults_to_frozen_contract() {
        let surface = ExtensionSurfaceState::default();

        assert_eq!(
            surface.admitted_contract.abi_version.slug(),
            "onecalc-native-extension-abi-v0"
        );
        assert_eq!(surface.admitted_contract.platforms.len(), 3);
        assert_eq!(
            surface.admitted_contract.platforms[0].platform.slug(),
            "windows-desktop"
        );
        assert_eq!(
            surface.provider_catalog.runtime_platform,
            crate::extensions::current_extension_runtime_platform()
        );
        assert!(surface.provider_catalog.providers.is_empty());
        assert!(surface.rtd_host.topics.is_empty());
        assert_eq!(surface.upstream_pressure.records.len(), 3);
        assert_eq!(
            surface.upstream_pressure.report().summary(),
            "3 open / 1 draft / 1 planned / 1 windows-only"
        );
    }
}
