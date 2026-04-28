//! WS-14 wasm-bindgen browser corpus harness.
//!
//! Single integration-test binary that mounts the home shell into a real
//! headless browser (Microsoft Edge via wasm-bindgen-test). The
//! `scaffold` module owns the mount / teardown / DOM-query helpers; the
//! per-surface modules (`editor_core`, ...) own the invariants.
//!
//! Runs only on `wasm32-unknown-unknown`; the file is empty on every other
//! target so `cargo test --workspace` on the host stays clean.

#![cfg(target_arch = "wasm32")]

#[path = "browser/scaffold.rs"]
mod scaffold;

#[path = "browser/editor_core.rs"]
mod editor_core;
