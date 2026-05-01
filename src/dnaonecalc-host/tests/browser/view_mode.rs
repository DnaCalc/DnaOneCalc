//! View-mode toggle invariants.
//!
//! The home shell carries a workspace-level reading-audience
//! preference. Default is User mode (Excel-user-friendly chrome
//! with phase chips, state slugs, and SEAM markers hidden);
//! Ctrl+Alt+D toggles to Developer mode (full engineering
//! surface).
//!
//! This corpus pins the toggle plumbing only. The mode-conditional
//! rendering of the foot chips and the walk-tree is pinned by
//! the corpora attached to the next two beads
//! (foot_chip_modes.rs and walk_tree_modes.rs).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_input, dispatch_keydown_with_modifiers, mount_home_shell,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn default_view_mode_is_user_on_first_mount() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let root = shell
        .select(".onecalc-home-shell")
        .expect("home-shell root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "fresh mount must default to User view-mode",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn ctrl_alt_d_toggles_data_view_mode_attribute() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    // Ctrl+Alt+D — User -> Developer.
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let root = shell
        .select(".onecalc-home-shell")
        .expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
        "Ctrl+Alt+D must flip data-view-mode to developer",
    );

    // Ctrl+Alt+D again — Developer -> User.
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    let root = shell
        .select(".onecalc-home-shell")
        .expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "second Ctrl+Alt+D must flip data-view-mode back to user",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn status_foot_dev_tag_visible_only_in_developer_mode() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    assert!(
        shell
            .select(".onecalc-home-shell__statusfoot-mode-tag")
            .is_none(),
        "User mode (default) must not render the dev tag",
    );

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let tag = shell
        .select(".onecalc-home-shell__statusfoot-mode-tag")
        .expect("dev tag mounted in Developer mode");
    assert_eq!(
        tag.get_attribute("data-view-mode").as_deref(),
        Some("developer"),
    );
    assert_eq!(tag.text_content().unwrap_or_default().trim(), "dev");

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;
    assert!(
        shell
            .select(".onecalc-home-shell__statusfoot-mode-tag")
            .is_none(),
        "toggling back to User mode must remove the dev tag",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn ctrl_d_alone_still_toggles_drill_not_view_mode() {
    // Pin that Ctrl+D (no Alt) keeps its existing meaning
    // (formula drill toggle) and does NOT flip the view-mode.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    // Drill panel is now expanded.
    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(
        panel.get_attribute("data-expanded").as_deref(),
        Some("true"),
    );

    // View-mode is still User (Ctrl+D alone did not flip it).
    let root = shell
        .select(".onecalc-home-shell")
        .expect("root mounted");
    assert_eq!(
        root.get_attribute("data-view-mode").as_deref(),
        Some("user"),
        "Ctrl+D (no Alt) must NOT flip the view-mode",
    );

    shell.tear_down();
}
