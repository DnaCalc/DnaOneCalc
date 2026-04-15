//! S8 — Explore → Inspect mode switching scenario.

use dnaonecalc_host::services::shell_composition::switch_active_mode;
use dnaonecalc_host::state::AppMode;

use super::fixtures::{
    fresh_state_with_active_space, inspect_projection, scenario_bridge, type_formula,
};

#[test]
fn switching_from_explore_to_inspect_preserves_entered_text() {
    // S8: type `=SUM(1,2)` in Explore, switch to Inspect, confirm the
    // Inspect view model still reflects the entered text and the formula
    // walk from the real editor document is present.
    let (mut state, _space) = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SUM(1,2)");
    switch_active_mode(&mut state, AppMode::Inspect);

    let inspect = inspect_projection(&state);
    assert_eq!(inspect.raw_entered_cell_text, "=SUM(1,2)");
    assert!(
        !inspect.formula_walk_nodes.is_empty(),
        "inspect mode surfaces the formula walk from the live OxFml bridge",
    );
}
