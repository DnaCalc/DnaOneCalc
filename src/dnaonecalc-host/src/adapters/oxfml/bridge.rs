use super::types::{EditorAnalysisStage, EditorDocument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditRequest {
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub previous_green_tree_key: Option<String>,
    pub analysis_stage: EditorAnalysisStage,
    /// Optional VerificationPublicationContext driving the runtime
    /// pass's effective_display_text computation. The host populates
    /// this from the active formula's FormulaFormattingState — the
    /// number format code, font / fill colours, and CF rules.
    /// `None` skips the formatted-display lane.
    pub formatting_request: Option<FormulaFormattingRequest>,
    /// Calc-options scenario policy. Drives whether the bridge
    /// supplies fixed `now_serial` / `random_value` seeds
    /// (Deterministic) or fresh values per request (LiveRecalc).
    pub scenario_policy: ScenarioPolicyRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingRequest {
    pub number_format_code: Option<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub style_id: Option<String>,
    pub date1904: bool,
    /// Conditional-formatting rules to evaluate on the result hero.
    /// Rendered through `VerificationPublicationContext.conditional_formatting_rules`
    /// in the runtime pass; the matching rule's
    /// `effective_display_text` (when set) replaces the base
    /// formatted display.
    pub conditional_formatting_rules: Vec<FormulaFormattingCfRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfRule {
    pub rule_kind: String,
    pub operator: Option<String>,
    pub thresholds: Vec<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    /// Optional typed CF rule payload, mirroring OxFml W073's
    /// `ConditionalFormattingTypedRule`. When set, the bridge attaches
    /// the upstream `typed_rule` field on the resulting
    /// `VerificationConditionalFormattingRule`. The bounded-string
    /// `thresholds` keeps riding along as the W072 fallback so older
    /// callers continue to work.
    pub typed_rule: Option<FormulaFormattingCfTypedRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfTypedRule {
    pub color_scale: Option<FormulaFormattingCfColorScaleRuleOptions>,
    pub data_bar: Option<FormulaFormattingCfDataBarRuleOptions>,
    pub icon_set: Option<FormulaFormattingCfIconSetRuleOptions>,
    pub rank: Option<FormulaFormattingCfRankRuleOptions>,
    pub average: Option<FormulaFormattingCfAverageRuleOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfColorScaleRuleOptions {
    pub stops: Vec<FormulaFormattingCfColorScaleStop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfColorScaleStop {
    pub position: FormulaFormattingCfThreshold,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfDataBarRuleOptions {
    pub minimum: Option<FormulaFormattingCfThreshold>,
    pub maximum: Option<FormulaFormattingCfThreshold>,
    pub bar_color: Option<String>,
    pub direction: Option<FormulaFormattingCfDataBarDirection>,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaFormattingCfDataBarDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfIconSetRuleOptions {
    pub set_kind: String,
    pub thresholds: Vec<FormulaFormattingCfThreshold>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfRankRuleOptions {
    pub rank: FormulaFormattingCfRank,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormulaFormattingCfRank {
    Count(usize),
    Percent(f64),
}

impl Eq for FormulaFormattingCfRank {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormulaFormattingCfAverageRuleOptions {
    pub include_equal: bool,
    pub stddev_multiplier: Option<f64>,
}

impl Eq for FormulaFormattingCfAverageRuleOptions {}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FormulaFormattingCfThreshold {
    #[default]
    Min,
    Mid,
    Max,
    Percent(f64),
    Percentile(f64),
    Number(f64),
}

impl Eq for FormulaFormattingCfThreshold {}

/// Mirror of `crate::persistence::ScenarioPolicy` at the bridge
/// boundary. Kept as its own type so the bridge module does not
/// take a hard dependency on `crate::persistence`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScenarioPolicyRequest {
    Deterministic,
    /// Default: matches `ScenarioPolicy::LiveRecalc`.
    #[default]
    LiveRecalc,
}

// `Eq` cannot be derived: `EditorDocument.value_presentation.published_value`
// is the upstream `EvalValue`, which contains `f64` (not `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaEditResult {
    pub document: EditorDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OxfmlEditorBridgeError {
    UpstreamFailure(String),
}

pub trait OxfmlEditorBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError>;
}
