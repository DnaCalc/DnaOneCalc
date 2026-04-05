use dnaonecalc_host::services::explore_mode::build_explore_view_model;
use dnaonecalc_host::ui::components::explore_shell::ExploreShell;
use dnaonecalc_host::ui::panels::explore::{
    build_explore_editor_cluster, build_explore_result_cluster,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::editor::state::EditorSelection;
use leptos::prelude::*;

#[test]
fn ex_10_real_explore_render_path_projects_mode_and_panel_models_into_html() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    formula_space.effective_display_summary = Some("1".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let view_model = build_explore_view_model(&formula_space);
    let html = view! {
        <ExploreShell
            editor=build_explore_editor_cluster(&view_model)
            result=build_explore_result_cluster(&view_model)
        />
    }
    .to_html();

    assert!(html.contains("Formula Explorer"));
    assert!(html.contains("data-component=\"formula-editor-surface\""));
    assert!(html.contains("data-role=\"editor-input\""));
    assert!(html.contains("data-role=\"syntax-layer\""));
    assert!(html.contains("data-token-role=\"function\""));
    assert!(html.contains("data-role=\"caret-indicator\""));
    assert!(html.contains("data-role=\"selection-indicator\""));
    assert!(html.contains("data-role=\"completion-popup\""));
    assert!(html.contains("data-role=\"signature-help-popup\""));
    assert!(html.contains("data-role=\"inline-diagnostic\""));
    assert!(html.contains("data-measurement-source=\"derived-grid\""));
    assert!(html.contains("data-anchor-line=\"0\""));
    assert!(html.contains("data-call-line=\"0\""));
    assert!(html.contains("data-role=\"function-help-card\""));
    assert!(html.contains("data-role=\"function-help-signature-argument\""));
    assert!(html.contains("data-selected=\"true\""));
    assert!(html.contains("Evaluation summary: "));
    assert!(html.contains("Number"));
}

#[test]
fn ex_19_real_explore_render_path_surfaces_range_selection_state() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    formula_space.editor_surface_state.selection = EditorSelection { anchor: 4, focus: 8 };

    let view_model = build_explore_view_model(&formula_space);
    let html = view! {
        <ExploreShell
            editor=build_explore_editor_cluster(&view_model)
            result=build_explore_result_cluster(&view_model)
        />
    }
    .to_html();

    assert!(html.contains("data-selection-kind=\"range\""));
    assert!(html.contains("data-selection-start=\"4\""));
    assert!(html.contains("data-selection-end=\"8\""));
}
