//! SEAM-ONECALC-EXTENDED-VALUE-ROUTING
//! SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED
//!
//! Target: route upstream `EvalValue` / `ExtendedValue` / `PresentationHint`
//! through `FormulaSpaceState` so the result cluster renders structurally.
//! Today `derive_formula_presentation` hand-evaluates a few patterns; the
//! real typed value path is unimplemented.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-EXTENDED-VALUE-ROUTING: typing `=SUM(1,2,3)` must
/// produce a typed `ExtendedValue::Number(6)` on the formula space, not
/// just a derived string.
///
/// Passes when `FormulaSpaceState.value_presentation` carries a typed
/// value family (Number / Array / Text / Error) and the cluster's
/// `latest_evaluation_summary` reflects the typed family label.
///
/// Ownership: flipped to green when the OxFml live bridge delivers typed
/// values to OneCalc and `editor_session.rs` propagates them.
#[test]
#[ignore = "pending SEAM-ONECALC-EXTENDED-VALUE-ROUTING"]
fn value_presentation_carries_typed_number_after_sum_round_trip() {
    seam_pending(
        "SEAM-ONECALC-EXTENDED-VALUE-ROUTING",
        "typed ExtendedValue::Number(6) must reach FormulaSpaceState.value_presentation",
    );
}

/// Pending SEAM-ONECALC-EXTENDED-VALUE-ROUTING: typing `=SEQUENCE(2,2)`
/// must produce a typed `ExtendedValue::Array` with the typed element
/// shape, not a local string mirror.
///
/// Passes when the cluster's `array_preview` comes from a typed array
/// value rather than a OneCalc-local derived preview.
#[test]
#[ignore = "pending SEAM-ONECALC-EXTENDED-VALUE-ROUTING"]
fn value_presentation_carries_typed_array_after_sequence_round_trip() {
    seam_pending(
        "SEAM-ONECALC-EXTENDED-VALUE-ROUTING",
        "typed ExtendedValue::Array must reach FormulaSpaceState.value_presentation for =SEQUENCE",
    );
}

/// Pending SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED: `RetainedArtifactRecord`
/// carries Excel observations as `serde_json::Value`; an adapter should
/// lift them into `ExtendedValue` so the Value Panel renders structurally.
///
/// Passes when opening an Excel-sourced artifact surfaces a typed value on
/// the Workbench projection instead of raw JSON.
#[test]
#[ignore = "pending SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED"]
fn excel_observation_summary_lifts_into_extended_value() {
    seam_pending(
        "SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED",
        "excel_comparison_value must lift into a typed ExtendedValue for projection",
    );
}
