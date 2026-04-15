//! SEAM-ONECALC-PERSISTENCE-V1
//!
//! Target: `persistence::save_workspace` + `load_workspace` must round-trip
//! a workspace across sessions. Today `persistence/mod.rs` is a placeholder.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-PERSISTENCE-V1: saving a workspace with two
/// formula spaces and a retained artifact, then loading it, must
/// reconstruct the same `OneCalcHostState` shape.
///
/// Passes when `save_workspace(&state)` produces v1 JSON and
/// `load_workspace(&json)` returns a `OneCalcHostState` that is
/// structurally equal (modulo ephemeral editor cache fields) to the
/// original.
///
/// Ownership: Phase B step 4 (workspace persistence and case lifecycle).
#[test]
#[ignore = "pending SEAM-ONECALC-PERSISTENCE-V1"]
fn workspace_json_v1_round_trips_two_spaces_and_retained_artifacts() {
    seam_pending(
        "SEAM-ONECALC-PERSISTENCE-V1",
        "save_workspace + load_workspace must round-trip two spaces and a retained artifact",
    );
}

/// Pending SEAM-ONECALC-PERSISTENCE-V1: recent-spaces tracking. No
/// `recent_formula_space_ids` field exists today.
#[test]
#[ignore = "pending SEAM-ONECALC-PERSISTENCE-V1"]
fn recent_formula_space_ids_carry_across_a_save_load_cycle() {
    seam_pending(
        "SEAM-ONECALC-PERSISTENCE-V1",
        "WorkspaceShellState must carry recent_formula_space_ids and round-trip through v1 JSON",
    );
}
