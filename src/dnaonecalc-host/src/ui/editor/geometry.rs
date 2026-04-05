use crate::adapters::oxfml::FormulaTextSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorOverlayMeasurementSource {
    DerivedGrid,
    DomMeasured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorLineColumn {
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorOverlayBox {
    pub start: EditorLineColumn,
    pub end: EditorLineColumn,
    pub top_px: usize,
    pub left_px: usize,
    pub width_px: usize,
    pub height_px: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorOverlayMeasurement {
    pub source: EditorOverlayMeasurementSource,
    pub char_width_px: usize,
    pub line_height_px: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMeasuredOverlayBox {
    pub top_px: usize,
    pub left_px: usize,
    pub width_px: usize,
    pub height_px: usize,
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorOverlayGeometrySnapshot {
    pub caret_box: Option<EditorMeasuredOverlayBox>,
    pub selection_box: Option<EditorMeasuredOverlayBox>,
    pub completion_anchor_box: Option<EditorMeasuredOverlayBox>,
    pub signature_help_anchor_box: Option<EditorMeasuredOverlayBox>,
}

impl EditorOverlayMeasurement {
    pub fn derived_grid() -> Self {
        Self {
            source: EditorOverlayMeasurementSource::DerivedGrid,
            char_width_px: 8,
            line_height_px: 22,
        }
    }

    pub fn offset_box(&self, text: &str, offset: usize) -> EditorOverlayBox {
        let start = offset_to_line_column(text, offset);
        EditorOverlayBox {
            start,
            end: start,
            top_px: start.line_index * self.line_height_px,
            left_px: start.column_index * self.char_width_px,
            width_px: self.char_width_px.max(1),
            height_px: self.line_height_px,
        }
    }

    pub fn span_box(&self, text: &str, span: FormulaTextSpan) -> EditorOverlayBox {
        let start = offset_to_line_column(text, span.start);
        let end = offset_to_line_column(text, span.start + span.len);
        let same_line = start.line_index == end.line_index;
        let column_span = if same_line {
            end.column_index.saturating_sub(start.column_index).max(1)
        } else {
            1
        };
        let line_span = end.line_index.saturating_sub(start.line_index) + 1;

        EditorOverlayBox {
            start,
            end,
            top_px: start.line_index * self.line_height_px,
            left_px: start.column_index * self.char_width_px,
            width_px: column_span * self.char_width_px,
            height_px: line_span * self.line_height_px,
        }
    }
}

pub fn offset_to_line_column(text: &str, offset: usize) -> EditorLineColumn {
    let mut line_index = 0;
    let mut column_index = 0;

    for (current_offset, ch) in text.chars().enumerate() {
        if current_offset == offset {
            return EditorLineColumn {
                line_index,
                column_index,
            };
        }

        if ch == '\n' {
            line_index += 1;
            column_index = 0;
        } else {
            column_index += 1;
        }
    }

    EditorLineColumn {
        line_index,
        column_index,
    }
}

pub fn resolve_overlay_box(
    measured_box: Option<EditorMeasuredOverlayBox>,
    derived_box: EditorOverlayBox,
) -> (EditorOverlayMeasurementSource, EditorOverlayBox) {
    match measured_box {
        Some(measured_box) => (
            EditorOverlayMeasurementSource::DomMeasured,
            EditorOverlayBox {
                start: EditorLineColumn {
                    line_index: measured_box.line_index,
                    column_index: measured_box.column_index,
                },
                end: EditorLineColumn {
                    line_index: measured_box.line_index,
                    column_index: measured_box.column_index,
                },
                top_px: measured_box.top_px,
                left_px: measured_box.left_px,
                width_px: measured_box.width_px,
                height_px: measured_box.height_px,
            },
        ),
        None => (EditorOverlayMeasurementSource::DerivedGrid, derived_box),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_to_line_column_tracks_multiline_positions() {
        let text = "=LET(\n  x, 1,\n  x)";
        assert_eq!(
            offset_to_line_column(text, 0),
            EditorLineColumn {
                line_index: 0,
                column_index: 0,
            }
        );
        assert_eq!(
            offset_to_line_column(text, 6),
            EditorLineColumn {
                line_index: 1,
                column_index: 0,
            }
        );
        assert_eq!(
            offset_to_line_column(text, text.chars().count()),
            EditorLineColumn {
                line_index: 2,
                column_index: 4,
            }
        );
    }

    #[test]
    fn derived_grid_span_box_produces_line_and_pixel_geometry() {
        let measurement = EditorOverlayMeasurement::derived_grid();
        let text = "=SUM(\n  1,\n  2)";
        let span_box = measurement.span_box(text, FormulaTextSpan { start: 6, len: 2 });

        assert_eq!(span_box.start.line_index, 1);
        assert_eq!(span_box.start.column_index, 0);
        assert_eq!(span_box.top_px, 22);
        assert_eq!(span_box.left_px, 0);
        assert_eq!(span_box.height_px, 22);
        assert_eq!(span_box.width_px, 16);
    }

    #[test]
    fn measured_box_is_preferred_over_derived_geometry() {
        let derived_box = EditorOverlayMeasurement::derived_grid().offset_box("=SUM(1,2)", 4);
        let (source, resolved_box) = resolve_overlay_box(
            Some(EditorMeasuredOverlayBox {
                top_px: 120,
                left_px: 48,
                width_px: 14,
                height_px: 20,
                line_index: 3,
                column_index: 6,
            }),
            derived_box,
        );

        assert_eq!(source, EditorOverlayMeasurementSource::DomMeasured);
        assert_eq!(resolved_box.top_px, 120);
        assert_eq!(resolved_box.left_px, 48);
        assert_eq!(resolved_box.start.line_index, 3);
        assert_eq!(resolved_box.start.column_index, 6);
    }
}
