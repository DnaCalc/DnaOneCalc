//! Shared helpers for the scenario integration crate.
//!
//! Scenarios start from `OneCalcHostState::default()` plus a minimal setup
//! block; anything that is more than two lines of setup lives here so the
//! scenario body itself stays short and readable.

use dnaonecalc_host::adapters::oxfml::{LiveOxfmlBridge, OxfmlEditorBridge};
use dnaonecalc_host::app::case_lifecycle::new_formula_space;
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::explore_mode::ExploreViewModel;
use dnaonecalc_host::services::inspect_mode::InspectViewModel;
use dnaonecalc_host::services::live_edit::apply_live_editor_input;
use dnaonecalc_host::services::programmatic_testing::{
    ProgrammaticArtifactCatalogEntry, ProgrammaticBatchPlan, ProgrammaticCapabilityProfile,
    ProgrammaticComparisonLane, ProgrammaticComparisonStatus, ProgrammaticHostProfile,
    ProgrammaticOpenModeHint,
};
use dnaonecalc_host::services::shell_composition::{
    build_active_mode_projection, build_shell_frame_view_model, ActiveModeProjection,
    ShellFrameViewModel,
};
use dnaonecalc_host::services::verification_bundle::{
    OxfmlVerificationSummary, VerificationBundleReport, VerificationCaseReport,
};
use dnaonecalc_host::services::workbench_mode::WorkbenchViewModel;
use dnaonecalc_host::state::OneCalcHostState;
use dnaonecalc_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};
use dnaonecalc_host::ui::panels::explore::{
    build_explore_editor_cluster, build_explore_result_cluster, ExploreEditorClusterViewModel,
    ExploreResultClusterViewModel,
};

/// Create a fresh workspace with one active untitled formula space and
/// return both the state and the id of that space.
pub fn fresh_state_with_active_space() -> (OneCalcHostState, FormulaSpaceId) {
    let mut state = OneCalcHostState::default();
    let formula_space_id = new_formula_space(&mut state);
    (state, formula_space_id)
}

/// Build the real live OxFml editor bridge used by the running app. A
/// scenario should construct one at the top and reuse it across multiple
/// `type_formula` calls so the bridge's edit-reuse cache behaves the way
/// it does in the browser.
pub fn scenario_bridge() -> LiveOxfmlBridge {
    LiveOxfmlBridge::default()
}

/// Dispatch a user-level "type the whole string" input event through the
/// real reducer + the provided bridge. Matches how the editor surface
/// forwards a textarea input change. Uses `LiveOxfmlBridge` in normal
/// scenarios so parse / bind / eval flow through the real OxFml + OxFunc
/// runtime; tests that need a deterministic fault injection (diagnostics,
/// blocked reasons) pass their own fake bridge to `apply_live_editor_input`
/// directly.
pub fn type_formula(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    text: &str,
) {
    let caret_offset = text.chars().count();
    apply_live_editor_input(
        bridge,
        state,
        EditorInputEvent {
            text: text.to_string(),
            selection_start: Some(caret_offset),
            selection_end: Some(caret_offset),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some(text.to_string()),
        },
    )
    .expect("live bridge should succeed for scenario inputs");
}

/// Build the active mode projection and drill into the Explore view model.
/// Panics if the active mode is not Explore; use `active_mode_projection`
/// for the general case.
pub fn explore_projection(state: &OneCalcHostState) -> ExploreViewModel {
    match build_active_mode_projection(state).expect("active mode projection") {
        ActiveModeProjection::Explore(view_model) => view_model,
        other => panic!("expected Explore projection, got {other:?}"),
    }
}

pub fn inspect_projection(state: &OneCalcHostState) -> InspectViewModel {
    match build_active_mode_projection(state).expect("active mode projection") {
        ActiveModeProjection::Inspect(view_model) => view_model,
        other => panic!("expected Inspect projection, got {other:?}"),
    }
}

pub fn workbench_projection(state: &OneCalcHostState) -> WorkbenchViewModel {
    match build_active_mode_projection(state).expect("active mode projection") {
        ActiveModeProjection::Workbench(view_model) => view_model,
        other => panic!("expected Workbench projection, got {other:?}"),
    }
}

pub fn shell_frame(state: &OneCalcHostState) -> ShellFrameViewModel {
    build_shell_frame_view_model(state).expect("shell frame view model")
}

pub fn explore_editor_cluster(
    view_model: &ExploreViewModel,
) -> ExploreEditorClusterViewModel {
    build_explore_editor_cluster(view_model)
}

