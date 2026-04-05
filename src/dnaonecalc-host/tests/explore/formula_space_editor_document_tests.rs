use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::editor_session::EditorSessionService;
use dnaonecalc_host::state::{FormulaSpaceCollectionState, FormulaSpaceState};
use dnaonecalc_host::test_support::sample_editor_document;

#[test]
fn ex_02_editor_session_keeps_raw_cell_entry_text_and_oxfml_document() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut formula_spaces = FormulaSpaceCollectionState::default();
    formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2,3)",
    ));

    EditorSessionService::apply_editor_document(
        &mut formula_spaces,
        &formula_space_id,
        sample_editor_document("'123.4"),
    )
    .expect("known formula space should update");

    let formula_space = formula_spaces.get(&formula_space_id).expect("space exists");
    assert_eq!(formula_space.raw_entered_cell_text, "'123.4");
    assert_eq!(
        formula_space
            .editor_document
            .as_ref()
            .expect("editor document retained")
            .editor_syntax_snapshot
            .green_tree_key,
        "green-1"
    );
    assert_eq!(formula_space.completion_help.completion_count, 1);
}
