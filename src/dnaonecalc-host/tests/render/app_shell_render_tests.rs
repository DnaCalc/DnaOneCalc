// TODO(dno-yjk.A5): this file exercises the archived rich Leptos surface
// and must move to `tests/ui_archive_2026_04/` under
// `#[cfg(feature = "ui-archive-2026-04")]` when `dno-yjk.A5` archives the
// rich shell components. Until the archive destination exists, the test
// stays here as a reference pin for the rich surface.

use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::programmatic_testing::{
    ProgrammaticArtifactCatalogEntry, ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
};
use dnaonecalc_host::services::retained_artifacts::{
    import_programmatic_artifact, RetainedArtifactImportRequest,
};
use dnaonecalc_host::state::{AppMode, FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::components::app_shell::OneCalcShellApp;
use leptos::prelude::*;

#[test]
fn ex_01_shell_render_path_wraps_explore_mode_inside_shared_frame() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-theme=\"onecalc-theme\""));
    assert!(html.contains("data-host-app=\"onecalc\""));
    assert!(html.contains("DNA OneCalc"));
    assert!(html.contains("data-mode=\"Explore\""));
    assert!(html.contains("data-role=\"shell-frame-configure-toggle\""));
    assert!(html.contains("data-component=\"formula-editor-surface\""));
    assert!(!html.contains("Formula Explorer"));
}

#[test]
fn in_01_shell_render_path_wraps_inspect_mode_inside_shared_frame() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = AppMode::Inspect;
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-theme=\"onecalc-theme\""));
    assert!(html.contains("DNA OneCalc"));
    assert!(html.contains("Semantic Inspect"));
    assert!(html.contains("data-mode=\"Inspect\""));
}

#[test]
fn wb_02_shell_render_path_wraps_workbench_mode_inside_shared_frame() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = AppMode::Workbench;
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-theme=\"onecalc-theme\""));
    assert!(html.contains("DNA OneCalc"));
    assert!(html.contains("Twin Oracle Workbench"));
    assert!(html.contains("data-panel=\"workbench-outcome\""));
    assert!(html.contains("data-panel=\"workbench-catalog\""));
    assert!(html.contains("data-panel=\"workbench-compare\""));
    assert!(html.contains("data-panel=\"workbench-replay\""));
}

#[test]
fn wb_03_workbench_render_path_surfaces_open_retained_artifact() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.active_formula_space_view.active_mode = AppMode::Workbench;
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id,
            catalog_entry: ProgrammaticArtifactCatalogEntry {
                artifact_id: "artifact-1".to_string(),
                case_id: "case-1".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Mismatched,
                open_mode_hint: ProgrammaticOpenModeHint::Workbench,
            },
            discrepancy_summary: Some("dna=1 excel=2".to_string()),
        },
    );
    state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("Twin Oracle Workbench"));
    assert!(html.contains("data-role=\"retained-artifact-id\""));
    assert!(html.contains("artifact-1"));
    assert!(html.contains("data-role=\"retained-discrepancy-summary\""));
    assert!(html.contains("dna=1 excel=2"));
    assert!(html.contains("data-role=\"retained-catalog-item\""));
    assert!(html.contains("data-open=\"true\""));
}
