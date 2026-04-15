//! S6 / S7 — retained artifact + verification bundle import scenarios.

use dnaonecalc_host::app::reducer::{
    import_manual_retained_artifact_into_active_formula_space,
    import_verification_bundle_report_into_workspace, open_retained_artifact_from_catalog,
};
use dnaonecalc_host::services::programmatic_testing::ProgrammaticComparisonStatus;
use dnaonecalc_host::services::retained_artifacts::{
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use dnaonecalc_host::state::AppMode;

use super::fixtures::{
    fresh_state_with_active_space, minimal_bundle_report, shell_frame, workbench_projection,
    BundleCaseFixture,
};

#[test]
fn opening_a_retained_artifact_routes_the_shell_to_workbench() {
    // S6: preseed a manual retained artifact against the active formula
    // space, then open it from the catalog. Active mode must be Workbench
    // and the Workbench projection must build without panic.
    let (mut state, _space) = fresh_state_with_active_space();
    let imported = import_manual_retained_artifact_into_active_formula_space(
        &mut state,
        ManualRetainedArtifactImportRequest {
            artifact_id: "artifact-42".to_string(),
            case_id: "case-42".to_string(),
            comparison_status: ProgrammaticComparisonStatus::Mismatched,
            discrepancy_summary: Some("dna=3 excel=4".to_string()),
        },
    );
    assert!(imported);

    // After import the shell is already in Workbench for the active space;
    // re-open the artifact explicitly to exercise the catalog path.
    let opened = open_retained_artifact_from_catalog(&mut state, "artifact-42");
    assert!(opened);

    assert_eq!(
        state.active_formula_space_view.active_mode,
        AppMode::Workbench
    );
    let workbench = workbench_projection(&state);
    assert_eq!(
        workbench.retained_artifact_id.as_deref(),
        Some("artifact-42"),
        "workbench projection exposes the opened artifact via retained_artifact_id",
    );
    assert_eq!(workbench.retained_case_id.as_deref(), Some("case-42"),);
}

#[test]
fn importing_a_verification_bundle_populates_the_shell_and_catalog() {
    // S7: import a two-case verification bundle report. Shell frame should
    // list a formula space per case (each prefixed with `verify-`), the
    // retained artifact catalog should contain one entry per case, and the
    // first imported artifact should be auto-opened.
    let mut state = fresh_state_with_active_space().0;
    let report = minimal_bundle_report(vec![
        BundleCaseFixture::matched("case-a", "artifact-a", "=SUM(1,2)", "3"),
        BundleCaseFixture::mismatched("case-b", "artifact-b", "=SUM(2,2)", "4"),
    ]);
    let report_json = serde_json::to_string(&report).expect("serialise bundle report");

    let imported = import_verification_bundle_report_into_workspace(
        &mut state,
        VerificationBundleImportRequest { report_json },
    );
    assert!(imported);

    // Catalog carries both artifacts.
    assert!(state.retained_artifacts.catalog.contains_key("artifact-a"));
    assert!(state.retained_artifacts.catalog.contains_key("artifact-b"));

    // Shell frame lists the imported verification spaces alongside the
    // initial untitled space.
    let frame = shell_frame(&state);
    let verify_space_labels: Vec<_> = frame
        .formula_spaces
        .iter()
        .filter(|item| item.formula_space_id.starts_with("verify-"))
        .map(|item| item.label.clone())
        .collect();
    assert_eq!(verify_space_labels.len(), 2);
    assert!(verify_space_labels.iter().any(|label| label == "case-a"));
    assert!(verify_space_labels.iter().any(|label| label == "case-b"));

    // First artifact is auto-opened and the shell is in Workbench (the
    // default `open_mode_hint` for this fixture).
    assert_eq!(
        state.retained_artifacts.open_artifact_id.as_deref(),
        Some("artifact-a"),
    );
    assert_eq!(
        state.active_formula_space_view.active_mode,
        AppMode::Workbench
    );
}
