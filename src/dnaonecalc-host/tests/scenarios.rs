//! End-to-end scenario tests for the OneCalc view-model layer.
//!
//! See `~/.claude/plans/fuzzy-spinning-flame.md` → "Scenario test catalogue".
//!
//! Each scenario walks a user action from `OneCalcHostState::default()`
//! through the reducer / live-edit entry points (using the real
//! `PreviewOxfmlBridge` where possible), builds the full projection via
//! `build_active_mode_projection` + cluster builders, and asserts on
//! user-visible cluster fields only. Cross-layer composition is the point;
//! the tests are named for the user action, not the functions called.

#[path = "scenarios/fixtures.rs"]
mod fixtures;
#[path = "scenarios/typing.rs"]
mod typing;
#[path = "scenarios/workspace.rs"]
mod workspace;
#[path = "scenarios/mode_switching.rs"]
mod mode_switching;
#[path = "scenarios/artifacts.rs"]
mod artifacts;
