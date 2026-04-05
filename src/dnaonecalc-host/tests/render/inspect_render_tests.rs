use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::inspect_mode::build_inspect_view_model;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::components::inspect_shell::InspectShell;
use dnaonecalc_host::ui::panels::inspect::{
    build_inspect_summary_cluster, build_inspect_walk_cluster,
};
use leptos::prelude::*;

#[test]
fn in_10_real_inspect_render_path_projects_walk_and_summary_into_html() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let view_model = build_inspect_view_model(&formula_space, None);
    let html = view! {
        <InspectShell
            walk=build_inspect_walk_cluster(&view_model)
            summary=build_inspect_summary_cluster(&view_model)
        />
    }
    .to_html();

    assert!(html.contains("Semantic Inspect"));
    assert!(html.contains("=SUM(1,2)"));
    assert!(html.contains("Green tree: "));
    assert!(html.contains("green-1"));
    assert!(html.contains("data-panel=\"inspect-parse\""));
    assert!(html.contains("Parse"));
    assert!(html.contains("Valid"));
}
