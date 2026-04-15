//! S1 / S9 / S10 / S11 — workspace / formula-space lifecycle scenarios.

use dnaonecalc_host::app::case_lifecycle::{
    close_formula_space, new_formula_space, toggle_pin_formula_space,
};
use dnaonecalc_host::services::shell_composition::{select_active_formula_space, ShellRailSection};
use dnaonecalc_host::state::OneCalcHostState;

use super::fixtures::{
    explore_editor_cluster, explore_projection, explore_result_cluster,
    fresh_state_with_active_space, scenario_bridge, shell_frame, type_formula,
};

#[test]
fn cold_boot_shows_an_untitled_space_with_no_result() {
    // S1: default state + `new_formula_space` → shell frame has one
    // untitled space, explore cluster has empty text, result cluster has
    // no display summary.
    let (state, _space) = fresh_state_with_active_space();

    let frame = shell_frame(&state);
    assert_eq!(frame.formula_spaces.len(), 1);
    let space_item = &frame.formula_spaces[0];
    assert_eq!(space_item.label, "Untitled 1");
    assert!(space_item.is_active);
    assert_eq!(space_item.section, ShellRailSection::Open);

    let explore = explore_projection(&state);
    let editor = explore_editor_cluster(&explore);
    assert_eq!(editor.raw_entered_cell_text, "");

    let result = explore_result_cluster(&explore);
    assert!(result.effective_display_summary.is_none());
    assert!(result.latest_evaluation_summary.is_none());
}

#[test]
fn two_spaces_retain_isolated_text_across_space_switches() {
    // S9: create two formula spaces, type different text into each, confirm
    // each space's cluster reflects only its own text when active.
    let mut state = OneCalcHostState::default();
    let bridge = scenario_bridge();
    let first = new_formula_space(&mut state);
    type_formula(&bridge, &mut state, "=SUM(1,2)");
    let second = new_formula_space(&mut state);
    type_formula(&bridge, &mut state, "=SUM(10,20)");

    // Second space is currently active — expect its text.
    let explore_second = explore_projection(&state);
    assert_eq!(
        explore_editor_cluster(&explore_second).raw_entered_cell_text,
        "=SUM(10,20)",
    );
    assert_eq!(
        explore_result_cluster(&explore_second)
            .effective_display_summary
            .as_deref(),
        Some("30"),
    );

    // Switch back to the first space.
    select_active_formula_space(&mut state, first.as_str());
    let explore_first = explore_projection(&state);
    assert_eq!(
        explore_editor_cluster(&explore_first).raw_entered_cell_text,
        "=SUM(1,2)",
    );
    assert_eq!(
        explore_result_cluster(&explore_first)
            .effective_display_summary
            .as_deref(),
        Some("3"),
    );

    // Shell frame should still list both spaces.
    let frame = shell_frame(&state);
    assert_eq!(frame.formula_spaces.len(), 2);
    let _ = second;
}

#[test]
fn closing_the_last_space_keeps_the_workspace_non_empty() {
    // S10: seed one space, close it, confirm a new untitled space spins up
    // automatically so the workspace is never blank.
    let (mut state, space) = fresh_state_with_active_space();

    let closed = close_formula_space(&mut state, space.as_str());

    assert!(closed);
    let frame = shell_frame(&state);
    assert_eq!(
        frame.formula_spaces.len(),
        1,
        "workspace spins up a fresh untitled space after last-close",
    );
    let explore = explore_projection(&state);
    let editor = explore_editor_cluster(&explore);
    assert_eq!(editor.raw_entered_cell_text, "");
}

#[test]
fn pinning_a_space_moves_it_to_the_pinned_rail_section() {
    // S11: pin a space and confirm the shell frame reports it as pinned.
    let mut state = OneCalcHostState::default();
    let first = new_formula_space(&mut state);
    let _second = new_formula_space(&mut state);

    let pinned = toggle_pin_formula_space(&mut state, first.as_str());
    assert!(pinned);

    let frame = shell_frame(&state);
    let first_item = frame
        .formula_spaces
        .iter()
        .find(|item| item.formula_space_id == first.as_str())
        .expect("first space is in shell frame");
    assert_eq!(first_item.section, ShellRailSection::Pinned);
    assert!(first_item.is_pinned);
}
