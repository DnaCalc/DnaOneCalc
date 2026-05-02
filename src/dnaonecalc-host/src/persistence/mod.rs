//! Per-formula persistence.
//!
//! Slice 1 (this module) implements the in-memory `Scenario` shape +
//! XML emitter + XML parser for `.dnafml` / `.xml` files. Wiring the
//! breadcrumb's `Save as…` / `Open…` actions to the file system
//! belongs to slice 1b; the `<dna:CompareBundle>` merge is slice 4.
//!
//! See `docs/PERSISTENCE_FORMAT_PLAN.md` §10 for the full seam ladder.

pub mod formula_file;

pub use formula_file::{
    read_formula_xml, write_formula_xml, CfRule, Context, Entry, EntryMode, FormulaFileError,
    HostProfile, Identity, Locale, PublicationContext, Scenario, ScenarioPolicy, UiPreferences,
};
