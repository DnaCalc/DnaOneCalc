use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::modes::explore::build_explore_view_model;

#[test]
fn ex_04_explore_view_model_uses_oxfml_editor_document_for_surface_projection() {
    let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    formula_space.effective_display_summary = Some("1".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());

    let view_model = build_explore_view_model(&formula_space);

    assert_eq!(view_model.raw_entered_cell_text, "=LET(x,1,x)");
    assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
    assert_eq!(view_model.syntax_runs.len(), 9);
    assert_eq!(view_model.syntax_runs[1].text, "LET");
    assert_eq!(view_model.diagnostics.len(), 1);
    assert_eq!(view_model.function_help_lookup_key.as_deref(), Some("SUM"));
}

#[test]
fn ex_05_explore_view_model_falls_back_to_raw_text_without_editor_document() {
    let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-2"), "'123.4");

    let view_model = build_explore_view_model(&formula_space);

    assert_eq!(view_model.syntax_runs.len(), 1);
    assert_eq!(view_model.syntax_runs[0].text, "'123.4");
    assert!(view_model.green_tree_key.is_none());
    assert!(view_model.diagnostics.is_empty());
}
