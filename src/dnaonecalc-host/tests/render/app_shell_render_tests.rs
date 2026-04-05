use dnaonecalc_host::domain::ids::FormulaSpaceId;
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
    assert!(html.contains("Formula Explorer"));
    assert!(html.contains("data-mode=\"Explore\""));
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
    assert!(html.contains("data-panel=\"workbench-compare\""));
    assert!(html.contains("data-panel=\"workbench-replay\""));
}
