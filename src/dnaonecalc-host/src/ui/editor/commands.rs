use crate::ui::editor::state::{EditorCaret, EditorSelection, EditorSurfaceState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    MoveCaretLeft,
    MoveCaretRight,
    IndentWithSpaces,
    OutdentWithSpaces,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorInputEvent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommandResult {
    pub text: String,
    pub state: EditorSurfaceState,
}

pub fn apply_editor_command(
    text: &str,
    state: &EditorSurfaceState,
    command: EditorCommand,
) -> EditorCommandResult {
    match command {
        EditorCommand::MoveCaretLeft => EditorCommandResult {
            text: text.to_string(),
            state: EditorSurfaceState {
                caret: EditorCaret {
                    offset: state.caret.offset.saturating_sub(1),
                },
                selection: EditorSelection::collapsed(state.caret.offset.saturating_sub(1)),
                scroll_window: state.scroll_window.clone(),
            },
        },
        EditorCommand::MoveCaretRight => {
            let next = (state.caret.offset + 1).min(text.chars().count());
            EditorCommandResult {
                text: text.to_string(),
                state: EditorSurfaceState {
                    caret: EditorCaret { offset: next },
                    selection: EditorSelection::collapsed(next),
                    scroll_window: state.scroll_window.clone(),
                },
            }
        }
        EditorCommand::IndentWithSpaces => indent_with_spaces(text, state),
        EditorCommand::OutdentWithSpaces => outdent_with_spaces(text, state),
    }
}

pub fn keydown_to_command(key: &str, shift_key: bool) -> Option<EditorCommand> {
    match (key, shift_key) {
        ("ArrowLeft", false) => Some(EditorCommand::MoveCaretLeft),
        ("ArrowRight", false) => Some(EditorCommand::MoveCaretRight),
        ("Tab", false) => Some(EditorCommand::IndentWithSpaces),
        ("Tab", true) => Some(EditorCommand::OutdentWithSpaces),
        _ => None,
    }
}

fn indent_with_spaces(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    let indent = "    ";
    let line_starts = selected_line_starts(text, &state.selection);
    let mut result = text.to_string();
    let mut inserted_before_caret = 0usize;
    let mut inserted_before_anchor = 0usize;
    let mut inserted_before_focus = 0usize;

    for line_start in line_starts.into_iter().rev() {
        let byte_idx = char_to_byte_idx(&result, line_start);
        result.insert_str(byte_idx, indent);
        if line_start <= state.caret.offset {
            inserted_before_caret += indent.chars().count();
        }
        if line_start <= state.selection.anchor {
            inserted_before_anchor += indent.chars().count();
        }
        if line_start <= state.selection.focus {
            inserted_before_focus += indent.chars().count();
        }
    }

    let new_caret = state.caret.offset + inserted_before_caret;
    EditorCommandResult {
        text: result,
        state: EditorSurfaceState {
            caret: EditorCaret { offset: new_caret },
            selection: EditorSelection {
                anchor: state.selection.anchor + inserted_before_anchor,
                focus: state.selection.focus + inserted_before_focus,
            },
            scroll_window: state.scroll_window.clone(),
        },
    }
}

fn outdent_with_spaces(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    let line_starts = selected_line_starts(text, &state.selection);
    let mut result = text.to_string();
    let mut removed_before_caret = 0usize;
    let mut removed_before_anchor = 0usize;
    let mut removed_before_focus = 0usize;

    for line_start in line_starts.into_iter().rev() {
        let removal = leading_spaces_at(&result, line_start).min(4);
        if removal == 0 {
            continue;
        }

        let byte_idx = char_to_byte_idx(&result, line_start);
        let end_idx = char_to_byte_idx(&result, line_start + removal);
        result.replace_range(byte_idx..end_idx, "");

        if line_start < state.caret.offset {
            removed_before_caret += removal.min(state.caret.offset - line_start);
        }
        if line_start < state.selection.anchor {
            removed_before_anchor += removal.min(state.selection.anchor - line_start);
        }
        if line_start < state.selection.focus {
            removed_before_focus += removal.min(state.selection.focus - line_start);
        }
    }

    let new_caret = state.caret.offset.saturating_sub(removed_before_caret);
    EditorCommandResult {
        text: result,
        state: EditorSurfaceState {
            caret: EditorCaret { offset: new_caret },
            selection: EditorSelection {
                anchor: state.selection.anchor.saturating_sub(removed_before_anchor),
                focus: state.selection.focus.saturating_sub(removed_before_focus),
            },
            scroll_window: state.scroll_window.clone(),
        },
    }
}

fn selected_line_starts(text: &str, selection: &EditorSelection) -> Vec<usize> {
    let mut starts = line_start_offsets(text);
    let start = line_start_for_offset(text, selection.start());
    let end = line_start_for_offset(text, selection.end());
    starts.retain(|offset| *offset >= start && *offset <= end);
    if starts.is_empty() {
        vec![0]
    } else {
        starts
    }
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in text.chars().enumerate() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_start_for_offset(text: &str, offset: usize) -> usize {
    let mut current = 0usize;
    for (idx, ch) in text.chars().enumerate() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            current = idx + 1;
        }
    }
    current
}

fn leading_spaces_at(text: &str, line_start: usize) -> usize {
    text.chars()
        .skip(line_start)
        .take_while(|ch| *ch == ' ')
        .count()
}

fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .map(|(idx, _)| idx)
        .nth(char_idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::editor::state::{EditorScrollWindow, EditorSelection, EditorSurfaceState};

    #[test]
    fn move_caret_commands_stay_in_bounds() {
        let text = "=SUM(1,2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 1 },
            selection: EditorSelection::collapsed(1),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
        };

        let moved_left = apply_editor_command(text, &state, EditorCommand::MoveCaretLeft);
        assert_eq!(moved_left.state.caret.offset, 0);

        let moved_right = apply_editor_command(text, &moved_left.state, EditorCommand::MoveCaretRight);
        assert_eq!(moved_right.state.caret.offset, 1);
    }

    #[test]
    fn indent_with_spaces_inserts_four_spaces_on_selected_lines() {
        let text = "SUM(\n1,\n2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 3 },
            selection: EditorSelection { anchor: 0, focus: text.chars().count() },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
        };

        let result = apply_editor_command(text, &state, EditorCommand::IndentWithSpaces);
        assert!(result.text.starts_with("    SUM("));
        assert!(result.text.contains("\n    1,"));
        assert!(result.text.contains("\n    2)"));
    }

    #[test]
    fn outdent_with_spaces_removes_up_to_four_spaces_per_line() {
        let text = "    SUM(\n    1,\n    2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 4 },
            selection: EditorSelection { anchor: 0, focus: text.chars().count() },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
        };

        let result = apply_editor_command(text, &state, EditorCommand::OutdentWithSpaces);
        assert!(result.text.starts_with("SUM("));
        assert!(result.text.contains("\n1,"));
        assert!(result.text.contains("\n2)"));
    }

    #[test]
    fn keydown_mapping_recognizes_tab_and_arrow_commands() {
        assert_eq!(keydown_to_command("ArrowLeft", false), Some(EditorCommand::MoveCaretLeft));
        assert_eq!(keydown_to_command("ArrowRight", false), Some(EditorCommand::MoveCaretRight));
        assert_eq!(keydown_to_command("Tab", false), Some(EditorCommand::IndentWithSpaces));
        assert_eq!(keydown_to_command("Tab", true), Some(EditorCommand::OutdentWithSpaces));
    }
}
