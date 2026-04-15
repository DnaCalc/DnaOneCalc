//! SEAM-ONECALC-RAIL-INLINE-RENAME
//!
//! Target: implement inline rename of a formula space from the rail:
//! `BeginInlineRenameFormulaSpace` / `CommitInlineRename` reducer actions
//! that update the shell frame rail.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-RAIL-INLINE-RENAME: the reducer must expose
/// inline rename actions that surface on `ShellFormulaSpaceListItemViewModel`
/// as an editing state.
///
/// Passes when `begin_inline_rename_formula_space` flips a rail item to
/// an editing state and `commit_inline_rename_formula_space(new_label)`
/// commits the new label to `FormulaSpaceContextState.scenario_label`.
///
/// Ownership: Phase B step 3 (shell frame grammar) bead.
#[test]
#[ignore = "pending SEAM-ONECALC-RAIL-INLINE-RENAME"]
fn begin_and_commit_inline_rename_updates_shell_frame_rail() {
    seam_pending(
        "SEAM-ONECALC-RAIL-INLINE-RENAME",
        "begin/commit inline rename reducer actions must update ShellFormulaSpaceListItemViewModel",
    );
}
