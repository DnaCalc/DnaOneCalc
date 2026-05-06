//! Workspace-level persistence for the browser host.
//!
//! Per `docs/PERSISTENCE_FORMAT_PLAN.md` §1A.2, `workspace.json`
//! lives at `localStorage["dnaonecalc.workspace.v1"]` on the
//! browser host (and at a per-user app-data path on the eventual
//! Tauri host). It carries app-state — pinned formula ids, the
//! active formula's last-edited content, the recents list — that
//! survives reload but is **not** part of any single user
//! document.
//!
//! Slice 1 of `SEAM-ONECALC-SCENARIO-PERSIST` ships the minimum
//! viable surface:
//!
//! - **Pinned ids** — survive reload.
//! - **Active formula snapshot** — last-edited text + formatting
//!   round-trip back into the workspace on the next load.
//!
//! Recents-with-full-state (the larger surface that carries N
//! recently-closed formulas across reload) is the obvious next
//! slice. Until then the recents list is in-memory only — it
//! tracks the current session, not history-across-reloads.
//!
//! The persistence format is JSON with a stable `version: 1` field
//! at the root. Inside the JSON, the active formula is itself
//! serialised as the same `.dnafml` XML the host already emits via
//! `write_formula_xml` — so workspace persistence reuses the
//! formula-file round-trip rather than re-stating its shape.

use crate::persistence::{
    apply_loaded_scenario_to_formula_space, formula_space_to_scenario, read_formula_xml,
    write_formula_xml, FormulaFileError,
};
use crate::state::{FormulaSpaceState, OneCalcHostState};
use serde::{Deserialize, Serialize};

/// Stable storage key. Bumped only on incompatible schema changes
/// — the JSON's `version` field is the soft compatibility lever
/// for additive changes within a key generation.
pub const WORKSPACE_STORAGE_KEY: &str = "dnaonecalc.workspace.v1";

/// Top-level workspace.json envelope. Versioned so the loader can
/// reject futures it doesn't understand and tolerate older shapes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceJson {
    /// Schema version. Currently `1`. Bumped on incompatible
    /// changes; the loader returns `Err` for any value it does not
    /// recognise.
    pub version: u32,
    /// Pinned formula ids in stable user-visible order. Reloading
    /// the workspace re-inserts each id into
    /// `workspace_shell.pinned_formula_space_ids` — those that are
    /// still present in `formula_spaces` (i.e. the active formula
    /// or a snapshot we restored) flip to the pinned section of
    /// the breadcrumb dropdown; ids without a matching formula are
    /// silently dropped on the way back in.
    #[serde(default)]
    pub pinned_formula_space_ids: Vec<String>,
    /// Optional active-formula snapshot. Stored as `.dnafml` XML
    /// so we reuse the existing formula-file round-trip rather
    /// than maintaining a parallel JSON shape. `None` when the
    /// workspace had no active formula at the time of write.
    #[serde(default)]
    pub active_formula_xml: Option<String>,
}

impl WorkspaceJson {
    /// Project the live host state into a workspace.json envelope.
    /// Pinned ids round-trip verbatim; the active formula is
    /// serialised through the existing `Scenario` → XML path so
    /// formatting / CF rules / scenario policy survive untouched.
    pub fn from_state(state: &OneCalcHostState) -> Self {
        let pinned_formula_space_ids = state
            .workspace_shell
            .pinned_formula_space_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let active_formula_xml = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|formula_space| {
                let scenario =
                    formula_space_to_scenario(formula_space, String::new(), String::new());
                write_formula_xml(&scenario)
            });
        Self {
            version: 1,
            pinned_formula_space_ids,
            active_formula_xml,
        }
    }

    /// Apply the workspace.json envelope back onto a fresh host
    /// state. Restores the active formula's text + formatting AND
    /// re-inserts the pinned id set. Pin entries whose formula
    /// isn't restored simply remain in the pinned set — the
    /// breadcrumb dropdown handles missing-target ids gracefully
    /// by surfacing them in the Pinned list with a placeholder
    /// label.
    pub fn apply_to_state(self, state: &mut OneCalcHostState) -> Result<(), WorkspaceLoadError> {
        if self.version != 1 {
            return Err(WorkspaceLoadError::UnsupportedVersion(self.version));
        }
        // Re-insert pins. Use `FormulaSpaceId::new` so the
        // BTreeSet ordering is identical to a fresh insert.
        state.workspace_shell.pinned_formula_space_ids.clear();
        for id in self.pinned_formula_space_ids {
            state
                .workspace_shell
                .pinned_formula_space_ids
                .insert(crate::domain::ids::FormulaSpaceId::new(id));
        }
        // Restore the active formula's content + formatting on
        // top of the workspace's existing first formula space (the
        // host always boots with a fresh `untitled-1`; we apply
        // the loaded scenario to that target).
        if let Some(xml) = self.active_formula_xml {
            let loaded = read_formula_xml(&xml).map_err(WorkspaceLoadError::FormulaFile)?;
            let target_id = state
                .workspace_shell
                .active_formula_space_id
                .clone()
                .or_else(|| {
                    state
                        .workspace_shell
                        .open_formula_space_order
                        .first()
                        .cloned()
                });
            let Some(target_id) = target_id else {
                return Err(WorkspaceLoadError::NoTargetFormulaSpace);
            };
            let Some(target) = state.formula_spaces.get_mut(&target_id) else {
                return Err(WorkspaceLoadError::NoTargetFormulaSpace);
            };
            apply_loaded_scenario_to_formula_space(target, loaded.scenario);
            // The reload lands the user back where they were — make
            // the target the active formula in case the workspace's
            // active id had drifted relative to open order.
            state.workspace_shell.active_formula_space_id = Some(target_id);
        }
        Ok(())
    }
}

