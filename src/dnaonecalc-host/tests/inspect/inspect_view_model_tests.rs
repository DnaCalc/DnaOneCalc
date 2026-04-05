use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::services::inspect_mode::build_inspect_view_model;

#[test]
fn in_04_inspect_view_model_projects_formula_walk_and_summaries() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let view_model = build_inspect_view_model(&formula_space);

    assert_eq!(view_model.raw_entered_cell_text, "=SUM(1,2)");
    assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
    assert_eq!(view_model.formula_walk_nodes.len(), 1);
    assert_eq!(view_model.formula_walk_nodes[0].label, "SUM");
    assert_eq!(view_model.parse_summary.as_ref().map(|x| x.status.as_str()), Some("Valid"));
    assert_eq!(view_model.bind_summary.as_ref().map(|x| x.variable_count), Some(0));
    assert_eq!(view_model.eval_summary.as_ref().map(|x| x.step_count), Some(1));
    assert_eq!(
        view_model
            .provenance_summary
            .as_ref()
            .map(|x| x.profile_summary.as_str()),
        Some("OC-H0")
    );
}

#[test]
fn in_06_inspect_view_model_keeps_read_only_fallback_without_editor_document() {
    let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-2"), "'123.4");

    let view_model = build_inspect_view_model(&formula_space);

    assert_eq!(view_model.raw_entered_cell_text, "'123.4");
    assert!(view_model.green_tree_key.is_none());
    assert!(view_model.formula_walk_nodes.is_empty());
    assert!(view_model.parse_summary.is_none());
}
