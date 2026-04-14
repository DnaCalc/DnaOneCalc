use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::adapters::oxfml::EditorDocument;
use crate::domain::ids::FormulaSpaceId;
use crate::services::programmatic_testing::{
    ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
};
use crate::services::spreadsheet_xml::SpreadsheetXmlCellExtraction;
use crate::services::verification_bundle::{
    OxReplayExplainRecord, OxReplayMismatchRecord, VerificationObservationGapReport,
};
use crate::ui::editor::geometry::EditorOverlayGeometrySnapshot;
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Clone)]
pub struct OneCalcHostState {
    pub workspace_shell: WorkspaceShellState,
    pub formula_spaces: FormulaSpaceCollectionState,
    pub active_formula_space_view: ActiveFormulaSpaceViewState,
    pub retained_artifacts: RetainedArtifactOpenState,
    pub capability_and_environment: CapabilityAndEnvironmentState,
    pub extension_surface: ExtensionSurfaceState,
    pub global_ui_chrome: GlobalUiChromeState,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceShellState {
    pub active_formula_space_id: Option<FormulaSpaceId>,
    pub open_formula_space_order: Vec<FormulaSpaceId>,
    pub pinned_formula_space_ids: BTreeSet<FormulaSpaceId>,
    pub navigation_selection: WorkspaceNavigationSelection,
}

impl Default for WorkspaceShellState {
    fn default() -> Self {
        Self {
            active_formula_space_id: None,
            open_formula_space_order: Vec::new(),
            pinned_formula_space_ids: BTreeSet::new(),
            navigation_selection: WorkspaceNavigationSelection::Overview,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSpaceState {
    pub formula_space_id: FormulaSpaceId,
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub editor_overlay_geometry: Option<EditorOverlayGeometrySnapshot>,
    pub editor_document: Option<EditorDocument>,
    pub completion_help: CompletionHelpState,
    pub latest_evaluation_summary: Option<String>,
    pub effective_display_summary: Option<String>,
    pub context: FormulaSpaceContextState,
    pub array_preview: Option<FormulaArrayPreviewState>,
}

impl FormulaSpaceState {
    pub fn new(formula_space_id: FormulaSpaceId, raw_entered_cell_text: impl Into<String>) -> Self {
        let raw_entered_cell_text = raw_entered_cell_text.into();
        let scenario_label = formula_space_id.as_str().to_string();
        Self {
            formula_space_id,
            raw_entered_cell_text: raw_entered_cell_text.clone(),
            editor_surface_state: EditorSurfaceState::for_text(&raw_entered_cell_text),
            editor_overlay_geometry: None,
            editor_document: None,
            completion_help: CompletionHelpState::default(),
            latest_evaluation_summary: None,
            effective_display_summary: None,
            context: FormulaSpaceContextState {
                scenario_label,
                ..Default::default()
            },
            array_preview: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionTruthSource {
    PreviewBacked,
    LiveBacked,
    LocalFallback,
}

impl ProjectionTruthSource {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectionTruthSource::PreviewBacked => "preview-backed",
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityAndEnvironmentState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtensionSurfaceState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalUiChromeState;

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
pub struct OpenFormulaSpaceRecord {
    pub formula_space_id: FormulaSpaceId,
    pub is_pinned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };

    #[test]
    fn formula_space_tracks_raw_entered_cell_text() {
        let state = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "'123.4");
        assert_eq!(state.raw_entered_cell_text, "'123.4");
        assert_eq!(state.context.scenario_label, "space-1");
        assert!(state.array_preview.is_none());
    }

    #[test]
    fn workspace_shell_defaults_to_explore_without_active_space() {
        let state = OneCalcHostState::default();
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Explore
        );
        assert!(state.workspace_shell.active_formula_space_id.is_none());
    }

    #[test]
    fn retained_artifact_open_state_defaults_to_empty_catalog() {
        let state = RetainedArtifactOpenState::default();
        assert!(state.open_artifact_id.is_none());
        assert!(state.catalog.is_empty());
    }

    #[test]
    fn retained_artifact_record_keeps_open_mode_hint() {
        let record = RetainedArtifactRecord {
            artifact_id: "artifact-1".to_string(),
            case_id: "case-1".to_string(),
            formula_space_id: FormulaSpaceId::new("space-1"),
            comparison_status: ProgrammaticComparisonStatus::Blocked,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            discrepancy_summary: Some("blocked by host policy".to_string()),
            bundle_report_path: None,
            case_output_dir: None,
            xml_extraction: None,
            upstream_gap_report: None,
            oxfml_comparison_value: None,
            excel_comparison_value: None,
            value_match: None,
            display_match: None,
            replay_equivalent: None,
            replay_mismatch_records: Vec::new(),
            replay_explain_records: Vec::new(),
            oxfml_effective_display_summary: None,
            excel_effective_display_text: None,
        };

        assert_eq!(record.open_mode_hint, ProgrammaticOpenModeHint::Workbench);
        assert_eq!(
            record.discrepancy_summary.as_deref(),
            Some("blocked by host policy")
        );
    }
}
