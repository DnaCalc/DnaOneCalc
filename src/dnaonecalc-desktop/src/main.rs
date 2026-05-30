// DNA OneCalc Desktop Shell
//
// Thin Tauri v2 wrapper over the shared Leptos/WASM application core.
// This host owns startup, window chrome, and platform-specific wiring only.
// Product behavior lives in the shared `dnaonecalc-host` crate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run DNA OneCalc desktop application");
}
