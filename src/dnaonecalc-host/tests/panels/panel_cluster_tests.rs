use dnaonecalc_host::services::explore_mode::build_explore_view_model;
use dnaonecalc_host::services::inspect_mode::build_inspect_view_model;
use dnaonecalc_host::ui::panels::explore::{
    build_explore_editor_cluster, build_explore_result_cluster,
};
use dnaonecalc_host::ui::panels::inspect::{
    build_inspect_summary_cluster, build_inspect_walk_cluster,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;

#[test]
fn ex_07_explore_panel_clusters_split_editing_and_result_fields_for_rendering() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.effective_display_summary = Some("3".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let mode_view = build_explore_view_model(&formula_space);
    let editor_cluster = build_explore_editor_cluster(&mode_view);
    let result_cluster = build_explore_result_cluster(&mode_view);

    assert_eq!(editor_cluster.raw_entered_cell_text, "=SUM(1,2)");
    assert_eq!(editor_cluster.green_tree_key.as_deref(), Some("green-1"));
    assert_eq!(result_cluster.effective_display_summary.as_deref(), Some("3"));
}

#[test]
fn in_08_inspect_panel_clusters_split_walk_and_summary_fields_for_rendering() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let mode_view = build_inspect_view_model(&formula_space);
    let walk_cluster = build_inspect_walk_cluster(&mode_view);
    let summary_cluster = build_inspect_summary_cluster(&mode_view);

    assert_eq!(walk_cluster.green_tree_key.as_deref(), Some("green-1"));
    assert_eq!(walk_cluster.formula_walk_nodes.len(), 1);
    assert_eq!(summary_cluster.parse_summary.as_ref().map(|x| x.status.as_str()), Some("Valid"));
}
