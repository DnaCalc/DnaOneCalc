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
    formula_space.editor_surface_state.completion_anchor_offset = Some(4);
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    formula_space.editor_surface_state.signature_help_anchor_offset = Some(4);
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
        completion_popup_box: Some(EditorMeasuredOverlayBox {
            top_px: 70,
            left_px: 24,
            width_px: 48,
            height_px: 22,
            line_index: 0,
            column_index: 1,
        }),
        signature_help_popup_box: Some(EditorMeasuredOverlayBox {
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
    assert!(html.contains("data-role=\"completion-popup-container\""));
    assert!(html.contains("data-role=\"signature-help-popup-container\""));
}

#[test]
fn multiline_completion_popup_uses_measured_box_position() {
    let formula_space_id = FormulaSpaceId::new("space-2");
    let mut state = dnaonecalc_host::state::OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());

    let multiline = "=LET(\n  alpha,\n  alpha)";
    let mut formula_space = FormulaSpaceState::new(formula_space_id, multiline);
    formula_space.editor_document = Some(sample_editor_document(multiline));
    formula_space.editor_surface_state.completion_anchor_offset = Some(8);
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    formula_space.editor_overlay_geometry = Some(EditorOverlayGeometrySnapshot {
        caret_box: Some(EditorMeasuredOverlayBox {
            top_px: 88,
            left_px: 64,
            width_px: 2,
            height_px: 22,
            line_index: 2,
            column_index: 7,
        }),
        selection_box: None,
        completion_anchor_box: Some(EditorMeasuredOverlayBox {
            top_px: 110,
            left_px: 18,
            width_px: 45,
            height_px: 22,
            line_index: 1,
            column_index: 2,
        }),
        signature_help_anchor_box: None,
        completion_popup_box: Some(EditorMeasuredOverlayBox {
            top_px: 110,
            left_px: 18,
            width_px: 45,
            height_px: 22,
            line_index: 1,
            column_index: 2,
        }),
        signature_help_popup_box: None,
    });
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-role=\"completion-popup-container\""));
    assert!(html.contains("data-popup-line=\"1\""));
    assert!(html.contains("top:132px;left:18px;"));
}

#[test]
fn multiline_signature_popup_uses_measured_box_position() {
    let formula_space_id = FormulaSpaceId::new("space-3");
    let mut state = dnaonecalc_host::state::OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());

    let multiline = "=SUM(\n  1,\n  2)";
    let mut formula_space = FormulaSpaceState::new(formula_space_id, multiline);
    formula_space.editor_document = Some(sample_editor_document(multiline));
    formula_space.editor_surface_state.signature_help_anchor_offset = Some(5);
    formula_space.editor_overlay_geometry = Some(EditorOverlayGeometrySnapshot {
        caret_box: None,
        selection_box: None,
        completion_anchor_box: None,
        signature_help_anchor_box: Some(EditorMeasuredOverlayBox {
            top_px: 132,
            left_px: 6,
            width_px: 80,
            height_px: 22,
            line_index: 0,
            column_index: 0,
        }),
        completion_popup_box: None,
        signature_help_popup_box: Some(EditorMeasuredOverlayBox {
            top_px: 132,
            left_px: 6,
            width_px: 80,
            height_px: 22,
            line_index: 0,
            column_index: 0,
        }),
    });
    state.formula_spaces.insert(formula_space);

    let html = view! { <OneCalcShellApp initial_state=state /> }.to_html();

    assert!(html.contains("data-role=\"signature-help-popup-container\""));
    assert!(html.contains("data-popup-line=\"0\""));
    assert!(html.contains("top:154px;left:6px;"));
}
