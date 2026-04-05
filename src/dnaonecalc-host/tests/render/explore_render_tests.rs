use dnaonecalc_host::services::explore_mode::build_explore_view_model;
use dnaonecalc_host::ui::components::explore_shell::ExploreShell;
use dnaonecalc_host::ui::panels::explore::{
    build_explore_editor_cluster, build_explore_result_cluster,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use leptos::prelude::*;

#[test]
fn ex_10_real_explore_render_path_projects_mode_and_panel_models_into_html() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
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
    assert!(html.contains("=LET(x,1,x)"));
    assert!(html.contains("Green tree: "));
    assert!(html.contains("green-1"));
    assert!(html.contains("Evaluation summary: "));
    assert!(html.contains("Number"));
}
