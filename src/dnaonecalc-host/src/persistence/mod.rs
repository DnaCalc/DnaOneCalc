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

#[cfg(target_arch = "wasm32")]
pub mod browser_file_io;

pub use formula_file::{
    read_formula_xml, write_formula_xml, CfRule, Context, Entry, EntryMode, FormulaFileError,
    HostProfile, Identity, Locale, PublicationContext, Scenario, ScenarioPolicy, UiPreferences,
};
pub use scenario_projection::{apply_loaded_scenario_to_formula_space, formula_space_to_scenario};

#[cfg(target_arch = "wasm32")]
pub use browser_file_io::{
    open_xml_via_file_input, save_xml_via_download, suggested_filename_stem, OpenedFormulaFile,
};
