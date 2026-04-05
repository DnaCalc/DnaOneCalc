use dnaonecalc_host::ui::editor::commands::{
    apply_editor_command, keydown_to_command, EditorCommand,
};
use dnaonecalc_host::ui::editor::state::{
    EditorCaret, EditorScrollWindow, EditorSelection, EditorSurfaceState,
};

#[test]
fn ex_12_indent_command_inserts_spaces_for_multiline_selection() {
    let text = "SUM(\n1,\n2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 0 },
        selection: EditorSelection {
            anchor: 0,
            focus: text.chars().count(),
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
    };

    let result = apply_editor_command(text, &state, EditorCommand::IndentWithSpaces);
    assert!(result.text.starts_with("    SUM("));
    assert!(result.text.contains("\n    1,"));
    assert!(result.text.contains("\n    2)"));
}

#[test]
fn ex_13_outdent_command_removes_spaces_for_multiline_selection() {
    let text = "    SUM(\n    1,\n    2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 4 },
        selection: EditorSelection {
            anchor: 0,
            focus: text.chars().count(),
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
    };

    let result = apply_editor_command(text, &state, EditorCommand::OutdentWithSpaces);
    assert!(result.text.starts_with("SUM("));
    assert!(result.text.contains("\n1,"));
    assert!(result.text.contains("\n2)"));
}

#[test]
fn ex_14_keydown_mapping_covers_tab_and_arrow_navigation() {
    assert_eq!(keydown_to_command("ArrowLeft", false), Some(EditorCommand::MoveCaretLeft));
    assert_eq!(keydown_to_command("ArrowRight", false), Some(EditorCommand::MoveCaretRight));
    assert_eq!(keydown_to_command("ArrowLeft", true), Some(EditorCommand::ExtendSelectionLeft));
    assert_eq!(
        keydown_to_command("ArrowRight", true),
        Some(EditorCommand::ExtendSelectionRight)
    );
    assert_eq!(keydown_to_command("Tab", false), Some(EditorCommand::IndentWithSpaces));
    assert_eq!(keydown_to_command("Tab", true), Some(EditorCommand::OutdentWithSpaces));
}

#[test]
fn ex_17_shift_arrow_commands_expand_and_contract_selection() {
    let text = "=SUM(1,2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 4 },
        selection: EditorSelection::collapsed(4),
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
    };

    let left = apply_editor_command(text, &state, EditorCommand::ExtendSelectionLeft);
    assert_eq!(left.state.selection.anchor, 4);
    assert_eq!(left.state.selection.focus, 3);
    assert!(!left.state.selection.is_collapsed());

    let right = apply_editor_command(text, &left.state, EditorCommand::ExtendSelectionRight);
    assert_eq!(right.state.selection.anchor, 4);
    assert_eq!(right.state.selection.focus, 4);
    assert!(right.state.selection.is_collapsed());
}
