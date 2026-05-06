//! Per-formula persistence.
//!
//! Slice 1 ships the in-memory `Scenario` shape + XML emitter + XML
//! parser for `.dnafml` / `.xml` files (`formula_file`). Slice 1b
//! adds the host-side projection (`scenario_projection`) and
//! browser-host file IO (`browser_file_io` — wasm32-only). Tauri
//! file IO and the `<dna:CompareBundle>` merge are later slices.
//!
//! See `docs/PERSISTENCE_FORMAT_PLAN.md` §10 for the full seam ladder.

pub mod formula_file;
pub mod scenario_projection;
pub mod workspace_storage;

#[cfg(target_arch = "wasm32")]
pub mod browser_file_io;

pub use formula_file::{
    apply_bundle_retention_policy, read_formula_xml, write_formula_xml, BundleVerdict, CfRule,
    CompareBundle, Context, Entry, EntryMode, FormulaFileError, HostProfile, Identity,
    LoadDiagnostic, LoadedFormula, Locale, PublicationContext, Scenario, ScenarioPolicy,
    UiPreferences, DEFAULT_BUNDLE_RETENTION_CAP,
};
pub use scenario_projection::{
    apply_loaded_scenario_to_formula_space, apply_loaded_scenario_with_diagnostics,
    formula_space_to_scenario,
};
pub use workspace_storage::{
    deserialize_workspace, hydrate_state_from_local_storage, save_workspace_to_local_storage,
    serialize_workspace, WorkspaceJson, WorkspaceLoadError, WORKSPACE_STORAGE_KEY,
};

#[cfg(target_arch = "wasm32")]
pub use browser_file_io::{
    open_xml_via_file_input, save_xml_via_download, suggested_filename_stem, OpenedFormulaFile,
};
