//! S2 / S3 / S4 / S5 / S12 — formula entry scenarios.
//!
//! Each test dispatches `apply_live_editor_input` against a real
//! `LiveOxfmlBridge` (or a `FakeBridge` where the live bridge can't
//! yet return the richer document a scenario needs), then walks the
//! full projection chain down to the Explore clusters.

use dnaonecalc_host::adapters::oxfml::{
    EditorDocument, FormulaEditRequest, FormulaEditResult, LiveDiagnostic, LiveDiagnosticSnapshot,
    OxfmlEditorBridge, OxfmlEditorBridgeError,
};
use dnaonecalc_host::services::live_edit::apply_live_editor_input;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};
use dnaonecalc_host::ui::editor::state::EditorLiveState;

use super::fixtures::{
    explore_editor_cluster, explore_projection, explore_result_cluster,
    fresh_state_with_active_space, scenario_bridge, type_formula,
};

#[test]
fn typing_a_sum_formula_shows_the_numeric_result() {
    // S2 / S3: type `=SUM(1,2,3)` and see `6` in the result cluster.
    // Drives the full OxFml + OxFunc runtime through `LiveOxfmlBridge` so
    // the assertion is on a real evaluation, not a fallback hand-eval.
    let (mut state, _space) = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SUM(1,2,3)");

    let explore = explore_projection(&state);
    let result = explore_result_cluster(&explore);
    assert_eq!(result.effective_display_summary.as_deref(), Some("6"));
    assert!(
        result
            .latest_evaluation_summary
            .as_deref()
            .is_some_and(|summary| summary.contains('6')),
        "evaluation summary should reference the computed value, got {:?}",
        result.latest_evaluation_summary,
    );
}

#[test]
fn typing_a_two_arg_sum_shows_the_addition_result() {
    // S2: type `=SUM(1,1)` and see `2` through the real engine.
    let (mut state, _space) = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SUM(1,1)");

    let explore = explore_projection(&state);
    let result = explore_result_cluster(&explore);
    assert_eq!(result.effective_display_summary.as_deref(), Some("2"));
}

#[test]
fn typing_a_sequence_formula_shows_the_two_by_two_array_preview() {
    // S4: type `=SEQUENCE(2,2)` and see a 2×2 array preview with 1..4
    // computed by the real OxFunc dynamic array path.
    let (mut state, _space) = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SEQUENCE(2,2)");

    let explore = explore_projection(&state);
    let result = explore_result_cluster(&explore);
    let preview = result
        .array_preview
        .expect("sequence scenario should populate array preview");
    assert_eq!(preview.rows.len(), 2);
    assert!(preview.rows.iter().all(|row| row.len() == 2));
    assert_eq!(preview.rows[0], vec!["1".to_string(), "2".to_string()]);
    assert_eq!(preview.rows[1], vec!["3".to_string(), "4".to_string()]);
}

struct DiagnosticFakeBridge {
    document: EditorDocument,
}

impl OxfmlEditorBridge for DiagnosticFakeBridge {
    fn apply_formula_edit(
        &self,
        _request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        Ok(FormulaEditResult {
            document: self.document.clone(),
        })
    }
}

#[test]
fn typing_an_invalid_formula_surfaces_a_diagnostic_in_the_cluster() {
    // S5: type `=SUM(` with an unclosed paren. The live bridge does not
    // currently emit diagnostics, so this scenario uses a fake bridge that
    // returns a document with a populated `live_diagnostics` snapshot. The
    // assertion is that the diagnostic reaches the editor cluster and the
    // result cluster falls back to the "Input incomplete" summary derived by
    // `derive_formula_presentation`.
    let (mut state, _space) = fresh_state_with_active_space();
    let mut document = sample_editor_document("=SUM(");
    document.live_diagnostics = LiveDiagnosticSnapshot {
        diagnostics: vec![LiveDiagnostic {
            diagnostic_id: "diag-1".to_string(),
            message: "unmatched '('".to_string(),
            span_start: 4,
            span_len: 1,
        }],
    };
    let bridge = DiagnosticFakeBridge { document };

    apply_live_editor_input(
        &bridge,
        &mut state,
        EditorInputEvent {
            text: "=SUM(".to_string(),
            selection_start: Some(5),
            selection_end: Some(5),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some("=SUM(".to_string()),
        },
    )
    .expect("fake bridge always succeeds");

    let explore = explore_projection(&state);
    let editor = explore_editor_cluster(&explore);
    let result = explore_result_cluster(&explore);
    assert_eq!(editor.diagnostics.len(), 1);
    assert_eq!(editor.diagnostics[0].message, "unmatched '('");
    assert_eq!(
        result.effective_display_summary.as_deref(),
        Some("Input incomplete"),
    );
}

#[test]
fn rapid_typing_preserves_the_latest_input_without_stale_state() {
    // S12: dispatch three sequential input events through the live
    // bridge. After the third, the cluster must reflect the third input
    // exactly, the final evaluation must be the real OxFml+OxFunc result,
    // and no stale diagnostics from earlier inputs may remain.
    let (mut state, _space) = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=");
    type_formula(&bridge, &mut state, "=SU");
    type_formula(&bridge, &mut state, "=SUM(1,2,3)");

    let explore = explore_projection(&state);
    let editor = explore_editor_cluster(&explore);
    assert_eq!(editor.raw_entered_cell_text, "=SUM(1,2,3)");
    assert!(editor.diagnostics.is_empty());

    let result = explore_result_cluster(&explore);
    assert_eq!(result.effective_display_summary.as_deref(), Some("6"));
    // Live state: the user has typed but has not committed, so the
    // formula space should report EditingLive through the cluster.
    assert_eq!(editor.live_state, EditorLiveState::EditingLive);
}