pub fn explore_result_cluster(
    view_model: &ExploreViewModel,
) -> ExploreResultClusterViewModel {
    build_explore_result_cluster(view_model)
}

/// Build a minimal verification bundle report fixture carrying the given
/// cases. Used by artifact-import scenarios (S7 / S16).
pub fn minimal_bundle_report(cases: Vec<BundleCaseFixture>) -> VerificationBundleReport {
    let retained_artifact_catalog: Vec<ProgrammaticArtifactCatalogEntry> = cases
        .iter()
        .map(|case| ProgrammaticArtifactCatalogEntry {
            artifact_id: case.artifact_id.clone(),
            case_id: case.case_id.clone(),
            comparison_status: case.comparison_status,
            open_mode_hint: case.open_mode_hint,
        })
        .collect();

    let case_reports: Vec<VerificationCaseReport> = cases
        .iter()
        .map(|case| VerificationCaseReport {
            case_id: case.case_id.clone(),
            entered_cell_text: case.entered_cell_text.clone(),
            artifact_catalog_entry: ProgrammaticArtifactCatalogEntry {
                artifact_id: case.artifact_id.clone(),
                case_id: case.case_id.clone(),
                comparison_status: case.comparison_status,
                open_mode_hint: case.open_mode_hint,
            },
            comparison_status: case.comparison_status,
            value_match: case.value_match,
            display_match: case.display_match,
            replay_equivalent: case.replay_equivalent,
            replay_mismatch_kinds: Vec::new(),
            replay_mismatch_records: Vec::new(),
            replay_explain_records: Vec::new(),
            discrepancy_summary: None,
            oxfml_summary: OxfmlVerificationSummary {
                evaluation_summary: Some(format!("Number · {}", case.case_id)),
                comparison_value: None,
                effective_display_summary: Some(case.effective_display_summary.clone()),
                blocked_reason: None,
                parse_status: Some("Valid".to_string()),
                green_tree_key: Some(format!("green-{}", case.case_id)),
            },
            excel_summary: None,
            spreadsheet_xml_extraction: None,
            upstream_gap_report: None,
            case_output_dir: format!("target/scenarios/{}", case.case_id),
            scenario_path: format!("target/scenarios/{}/scenario.json", case.case_id),
        })
        .collect();

    VerificationBundleReport {
        bundle_id: "scenario-bundle".to_string(),
        output_root: "target/scenarios/bundle".to_string(),
        host_profile: default_host_profile(),
        capabilities: default_capability_profile(),
        batch_plan: ProgrammaticBatchPlan {
            formula_count: cases.len(),
            comparison_lane: ProgrammaticComparisonLane::OxfmlOnly,
            discrepancy_index_required: false,
            retained_artifact_kinds: vec!["comparison_outcome".to_string()],
        },
        retained_artifact_catalog,
        case_reports,
    }
}

fn default_host_profile() -> ProgrammaticHostProfile {
    dnaonecalc_host::services::programmatic_testing::default_windows_excel_host_profile()
}

fn default_capability_profile() -> ProgrammaticCapabilityProfile {
    dnaonecalc_host::services::programmatic_testing::default_windows_excel_capability_profile()
}

/// Minimal fixture shape for a single case inside a verification bundle.
#[derive(Debug, Clone)]
pub struct BundleCaseFixture {
    pub case_id: String,
    pub artifact_id: String,
    pub entered_cell_text: String,
    pub effective_display_summary: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
    pub value_match: Option<bool>,
    pub display_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
}

impl BundleCaseFixture {
    pub fn matched(case_id: &str, artifact_id: &str, entered: &str, display: &str) -> Self {
        Self {
            case_id: case_id.to_string(),
            artifact_id: artifact_id.to_string(),
            entered_cell_text: entered.to_string(),
            effective_display_summary: display.to_string(),
            comparison_status: ProgrammaticComparisonStatus::Matched,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            value_match: Some(true),
            display_match: Some(true),
            replay_equivalent: Some(true),
        }
    }

    pub fn mismatched(case_id: &str, artifact_id: &str, entered: &str, display: &str) -> Self {
        Self {
            case_id: case_id.to_string(),
            artifact_id: artifact_id.to_string(),
            entered_cell_text: entered.to_string(),
            effective_display_summary: display.to_string(),
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            value_match: Some(false),
            display_match: Some(false),
            replay_equivalent: Some(false),
        }
    }
}