/// Errors the workspace loader can surface. `UnsupportedVersion`
/// fires when the stored JSON's `version` doesn't match what this
/// build understands; `FormulaFile` propagates a parse failure
/// from the embedded `.dnafml` XML; `Json` wraps a `serde_json`
/// failure on the envelope itself.
#[derive(Debug)]
pub enum WorkspaceLoadError {
    UnsupportedVersion(u32),
    FormulaFile(FormulaFileError),
    Json(serde_json::Error),
    NoTargetFormulaSpace,
    #[cfg(target_arch = "wasm32")]
    StorageUnavailable,
}

impl core::fmt::Display for WorkspaceLoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedVersion(v) => write!(f, "unsupported workspace version {v}"),
            Self::FormulaFile(e) => write!(f, "formula-file parse failure: {e}"),
            Self::Json(e) => write!(f, "workspace.json parse failure: {e}"),
            Self::NoTargetFormulaSpace => {
                write!(f, "workspace has no formula space to apply the snapshot to")
            }
            #[cfg(target_arch = "wasm32")]
            Self::StorageUnavailable => write!(f, "browser localStorage unavailable"),
        }
    }
}

impl std::error::Error for WorkspaceLoadError {}

/// Serialize the host state into the workspace.json wire form.
pub fn serialize_workspace(state: &OneCalcHostState) -> Result<String, serde_json::Error> {
    let envelope = WorkspaceJson::from_state(state);
    serde_json::to_string(&envelope)
}

/// Parse a workspace.json string into the envelope.
pub fn deserialize_workspace(json: &str) -> Result<WorkspaceJson, serde_json::Error> {
    serde_json::from_str(json)
}

// ---------------------------------------------------------------
// Browser localStorage adapter (wasm32-only)
// ---------------------------------------------------------------

/// Read the workspace envelope from `localStorage`. Returns
/// `Ok(None)` when no entry has been written yet (fresh user) and
/// `Err(...)` for storage failures or schema-incompatibility.
#[cfg(target_arch = "wasm32")]
pub fn load_workspace_from_local_storage() -> Result<Option<WorkspaceJson>, WorkspaceLoadError> {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .ok_or(WorkspaceLoadError::StorageUnavailable)?;
    let raw = match storage.get_item(WORKSPACE_STORAGE_KEY) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(None),
        Err(_) => return Err(WorkspaceLoadError::StorageUnavailable),
    };
    let envelope = deserialize_workspace(&raw).map_err(WorkspaceLoadError::Json)?;
    Ok(Some(envelope))
}

/// Write the workspace envelope into `localStorage`. Silent on
/// success; logs to the console on failure (storage quota, the
/// user disabling site data, etc.) so persistence failures don't
/// take the rest of the app down with them.
#[cfg(target_arch = "wasm32")]
pub fn save_workspace_to_local_storage(state: &OneCalcHostState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    let json = match serialize_workspace(state) {
        Ok(json) => json,
        Err(error) => {
            web_sys::console::error_1(
                &format!("[onecalc] workspace.json serialise failed: {error}").into(),
            );
            return;
        }
    };
    if let Err(error) = storage.set_item(WORKSPACE_STORAGE_KEY, &json) {
        web_sys::console::error_1(
            &format!("[onecalc] workspace.json write failed: {error:?}").into(),
        );
    }
}

