use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::FormulaSpaceState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::components::app_shell::OneCalcShellApp;
use dnaonecalc_host::ui::editor::geometry::{
    EditorMeasuredOverlayBox, EditorOverlayGeometrySnapshot,
};
use leptos::prelude::*;

#[test]
fn ex_28_browser_measurement_path_surfaces_dom_measured_overlay_geometry() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = dnaonecalc_host::state::OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());

    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=LET(x,1,x)");
    formula_space.editor_document = Some(sample_editor_document("=LET(x,1,x)"));
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    formula_space.editor_overlay_geometry = Some(EditorOverlayGeometrySnapshot {
        caret_box: Some(EditorMeasuredOverlayBox {
            top_px: 48,
            left_px: 72,
            width_px: 2,
            height_px: 22,
            line_index: 0,
            column_index: 4,
        }),
        selection_box: Some(EditorMeasuredOverlayBox {
            top_px: 48,
            left_px: 24,
            width_px: 48,
            height_px: 22,
            line_index: 0,
            column_index: 1,
        }),
        completion_anchor_box: Some(EditorMeasuredOverlayBox {
            top_px: 70,
            left_px: 24,
            width_px: 48,
            height_px: 22,
            line_index: 0,
            column_index: 1,
        }),
        signature_help_anchor_box: Some(EditorMeasuredOverlayBox {
            top_px: 92,
            left_px: 0,
            width_px: 88,
            height_px: 22,
            line_index: 0,
            column_index: 0,
        }),
    });
    formula_space.effective_display_summary = Some("1".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-measurement-source=\"mixed\""));
    assert!(html.contains("data-caret-measurement-source=\"dom-measured\""));
    assert!(html.contains("data-anchor-measurement-source=\"dom-measured\""));
    assert!(html.contains("data-caret-top-px=\"48\""));
    assert!(html.contains("data-role=\"selected-completion-summary\""));
    assert!(html.contains("data-role=\"help-sync-lookup\""));
}
