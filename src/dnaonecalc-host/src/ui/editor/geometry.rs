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
    pub completion_popup_box: Option<EditorMeasuredOverlayBox>,
    pub signature_help_popup_box: Option<EditorMeasuredOverlayBox>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorOverlayMeasurementEvent {
    pub snapshot: EditorOverlayGeometrySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextareaMeasurementMetrics {
    pub char_width_px: usize,
    pub line_height_px: usize,
    pub scroll_top_px: usize,
    pub scroll_left_px: usize,
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

pub fn derive_overlay_snapshot(
    text: &str,
    caret_offset: usize,
    selection_span: FormulaTextSpan,
    completion_anchor_span: Option<FormulaTextSpan>,
    signature_help_span: Option<FormulaTextSpan>,
) -> EditorOverlayGeometrySnapshot {
    let measurement = EditorOverlayMeasurement::derived_grid();

    EditorOverlayGeometrySnapshot {
        caret_box: Some(measured_box_from_overlay_box(
            measurement.offset_box(text, caret_offset),
        )),
        selection_box: Some(measured_box_from_overlay_box(
            measurement.span_box(text, selection_span),
        )),
        completion_anchor_box: completion_anchor_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        signature_help_anchor_box: signature_help_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        completion_popup_box: completion_anchor_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        signature_help_popup_box: signature_help_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
    }
}

pub fn derive_overlay_snapshot_with_metrics(
    text: &str,
    caret_offset: usize,
    selection_span: FormulaTextSpan,
    completion_anchor_span: Option<FormulaTextSpan>,
    signature_help_span: Option<FormulaTextSpan>,
    metrics: TextareaMeasurementMetrics,
) -> EditorOverlayGeometrySnapshot {
    let measurement = EditorOverlayMeasurement {
        source: EditorOverlayMeasurementSource::DomMeasured,
        char_width_px: metrics.char_width_px.max(1),
        line_height_px: metrics.line_height_px.max(1),
    };

    EditorOverlayGeometrySnapshot {
        caret_box: Some(measured_box_from_overlay_box(adjust_for_scroll(
            measurement.offset_box(text, caret_offset),
            metrics,
        ))),
        selection_box: Some(measured_box_from_overlay_box(adjust_for_scroll(
            measurement.span_box(text, selection_span),
            metrics,
        ))),
        completion_anchor_box: completion_anchor_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        signature_help_anchor_box: signature_help_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        completion_popup_box: completion_anchor_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        signature_help_popup_box: signature_help_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
    }
}

fn adjust_for_scroll(
    mut box_geometry: EditorOverlayBox,
    metrics: TextareaMeasurementMetrics,
) -> EditorOverlayBox {
    box_geometry.top_px = box_geometry.top_px.saturating_sub(metrics.scroll_top_px);
    box_geometry.left_px = box_geometry.left_px.saturating_sub(metrics.scroll_left_px);
    box_geometry
}

fn measured_box_from_overlay_box(box_geometry: EditorOverlayBox) -> EditorMeasuredOverlayBox {
    EditorMeasuredOverlayBox {
        top_px: box_geometry.top_px,
        left_px: box_geometry.left_px,
        width_px: box_geometry.width_px,
        height_px: box_geometry.height_px,
        line_index: box_geometry.start.line_index,
        column_index: box_geometry.start.column_index,
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

    #[test]
    fn derived_overlay_snapshot_captures_caret_selection_and_assist_boxes() {
        let snapshot = derive_overlay_snapshot(
            "=SUM(1,2)",
            4,
            FormulaTextSpan { start: 1, len: 3 },
            Some(FormulaTextSpan { start: 1, len: 3 }),
            Some(FormulaTextSpan { start: 0, len: 9 }),
        );

        assert_eq!(snapshot.caret_box.as_ref().map(|box_geometry| box_geometry.left_px), Some(32));
        assert_eq!(snapshot.selection_box.as_ref().map(|box_geometry| box_geometry.width_px), Some(24));
        assert_eq!(snapshot.completion_anchor_box.as_ref().map(|box_geometry| box_geometry.column_index), Some(1));
        assert_eq!(snapshot.signature_help_anchor_box.as_ref().map(|box_geometry| box_geometry.width_px), Some(72));
        assert_eq!(snapshot.completion_popup_box.as_ref().map(|box_geometry| box_geometry.column_index), Some(1));
        assert_eq!(snapshot.signature_help_popup_box.as_ref().map(|box_geometry| box_geometry.width_px), Some(72));
    }

    #[test]
    fn dom_metric_overlay_snapshot_accounts_for_scroll_and_multiline_offsets() {
        let snapshot = derive_overlay_snapshot_with_metrics(
            "=LET(\n  alpha,\n  beta,\n  alpha)",
            15,
            FormulaTextSpan { start: 6, len: 7 },
            Some(FormulaTextSpan { start: 6, len: 5 }),
            Some(FormulaTextSpan { start: 0, len: 31 }),
            TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 20,
                scroll_top_px: 20,
                scroll_left_px: 9,
            },
        );

        assert_eq!(snapshot.caret_box.as_ref().map(|box_geometry| box_geometry.top_px), Some(20));
        assert_eq!(snapshot.caret_box.as_ref().map(|box_geometry| box_geometry.left_px), Some(0));
        assert_eq!(snapshot.completion_anchor_box.as_ref().map(|box_geometry| box_geometry.top_px), Some(0));
        assert_eq!(snapshot.signature_help_anchor_box.as_ref().map(|box_geometry| box_geometry.height_px), Some(80));
        assert_eq!(snapshot.completion_popup_box.as_ref().map(|box_geometry| box_geometry.top_px), Some(0));
        assert_eq!(snapshot.signature_help_popup_box.as_ref().map(|box_geometry| box_geometry.height_px), Some(80));
    }
}
