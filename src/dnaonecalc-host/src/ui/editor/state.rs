#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCaret {
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl EditorSelection {
    pub fn collapsed(offset: usize) -> Self {
        Self {
            anchor: offset,
            focus: offset,
        }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.focus)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.focus)
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorScrollWindow {
    pub first_visible_line: usize,
    pub visible_line_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSurfaceState {
    pub caret: EditorCaret,
    pub selection: EditorSelection,
    pub scroll_window: EditorScrollWindow,
    pub completion_anchor_offset: Option<usize>,
    pub signature_help_anchor_offset: Option<usize>,
}

impl EditorSurfaceState {
    pub fn for_text(text: &str) -> Self {
        let offset = text.chars().count();
        Self::for_text_with_selection(text, offset, offset)
    }

    pub fn for_text_with_selection(text: &str, anchor: usize, focus: usize) -> Self {
        let text_len = text.chars().count();
        let anchor = anchor.min(text_len);
        let focus = focus.min(text_len);
        let line_count = text.lines().count().max(1);
        Self {
            caret: EditorCaret { offset: focus },
            selection: EditorSelection { anchor, focus },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: line_count.min(12),
            },
            completion_anchor_offset: None,
            signature_help_anchor_offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_surface_state_defaults_to_end_of_text() {
        let state = EditorSurfaceState::for_text("=SUM(1,2)");
        assert_eq!(state.caret.offset, 9);
        assert!(state.selection.is_collapsed());
        assert_eq!(state.scroll_window.first_visible_line, 0);
        assert!(state.completion_anchor_offset.is_none());
        assert!(state.signature_help_anchor_offset.is_none());
    }

    #[test]
    fn editor_surface_state_can_preserve_explicit_selection() {
        let state = EditorSurfaceState::for_text_with_selection("=SUM(1,2)", 2, 5);
        assert_eq!(state.selection.anchor, 2);
        assert_eq!(state.selection.focus, 5);
        assert_eq!(state.caret.offset, 5);
        assert!(state.completion_anchor_offset.is_none());
        assert!(state.signature_help_anchor_offset.is_none());
    }
}
