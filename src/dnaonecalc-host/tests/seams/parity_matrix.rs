//! SEAM-ONECALC-PARITY-MATRIX-VIEW
//!
//! Target: `WorkbenchViewModel` exposes a structured parity matrix (value /
//! display / replay rows × scenario columns) rather than the current flat
//! `outcome_summary` / `evidence_summary` / `lineage_items` strings.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-PARITY-MATRIX-VIEW: the workbench view model must
/// build a typed parity matrix from a `RetainedArtifactRecord`, not a flat
/// string list.
///
/// Passes when `WorkbenchViewModel.parity_matrix` is populated with rows
/// keyed by parity axis (value / display / replay) and columns keyed by
/// scenario label, each cell carrying an explicit verdict enum.
///
/// Ownership: Phase B step 5 (Parity Matrix in Workbench) bead.
#[test]
#[ignore = "pending SEAM-ONECALC-PARITY-MATRIX-VIEW"]
fn workbench_view_model_exposes_structured_parity_matrix() {
    seam_pending(
        "SEAM-ONECALC-PARITY-MATRIX-VIEW",
        "WorkbenchViewModel must carry a structured ParityMatrix, not flat outcome_summary strings",
    );
}

/// Pending SEAM-ONECALC-PARITY-MATRIX-VIEW: mismatched retained artifacts
/// must surface per-axis verdict cells that the cluster can render with
/// colour and link.
#[test]
#[ignore = "pending SEAM-ONECALC-PARITY-MATRIX-VIEW"]
fn parity_matrix_cells_carry_verdict_enum_not_strings() {
    seam_pending(
        "SEAM-ONECALC-PARITY-MATRIX-VIEW",
        "ParityMatrix cells must carry a typed verdict enum (Matched / Mismatched / Blocked)",
    );
}