/// Apply the most recently saved workspace.json (if any) to the
/// host state. Called from the home-shell mount before the
/// component subscribes to the state signal so the user sees their
/// last-edited formula immediately.
#[cfg(target_arch = "wasm32")]
pub fn hydrate_state_from_local_storage(state: &mut OneCalcHostState) {
    match load_workspace_from_local_storage() {
        Ok(Some(envelope)) => {
            if let Err(error) = envelope.apply_to_state(state) {
                web_sys::console::warn_1(
                    &format!("[onecalc] workspace.json apply failed: {error}").into(),
                );
            }
        }
        Ok(None) => {
            // Fresh user — no prior workspace to restore. Not an
            // error.
        }
        Err(error) => {
            web_sys::console::warn_1(
                &format!("[onecalc] workspace.json load failed: {error}").into(),
            );
        }
    }
}

/// Marker so non-wasm callers can branch without `cfg`.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_workspace_to_local_storage(_state: &OneCalcHostState) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn hydrate_state_from_local_storage(_state: &mut OneCalcHostState) {}

// Take an unused import to silence the warning when the wasm path
// isn't compiled.
#[cfg(not(target_arch = "wasm32"))]
fn _silence_unused(_: &FormulaSpaceState) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::case_lifecycle::{
        new_formula_space, pin_active_formula_space, toggle_pin_formula_space,
    };

    #[test]
    fn workspace_round_trips_pins_and_active_formula_text() {
        let mut original = OneCalcHostState::default();
        let id = new_formula_space(&mut original);
        // Mutate the formula a bit so we can assert restoration.
        let formula_space = original.formula_spaces.get_mut(&id).expect("active space");
        formula_space.raw_entered_cell_text = "=SUM(1,2,3)".to_string();
        formula_space.committed_cell_text = Some("=SUM(1,2,3)".to_string());
        formula_space.formatting.number_format_code = "0.00".to_string();
        // Pin the active formula.
        let _ = pin_active_formula_space(&mut original);

        // Round-trip through the JSON envelope.
        let json = serialize_workspace(&original).expect("serialise");
        let restored_envelope = deserialize_workspace(&json).expect("parse");

        // Boot a fresh host and apply the envelope on top.
        let mut restored = OneCalcHostState::default();
        let _ = new_formula_space(&mut restored);
        restored_envelope
            .apply_to_state(&mut restored)
            .expect("apply");

        // Pins survive verbatim.
        let pinned_ids: Vec<_> = restored
            .workspace_shell
            .pinned_formula_space_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(
            pinned_ids,
            vec![id.as_str().to_string()],
            "pinned ids must round-trip verbatim",
        );

        // Active formula text + formatting round-trips.
        let active = restored
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|aid| restored.formula_spaces.get(aid))
            .expect("active restored");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(active.formatting.number_format_code, "0.00");
    }

    #[test]
    fn workspace_envelope_rejects_future_versions() {
        let mut envelope = WorkspaceJson {
            version: 999,
            pinned_formula_space_ids: Vec::new(),
            active_formula_xml: None,
        };
        let mut state = OneCalcHostState::default();
        let _ = new_formula_space(&mut state);
        let result = std::mem::take(&mut envelope).apply_to_state(&mut state);
        assert!(matches!(
            result,
            Err(WorkspaceLoadError::UnsupportedVersion(999)),
        ));
    }

    impl Default for WorkspaceJson {
        fn default() -> Self {
            Self {
                version: 1,
                pinned_formula_space_ids: Vec::new(),
                active_formula_xml: None,
            }
        }
    }

    #[test]
    fn pin_toggle_persists_through_envelope() {
        let mut state = OneCalcHostState::default();
        let id = new_formula_space(&mut state);
        // Pin via toggle, then unpin via toggle.
        assert!(toggle_pin_formula_space(&mut state, id.as_str()));
        let json_pinned = serialize_workspace(&state).expect("serialise pinned");
        assert!(toggle_pin_formula_space(&mut state, id.as_str()));
        let json_unpinned = serialize_workspace(&state).expect("serialise unpinned");
        assert_ne!(
            json_pinned, json_unpinned,
            "envelope must reflect pin-toggle state",
        );
    }
}
