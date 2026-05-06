use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaFormattingRequest, RecalcModeRequest, ScenarioPolicyRequest,
};
use crate::domain::ids::FormulaSpaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIntent {
    ApplyFormulaEdit(ApplyFormulaEditIntent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFormulaEditIntent {
    pub formula_space_id: FormulaSpaceId,
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub analysis_stage: EditorAnalysisStage,
    /// Live formatting state for the active formula. The host
    /// derives this from `FormulaSpaceState.formatting` and forwards
    /// it through the bridge so the runtime pass populates the
    /// `verification_publication_surface.effective_display_text`
    /// the result hero renders. `None` skips the formatted-display
    /// lane (the hero falls back to the raw value).
    pub formatting_request: Option<FormulaFormattingRequest>,
    /// Calc-options scenario policy lifted from the active
    /// formula's `FormulaFormattingState.scenario_policy`. Drives
    /// `now_serial` / `random_value` seeding in the bridge.
    pub scenario_policy: ScenarioPolicyRequest,
    /// When `true`, skip the runtime-evaluation pass and run only
    /// parse / bind / popup / signature-help / function-help. The
    /// resulting `EditorDocument` carries the prior runtime fields
    /// from cache (or `None` for fresh formulas) — popups + caret
    /// state refresh, value / walk / display do not change. Set on
    /// caret-only navigation (mouse click, arrow keys, Home / End,
    /// PageUp / PageDown).
    pub skip_runtime_evaluation: bool,
    /// Per-formula recalc mode. `Auto` runs the runtime on every
    /// text-input event (subject to `skip_runtime_evaluation`);
    /// `Manual` gates the runtime on an explicit Calculate / F9
    /// request, decoupling typing latency from formula complexity.
    pub recalc_mode: RecalcModeRequest,
}
