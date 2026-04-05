use dnaonecalc_host::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken};
use dnaonecalc_host::ui::editor::render_projection::syntax_runs_from_snapshot;

#[test]
fn ex_01_projection_uses_editor_syntax_snapshot_as_render_source() {
    let snapshot = EditorSyntaxSnapshot {
        formula_stable_id: "formula-1".to_string(),
        green_tree_key: "green-42".to_string(),
        tokens: vec![
            EditorToken {
                text: "=LET(".to_string(),
                span_start: 0,
                span_len: 5,
            },
            EditorToken {
                text: "values".to_string(),
                span_start: 5,
                span_len: 6,
            },
        ],
    };

    let runs = syntax_runs_from_snapshot(&snapshot);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "=LET(");
    assert_eq!(runs[1].span_start, 5);
}
