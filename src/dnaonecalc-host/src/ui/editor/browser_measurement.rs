use crate::ui::panels::explore::ExploreEditorClusterViewModel;

#[cfg(target_arch = "wasm32")]
use crate::ui::editor::geometry::{
    derive_overlay_snapshot_with_metrics, EditorOverlayMeasurementEvent, TextareaMeasurementMetrics,
};
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlTextAreaElement;

#[cfg(target_arch = "wasm32")]
pub fn capture_overlay_measurement_event(
    textarea: &HtmlTextAreaElement,
    editor: &ExploreEditorClusterViewModel,
) -> EditorOverlayMeasurementEvent {
    let rows = textarea.rows().max(1) as usize;
    let cols = textarea.cols().max(1) as usize;
    let client_height = textarea.client_height().max(1) as usize;
    let client_width = textarea.client_width().max(1) as usize;

    let metrics = TextareaMeasurementMetrics {
        char_width_px: (client_width / cols).max(1),
        line_height_px: (client_height / rows).max(1),
        scroll_top_px: textarea.scroll_top().max(0) as usize,
        scroll_left_px: textarea.scroll_left().max(0) as usize,
    };

    EditorOverlayMeasurementEvent {
        snapshot: derive_overlay_snapshot_with_metrics(
            &editor.raw_entered_cell_text,
            editor.editor_surface_state.caret.offset,
            crate::adapters::oxfml::FormulaTextSpan {
                start: editor.editor_surface_state.selection.start(),
                len: editor
                    .editor_surface_state
                    .selection
                    .end()
                    .saturating_sub(editor.editor_surface_state.selection.start()),
            },
            editor.completion_anchor_span,
            editor.signature_help.as_ref().map(|help| help.call_span),
            metrics,
        ),
    }
}

#[cfg(not(target_arch = "wasm32"))]
use crate::ui::editor::geometry::{derive_overlay_snapshot, EditorOverlayMeasurementEvent};

#[cfg(not(target_arch = "wasm32"))]
pub fn capture_overlay_measurement_event(
    _textarea: &(),
    editor: &ExploreEditorClusterViewModel,
) -> EditorOverlayMeasurementEvent {
    EditorOverlayMeasurementEvent {
        snapshot: derive_overlay_snapshot(
            &editor.raw_entered_cell_text,
            editor.editor_surface_state.caret.offset,
            crate::adapters::oxfml::FormulaTextSpan {
                start: editor.editor_surface_state.selection.start(),
                len: editor
                    .editor_surface_state
                    .selection
                    .end()
                    .saturating_sub(editor.editor_surface_state.selection.start()),
            },
            editor.completion_anchor_span,
            editor.signature_help.as_ref().map(|help| help.call_span),
        ),
    }
}
