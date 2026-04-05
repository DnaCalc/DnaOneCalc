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
}

impl EditorSurfaceState {
    pub fn for_text(text: &str) -> Self {
        let offset = text.chars().count();
        let line_count = text.lines().count().max(1);
        Self {
            caret: EditorCaret { offset },
            selection: EditorSelection::collapsed(offset),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: line_count.min(12),
            },
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
    }
}
