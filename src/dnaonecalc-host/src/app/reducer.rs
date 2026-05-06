use crate::services::completion_popup::{
    accept_selected_completion as popup_accept_selected, dismiss_completion_popup as popup_dismiss,
    move_completion_selection as popup_move_selection, CompletionAcceptance,
};
use crate::services::retained_artifacts::{
    import_manual_artifact_for_active_formula_space, import_verification_bundle_report_json,
    open_retained_artifact_by_id, open_retained_artifact_in_inspect_by_id,
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use crate::state::{CompletionHelpState, FormulaSpaceState, OneCalcHostState};
use crate::ui::editor::commands::{
    apply_editor_command, cycle_completion_selection, EditorCommand, EditorInputEvent,
};
use crate::ui::editor::geometry::{EditorOverlayMeasurementEvent, TextareaMeasurementMetrics};
use crate::ui::editor::state::EditorSurfaceState;

pub fn apply_editor_input_to_active_formula_space(
    state: &mut OneCalcHostState,
    input_event: EditorInputEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    let next_editor_state = if let (Some(selection_start), Some(selection_end)) =
        (input_event.selection_start, input_event.selection_end)
    {
        EditorSurfaceState::for_text_with_selection(
            &input_event.text,
            selection_start,
            selection_end,
        )
    } else {
        EditorSurfaceState::for_text(&input_event.text)
    };
    apply_local_editor_text_change(formula_space, input_event.text, next_editor_state);
    true
}

pub fn apply_editor_command_to_active_formula_space(
    state: &mut OneCalcHostState,
    command: EditorCommand,
) -> bool {
    match &command {
        EditorCommand::ToggleEditorSettingsPopover => {
            return toggle_editor_settings_popover(state);
        }
        EditorCommand::UpdateEditorSetting(update) => {
            return update_editor_setting(state, *update);
        }
        EditorCommand::ToggleConfigureDrawer => {
            return toggle_configure_drawer(state);
        }
        _ => {}
    }

    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    if apply_completion_command(formula_space, &command) {
        return true;
    }

    if apply_live_state_command(formula_space, &command) {
        return true;
    }

    let result = apply_editor_command(
        &formula_space.raw_entered_cell_text,
        &formula_space.editor_surface_state,
        command,
    );
    apply_local_editor_text_change(formula_space, result.text, result.state);
    true
}

fn apply_live_state_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
    // UI chrome commands are handled at the host level; delegate them back up.
    if matches!(
        command,
        EditorCommand::ToggleEditorSettingsPopover
            | EditorCommand::UpdateEditorSetting(_)
            | EditorCommand::ToggleConfigureDrawer
    ) {
        return false;
    }
    match command {
        EditorCommand::CommitEntry => {
            formula_space.committed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::RequestProof => {
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::CancelEntry => {
            if let Some(committed) = formula_space.committed_cell_text.clone() {
                let restored_state = EditorSurfaceState::for_text(&committed);
                apply_local_editor_text_change(formula_space, committed.clone(), restored_state);
                formula_space.committed_cell_text = Some(committed.clone());
                formula_space.proofed_cell_text = Some(committed);
            }
            true
        }
        EditorCommand::ToggleExpandedHeight => {
            formula_space.expanded_editor = !formula_space.expanded_editor;
            true
        }
        EditorCommand::DismissCompletion => {
            formula_space.editor_surface_state.completion_anchor_offset = None;
            formula_space.editor_surface_state.completion_selected_index = None;
            true
        }
        EditorCommand::CycleReferenceForm => {
            let selection = &formula_space.editor_surface_state.selection;
            if let Some(result) = crate::ui::editor::reference_cycle::cycle_reference_form(
                &formula_space.raw_entered_cell_text,
                selection.start(),
                selection.end(),
            ) {
                let next_state = crate::ui::editor::state::EditorSurfaceState {
                    caret: crate::ui::editor::state::EditorCaret {
                        offset: result.reference_end,
                    },
                    selection: crate::ui::editor::state::EditorSelection {
                        anchor: result.reference_start,
                        focus: result.reference_end,
                    },
                    scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                    completion_anchor_offset: None,
                    completion_selected_index: None,
                    signature_help_anchor_offset: None,
                };
                apply_local_editor_text_change(formula_space, result.text, next_state);
            }
            true
        }
        EditorCommand::ForceShowCompletion | EditorCommand::SendSelectionToInspect => {
            // Model-level no-ops that the view layer or downstream services consume.
            true
        }
        _ => false,
    }
}

pub fn update_editor_setting(
    state: &mut OneCalcHostState,
    update: crate::ui::editor::state::EditorSettingUpdate,
) -> bool {
    state.global_ui_chrome.editor_settings.apply(update);
    true
}

pub fn toggle_editor_settings_popover(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.editor_settings_popover_open =
        !state.global_ui_chrome.editor_settings_popover_open;
    true
}

pub fn toggle_configure_drawer(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.configure_drawer_open = !state.global_ui_chrome.configure_drawer_open;
    true
}

/// Toggle the titlebar scenario-breadcrumb dropdown. Returns `true`
/// to signal the reducer caller that state changed.
pub fn toggle_scenario_breadcrumb(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.scenario_breadcrumb_open =
        !state.global_ui_chrome.scenario_breadcrumb_open;
    true
}

/// Force the scenario-breadcrumb dropdown closed (outside-click,
/// Esc). No-op when already closed; returns `false` in that case so
/// the host avoids a redundant re-render.
pub fn close_scenario_breadcrumb(state: &mut OneCalcHostState) -> bool {
    if !state.global_ui_chrome.scenario_breadcrumb_open {
        return false;
    }
    state.global_ui_chrome.scenario_breadcrumb_open = false;
    true
}

// -----------------------------------------------------------------
// Array-browser reducers (zoom, selection, resize, display options)
// -----------------------------------------------------------------

const ARRAY_BROWSER_ZOOM_MIN: f32 = 0.5;
const ARRAY_BROWSER_ZOOM_MAX: f32 = 3.0;
const ARRAY_BROWSER_ZOOM_STEP: f32 = 0.1;
const ARRAY_BROWSER_COL_MIN_REM: f32 = 1.5;
const ARRAY_BROWSER_COL_MAX_REM: f32 = 32.0;
const ARRAY_BROWSER_ROW_MIN_REM: f32 = 1.0;
const ARRAY_BROWSER_ROW_MAX_REM: f32 = 12.0;

/// Set the active formula's array-browser zoom factor. Clamped
/// to `[0.5, 3.0]`. Returns `true` when the value actually
/// changed.
pub fn set_active_array_browser_zoom(state: &mut OneCalcHostState, zoom: f32) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = zoom.clamp(ARRAY_BROWSER_ZOOM_MIN, ARRAY_BROWSER_ZOOM_MAX);
    if (formula_space.array_browser.zoom - next).abs() < f32::EPSILON {
        return false;
    }
    formula_space.array_browser.zoom = next;
    true
}

/// Step the zoom up by one notch (0.1).
pub fn zoom_in_active_array_browser(state: &mut OneCalcHostState) -> bool {
    let current = active_formula_space_mut(state)
        .map(|space| space.array_browser.zoom)
        .unwrap_or(1.0);
    set_active_array_browser_zoom(state, current + ARRAY_BROWSER_ZOOM_STEP)
}

/// Step the zoom down by one notch (0.1).
pub fn zoom_out_active_array_browser(state: &mut OneCalcHostState) -> bool {
    let current = active_formula_space_mut(state)
        .map(|space| space.array_browser.zoom)
        .unwrap_or(1.0);
    set_active_array_browser_zoom(state, current - ARRAY_BROWSER_ZOOM_STEP)
}

/// Reset zoom to 1.0.
pub fn reset_active_array_browser_zoom(state: &mut OneCalcHostState) -> bool {
    set_active_array_browser_zoom(state, 1.0)
}

/// Set the active formula's array-browser block selection. Pass
/// `None` to clear. Idempotent.
pub fn set_active_array_browser_selection(
    state: &mut OneCalcHostState,
    selection: Option<crate::state::ArrayBlockSelection>,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.array_browser.selection == selection {
        return false;
    }
    formula_space.array_browser.selection = selection;
    true
}

/// Override the displayed width (in rem) for a single column in
/// the active formula's array browser. Clamped so the column
/// stays useful.
pub fn set_active_array_browser_column_width(
    state: &mut OneCalcHostState,
    column: usize,
    width_rem: f32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = width_rem.clamp(ARRAY_BROWSER_COL_MIN_REM, ARRAY_BROWSER_COL_MAX_REM);
    let prior = formula_space
        .array_browser
        .column_widths_rem
        .insert(column, next);
    prior.is_none() || (prior.unwrap() - next).abs() >= f32::EPSILON
}

/// Override the displayed height (in rem) for a single row in
/// the active formula's array browser. Clamped to stay useful.
pub fn set_active_array_browser_row_height(
    state: &mut OneCalcHostState,
    row: usize,
    height_rem: f32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = height_rem.clamp(ARRAY_BROWSER_ROW_MIN_REM, ARRAY_BROWSER_ROW_MAX_REM);
    let prior = formula_space
        .array_browser
        .row_heights_rem
        .insert(row, next);
    prior.is_none() || (prior.unwrap() - next).abs() >= f32::EPSILON
}

/// Toggle a workspace-level array-browser display option. The
/// `which` enum picks which knob to toggle.
pub fn toggle_array_browser_display_option(
    state: &mut OneCalcHostState,
    which: ArrayBrowserDisplayToggle,
) -> bool {
    let target = match which {
        ArrayBrowserDisplayToggle::GridLines => {
            &mut state.global_ui_chrome.array_browser_display.show_grid_lines
        }
        ArrayBrowserDisplayToggle::AlternatingRows => {
            &mut state
                .global_ui_chrome
                .array_browser_display
                .show_alternating_rows
        }
        ArrayBrowserDisplayToggle::RowColumnHeaders => {
            &mut state
                .global_ui_chrome
                .array_browser_display
                .show_row_column_headers
        }
    };
    *target = !*target;
    true
}

#[derive(Debug, Clone, Copy)]
pub enum ArrayBrowserDisplayToggle {
    GridLines,
    AlternatingRows,
    RowColumnHeaders,
}

/// Toggle the command-palette overlay. Resets the filter query
/// and selected index to the empty / 0 state on every toggle so
/// the next open starts fresh.
pub fn toggle_command_palette(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.command_palette_open = !state.global_ui_chrome.command_palette_open;
    state.global_ui_chrome.command_palette_query.clear();
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Force the command palette closed (outside-click, Esc). No-op
/// when already closed.
pub fn close_command_palette(state: &mut OneCalcHostState) -> bool {
    if !state.global_ui_chrome.command_palette_open {
        return false;
    }
    state.global_ui_chrome.command_palette_open = false;
    state.global_ui_chrome.command_palette_query.clear();
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Update the command palette's filter input. Resets the selected
/// index to 0 because the prior selection's position in the
/// filtered list is no longer meaningful after the query changes.
pub fn set_command_palette_query(state: &mut OneCalcHostState, query: String) -> bool {
    if state.global_ui_chrome.command_palette_query == query {
        return false;
    }
    state.global_ui_chrome.command_palette_query = query;
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Move the command palette's selected index by `delta` (signed).
/// Wraps around `total` so Up at index 0 lands on the last entry
/// and Down at the last entry lands on 0. No-op when `total == 0`.
pub fn move_command_palette_selection(
    state: &mut OneCalcHostState,
    delta: isize,
    total: usize,
) -> bool {
    if total == 0 {
        return false;
    }
    let current = state.global_ui_chrome.command_palette_selected_index as isize;
    let next = current + delta;
    let total_signed = total as isize;
    let wrapped = ((next % total_signed) + total_signed) % total_signed;
    state.global_ui_chrome.command_palette_selected_index = wrapped as usize;
    true
}

/// Slice 5 — formatting-control mutations on the active formula's
/// FormulaFormattingState. Each setter returns `true` when the value
/// actually changed (so callers can short-circuit re-renders) and
/// `false` when the value is unchanged.
pub fn set_active_number_format_code(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.number_format_code == value {
        return false;
    }
    formula_space.formatting.number_format_code = value;
    true
}

pub fn set_active_font_color(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.font_color == value {
        return false;
    }
    formula_space.formatting.font_color = value;
    true
}

pub fn set_active_fill_color(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.fill_color == value {
        return false;
    }
    formula_space.formatting.fill_color = value;
    true
}

pub fn set_active_date1904(state: &mut OneCalcHostState, value: bool) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.date1904 == value {
        return false;
    }
    formula_space.formatting.date1904 = value;
    true
}

/// Set the active formula's scenario policy (Deterministic /
/// LiveRecalc). Drives whether the bridge passes fixed seeds for
/// `now_serial` / `random_value` (Deterministic — formula re-runs
/// identically on every keystroke) or fresh values per round-trip
/// (LiveRecalc — `=NOW()` advances, `=RAND()` rolls). Returns true
/// when the value actually changed.
pub fn set_active_scenario_policy(
    state: &mut OneCalcHostState,
    policy: crate::persistence::ScenarioPolicy,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.scenario_policy == policy {
        return false;
    }
    formula_space.formatting.scenario_policy = policy;
    true
}

/// Switch the workspace's ambient locale preset to the given
/// BCP-47 language tag. Replaces the date / datetime / time format
/// codes and the `language_tag` field on `AmbientAppContext`.
/// Returns true when the value actually changed.
///
/// Note: surface-only today — the result hero's format defaults
/// follow this preset, but month / weekday names and numeric
/// separators stay en-US until OxFml's locale tables land
/// (`SEAM-OXFML-LOCALE-EXPAND`).
pub fn set_workspace_locale_preset(state: &mut OneCalcHostState, language_tag: String) -> bool {
    let next = crate::services::ambient_app_context::ambient_app_context_for_language_tag(Some(
        &language_tag,
    ));
    if state.ambient_app_context == next {
        return false;
    }
    state.ambient_app_context = next;
    true
}

/// Append a conditional-formatting rule to the active formula.
/// Returns the new rule's index in the list, or `None` when there
/// is no active formula space.
pub fn add_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    rule: crate::state::FormulaConditionalFormattingRule,
) -> Option<usize> {
    let formula_space = active_formula_space_mut(state)?;
    formula_space
        .formatting
        .conditional_formatting_rules
        .push(rule);
    Some(formula_space.formatting.conditional_formatting_rules.len() - 1)
}

/// Remove a conditional-formatting rule by index. Returns true when
/// the rule existed and was removed.
pub fn remove_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    index: usize,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if index >= formula_space.formatting.conditional_formatting_rules.len() {
        return false;
    }
    formula_space
        .formatting
        .conditional_formatting_rules
        .remove(index);
    true
}

/// Update an existing CF rule by index. Returns true when the rule
/// existed and the new value is different from the current value.
pub fn update_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    index: usize,
    rule: crate::state::FormulaConditionalFormattingRule,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let rules = &mut formula_space.formatting.conditional_formatting_rules;
    if index >= rules.len() {
        return false;
    }
    if rules[index] == rule {
        return false;
    }
    rules[index] = rule;
    true
}

pub fn apply_editor_overlay_measurement_to_active_formula_space(
    state: &mut OneCalcHostState,
    measurement_event: EditorOverlayMeasurementEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    formula_space.editor_overlay_geometry = Some(measurement_event.snapshot);
    true
}

/// Set the popup's `selected_index` to the row matching `proposal_id`,
/// then accept it. Returns the acceptance (or `None` when the popup is
/// `Hidden` or the id is unknown). Used by the popup's click-to-accept
/// path so the home shell doesn't need to dispatch separate
/// "select-by-id then accept" reducer calls per click.
pub fn accept_completion_by_proposal_id_on_active_formula_space(
    state: &mut OneCalcHostState,
    proposal_id: &str,
) -> Option<crate::services::completion_popup::CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    if let crate::services::completion_popup::CompletionPopupState::Open {
        items,
        selected_index,
        ..
    } = &mut formula_space.completion_popup
    {
        if let Some(index) = items
            .iter()
            .position(|item| item.proposal_id == proposal_id)
        {
            *selected_index = index;
        } else {
            return None;
        }
    } else {
        return None;
    }
    let acceptance = popup_accept_selected(&mut formula_space.completion_popup, &raw_text)?;
    formula_space.completion_popup_suppressed_until_next_input = true;
    Some(acceptance)
}

/// Accept the popup's currently-selected completion (the keyboard
/// path: Tab / Enter from a popup-Open state). Mirrors the
/// proposal-id variant for the click path; both set the
/// `completion_popup_suppressed_until_next_input` flag so the next
/// bridge refresh does not re-open the popup over the just-accepted
/// proposal.
pub fn accept_selected_completion_with_suppression_on_active_formula_space(
    state: &mut OneCalcHostState,
) -> Option<crate::services::completion_popup::CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    let acceptance = popup_accept_selected(&mut formula_space.completion_popup, &raw_text)?;
    formula_space.completion_popup_suppressed_until_next_input = true;
    Some(acceptance)
}

/// Move the completion popup's selected index by `delta` (typically
/// `+1` for ArrowDown / `-1` for ArrowUp). Wraps at both ends. No-op
/// when the popup is `Hidden`. Returns `true` when state changed.
pub fn move_completion_popup_selection_on_active_formula_space(
    state: &mut OneCalcHostState,
    delta: i32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    popup_move_selection(&mut formula_space.completion_popup, delta)
}

/// Force the completion popup to `Hidden`. No-op when already
/// `Hidden`. Returns `true` when state changed.
pub fn dismiss_completion_popup_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    popup_dismiss(&mut formula_space.completion_popup)
}

/// Toggle the workspace's view-mode preference. Returns the new
/// value (User or Developer). Pure flip; if a caller needs an
/// explicit set, use [`set_view_mode_on_workspace`].
pub fn toggle_view_mode_on_workspace(state: &mut OneCalcHostState) -> crate::state::ViewMode {
    state.view_mode = state.view_mode.toggle();
    state.view_mode
}

/// Set the workspace's view-mode preference explicitly. Returns
/// `true` when the value changed. Used by the workspace settings
/// page (later bead) and by tests; the keyboard chord uses the
/// toggle entry above.
pub fn set_view_mode_on_workspace(
    state: &mut OneCalcHostState,
    mode: crate::state::ViewMode,
) -> bool {
    if state.view_mode == mode {
        return false;
    }
    state.view_mode = mode;
    true
}

/// Toggle the formula-drill-down panel on the active formula space.
/// Returns the new `formula_drill_open` value, or `false` when there
/// is no active formula space (caller can treat that as a no-op).
pub fn toggle_formula_drill_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    formula_space.formula_drill_open = !formula_space.formula_drill_open;
    formula_space.formula_drill_open
}

/// Force-close the formula-drill-down panel. Used by tests and by
/// any future "exit drill-down on Esc" wiring. Returns `true` when
/// state changed.
pub fn close_formula_drill_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if !formula_space.formula_drill_open {
        return false;
    }
    formula_space.formula_drill_open = false;
    true
}

/// Toggle the formatting-controls panel on the active formula space.
/// The panel sits between the formula drill-down and the result
/// section; when collapsed it renders as a single chip with the
/// number-format summary, when expanded it renders the full
/// formatting controls row. Returns the new `formatting_panel_open`
/// value, or `false` when there is no active formula space.
pub fn toggle_formatting_panel_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    formula_space.formatting_panel_open = !formula_space.formatting_panel_open;
    formula_space.formatting_panel_open
}

/// Accept the popup's currently-selected completion. Returns the
/// `CompletionAcceptance` describing the splice the editor layer
/// should apply, plus transitions the popup to `Hidden`. Returns
/// `None` when the popup is `Hidden` (no acceptance to apply) or
/// when there is no active formula space.
pub fn accept_selected_completion_on_active_formula_space(
    state: &mut OneCalcHostState,
) -> Option<CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    popup_accept_selected(&mut formula_space.completion_popup, &raw_text)
}

/// Set or update the browser-measured textarea metrics on the active
/// formula space. Returns `true` when the metrics actually changed so
/// the home shell can short-circuit no-op re-renders. Increments the
/// monotonic `editor_box_metrics_tick` counter on every change so
/// browser tests can detect re-measurement even when the values are
/// numerically identical.
pub fn apply_editor_box_metrics_to_active_formula_space(
    state: &mut OneCalcHostState,
    metrics: TextareaMeasurementMetrics,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.editor_box_metrics == Some(metrics) {
        return false;
    }
    formula_space.editor_box_metrics = Some(metrics);
    formula_space.editor_box_metrics_tick = formula_space.editor_box_metrics_tick.wrapping_add(1);
    true
}

pub fn open_retained_artifact_from_catalog(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_by_id(state, artifact_id).is_ok()
}

pub fn open_retained_artifact_from_catalog_in_inspect(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_in_inspect_by_id(state, artifact_id).is_ok()
}

pub fn import_manual_retained_artifact_into_active_formula_space(
    state: &mut OneCalcHostState,
    request: ManualRetainedArtifactImportRequest,
) -> bool {
    import_manual_artifact_for_active_formula_space(state, request).is_ok()
}

pub fn import_verification_bundle_report_into_workspace(
    state: &mut OneCalcHostState,
    request: VerificationBundleImportRequest,
) -> bool {
    import_verification_bundle_report_json(state, request).is_ok()
}

fn active_formula_space_mut(state: &mut OneCalcHostState) -> Option<&mut FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())?;
    state.formula_spaces.get_mut(&formula_space_id)
}

fn apply_local_editor_text_change(
    formula_space: &mut FormulaSpaceState,
    text: String,
    editor_surface_state: EditorSurfaceState,
) {
    formula_space.raw_entered_cell_text = text;
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.editor_overlay_geometry = None;
    formula_space.editor_document = None;
    formula_space.completion_help = CompletionHelpState::default();
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;
}

fn apply_completion_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
    match command {
        EditorCommand::SelectPreviousCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
                    formula_space.editor_surface_state.completion_selected_index,
                    proposal_count,
                    -1,
                );
            true
        }
        EditorCommand::SelectNextCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
                    formula_space.editor_surface_state.completion_selected_index,
                    proposal_count,
                    1,
                );
            true
        }
        EditorCommand::SelectCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            true
        }
        EditorCommand::AcceptSelectedCompletion => {
            let Some(document) = formula_space.editor_document.as_ref() else {
                return false;
            };
            let proposal_count = document.completion_proposals.len();
            if proposal_count == 0 {
                return false;
            }
            let selected_index = formula_space
                .editor_surface_state
                .completion_selected_index
                .unwrap_or(0)
                .min(proposal_count.saturating_sub(1));
            let proposal = &document.completion_proposals[selected_index];
            let (selection_start, selection_end) = proposal
                .replacement_span
                .map(|span| (span.start, span.start + span.len))
                .unwrap_or((
                    formula_space.editor_surface_state.selection.start(),
                    formula_space.editor_surface_state.selection.end(),
                ));
            let replacement_state = EditorSurfaceState {
                selection: crate::ui::editor::state::EditorSelection {
                    anchor: selection_start,
                    focus: selection_end,
                },
                caret: crate::ui::editor::state::EditorCaret {
                    offset: selection_end,
                },
                scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                completion_anchor_offset: formula_space
                    .editor_surface_state
                    .completion_anchor_offset,
                completion_selected_index: formula_space
                    .editor_surface_state
                    .completion_selected_index,
                signature_help_anchor_offset: formula_space
                    .editor_surface_state
                    .signature_help_anchor_offset,
            };
            let result = apply_editor_command(
                &formula_space.raw_entered_cell_text,
                &replacement_state,
                EditorCommand::InsertText(proposal.insert_text.clone()),
            );
            apply_local_editor_text_change(formula_space, result.text, result.state);
            true
        }
        EditorCommand::AcceptCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            apply_completion_command(formula_space, &EditorCommand::AcceptSelectedCompletion)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FormulaSpaceState;
    use crate::ui::editor::commands::EditorInputKind;
    use crate::ui::editor::geometry::{
        EditorMeasuredOverlayBox, EditorOverlayGeometrySnapshot, EditorOverlayMeasurementEvent,
    };
    use crate::ui::editor::state::EditorSelection;
    use crate::{
        domain::ids::FormulaSpaceId,
        services::{
            programmatic_testing::{
                ProgrammaticArtifactCatalogEntry, ProgrammaticComparisonStatus,
                ProgrammaticOpenModeHint,
            },
            retained_artifacts::RetainedArtifactImportRequest,
        },
    };

    fn fresh_state_with_active_space(id: &str) -> OneCalcHostState {
        let formula_space_id = FormulaSpaceId::new(id);
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, ""));
        state
    }

    #[test]
    fn array_browser_zoom_reducers_clamp_to_bounds() {
        let mut state = fresh_state_with_active_space("space-zoom");
        // Default zoom is 1.0.
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!((zoom - 1.0).abs() < f32::EPSILON);

        // Step up several times — clamped at 3.0.
        for _ in 0..50 {
            let _ = zoom_in_active_array_browser(&mut state);
        }
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!(zoom <= 3.0 + f32::EPSILON);
        assert!(zoom >= 3.0 - f32::EPSILON);

        // Reset returns to 1.0.
        let _ = reset_active_array_browser_zoom(&mut state);
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!((zoom - 1.0).abs() < f32::EPSILON);

        // Step down several times — clamped at 0.5.
        for _ in 0..50 {
            let _ = zoom_out_active_array_browser(&mut state);
        }
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!(zoom <= 0.5 + f32::EPSILON);
        assert!(zoom >= 0.5 - f32::EPSILON);
    }

    #[test]
    fn array_browser_selection_reducer_round_trips() {
        use crate::state::ArrayBlockSelection;
        let mut state = fresh_state_with_active_space("space-sel");
        let sel = ArrayBlockSelection::from_corners(0, 0, 2, 3);
        assert!(set_active_array_browser_selection(&mut state, Some(sel)));
        let stored = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection)
            .expect("stored selection");
        assert_eq!(stored.normalized(), (0, 2, 0, 3));
        // Idempotent: setting the same selection returns false.
        assert!(!set_active_array_browser_selection(&mut state, Some(sel)));
        // Clearing returns true.
        assert!(set_active_array_browser_selection(&mut state, None));
    }

    #[test]
    fn array_browser_column_width_clamps_to_min() {
        let mut state = fresh_state_with_active_space("space-col");
        // 0.1rem is below the minimum 1.5rem clamp.
        let _ = set_active_array_browser_column_width(&mut state, 2, 0.1);
        let stored = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.column_widths_rem.get(&2).copied())
            .flatten()
            .expect("column width stored");
        assert!(stored >= 1.5 - f32::EPSILON);
    }

    #[test]
    fn array_browser_display_toggles_flip_settings() {
        let mut state = fresh_state_with_active_space("space-display");
        let initial = state.global_ui_chrome.array_browser_display;
        let _ = toggle_array_browser_display_option(
            &mut state,
            ArrayBrowserDisplayToggle::AlternatingRows,
        );
        assert_ne!(
            initial.show_alternating_rows,
            state
                .global_ui_chrome
                .array_browser_display
                .show_alternating_rows
        );
        let _ =
            toggle_array_browser_display_option(&mut state, ArrayBrowserDisplayToggle::GridLines);
        assert_ne!(
            initial.show_grid_lines,
            state.global_ui_chrome.array_browser_display.show_grid_lines
        );
    }

    #[test]
    fn input_event_updates_raw_text_and_editor_state_for_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: None,
                selection_end: None,
                input_kind: EditorInputKind::Other,
                inserted_text: None,
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(active.editor_surface_state.caret.offset, 11);
    }

    #[test]
    fn input_event_preserves_selection_metadata_when_provided() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2)".to_string(),
                selection_start: Some(2),
                selection_end: Some(5),
                input_kind: EditorInputKind::InsertText,
                inserted_text: Some("M".to_string()),
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.editor_surface_state.selection.anchor, 2);
        assert_eq!(active.editor_surface_state.selection.focus, 5);
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn command_updates_editor_state_and_clears_stale_analysis() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "SUM(\n1,\n2)");
        formula_space.editor_surface_state.selection = EditorSelection {
            anchor: 0,
            focus: formula_space.raw_entered_cell_text.chars().count(),
        };
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.effective_display_summary = Some("3".to_string());
        state.formula_spaces.insert(formula_space);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert!(active.raw_entered_cell_text.starts_with("    SUM("));
        assert!(active.latest_evaluation_summary.is_none());
        assert!(active.effective_display_summary.is_none());
    }

    /// §11.2 invariant 2: `apply_editor_command_to_active_formula_space`
    /// returns `false` when no active formula space exists (and is not a
    /// UI chrome command). Pins the guard in reducer.rs.
    #[test]
    fn apply_editor_command_returns_false_when_no_active_formula_space() {
        let mut state = OneCalcHostState::default();
        assert!(state.workspace_shell.active_formula_space_id.is_none());

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );
        assert!(!changed);

        let changed =
            apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CommitEntry);
        assert!(!changed);
    }

    /// §11.2 invariant 3: UI-chrome commands succeed even when no active
    /// formula space exists. `ToggleConfigureDrawer`,
    /// `ToggleEditorSettingsPopover`, and `UpdateEditorSetting(_)` mutate
    /// workspace-level state and must return `true` against an empty
    /// workspace.
    #[test]
    fn ui_chrome_commands_succeed_with_no_active_formula_space() {
        use crate::ui::editor::state::EditorSettingUpdate;
        let mut state = OneCalcHostState::default();

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleConfigureDrawer,
        ));
        assert!(state.global_ui_chrome.configure_drawer_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        ));
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        ));
        // The setting must have changed even though there's no formula space.
        assert!(
            !state
                .global_ui_chrome
                .editor_settings
                .highlight_bracket_pairs
        );
    }

    #[test]
    fn commit_entry_records_committed_text_and_transitions_live_state() {
        use crate::ui::editor::state::EditorLiveState;
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CommitEntry,
        ));
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.committed_cell_text.as_deref(), Some("=SUM(1,2)"));
        assert_eq!(active.live_state(), EditorLiveState::Committed);

        apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: None,
                selection_end: None,
                input_kind: EditorInputKind::Other,
                inserted_text: None,
            },
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::EditingLive);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::RequestProof);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::ProofedScratch);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CancelEntry);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(active.live_state(), EditorLiveState::Committed);
    }

    #[test]
    fn editor_settings_popover_toggle_and_update_apply_to_global_chrome() {
        use crate::ui::editor::state::{CompletionAggressiveness, EditorSettingUpdate};
        let mut state = OneCalcHostState::default();
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        );
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        );
        assert!(
            !state
                .global_ui_chrome
                .editor_settings
                .highlight_bracket_pairs
        );

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::SetCompletionAggressiveness(
                CompletionAggressiveness::Always,
            )),
        );
        assert_eq!(
            state
                .global_ui_chrome
                .editor_settings
                .completion_aggressiveness,
            CompletionAggressiveness::Always
        );
    }

    /// §11.5 invariant 9: F4 with a single-cell reference under the
    /// caret rotates one step in the cycle and the selection covers the
    /// rewritten reference. Walks all four steps of the cycle through the
    /// reducer entry point so the integration between
    /// `reference_cycle::cycle_reference_form` and the reducer's caret /
    /// selection update is pinned end-to-end.
    #[test]
    fn cycle_reference_form_rewrites_cell_and_selects_new_span() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=A1+B2");
        formula_space.editor_surface_state.caret =
            crate::ui::editor::state::EditorCaret { offset: 1 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(1);
        state.formula_spaces.insert(formula_space);

        // Step 1: A1 → $A$1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 5);

        // Step 2: $A$1 → A$1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 3: A$1 → $A1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 4: $A1 → A1 (back to the starting form)
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 3);
    }

    /// §11.5 invariant 8: accepting a completion proposal (via
    /// `AcceptCompletionByIndex(0)`) replaces the proposal's
    /// `replacement_span` with the `insert_text` and lands the caret at
    /// the end of the inserted text.
    #[test]
    fn accept_completion_from_tab_replaces_anchor_span_and_lands_caret_at_end() {
        use crate::adapters::oxfml::{
            CompletionProposal, CompletionProposalKind, EditorDocument, FormulaEditReuseSummary,
            FormulaTextSpan,
        };

        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.caret =
            crate::ui::editor::state::EditorCaret { offset: 3 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(3);
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SU".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: crate::test_support::make_editor_syntax_snapshot(
                "formula-1",
                "green-1",
                vec![],
            ),
            live_diagnostics: crate::test_support::empty_live_diagnostic_snapshot(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: false,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: None,
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "completion-SUM".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                documentation_ref: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        });
        state.formula_spaces.insert(formula_space);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::AcceptCompletionByIndex(0),
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        // Replacement span [1, 3) covers "SU"; after insert_text = "SUM("
        // the text becomes "=SUM(" and the caret lands at offset 5.
        assert_eq!(active.raw_entered_cell_text, "=SUM(");
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn dismiss_completion_clears_anchor_and_selected_index() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.completion_anchor_offset = Some(1);
        formula_space.editor_surface_state.completion_selected_index = Some(2);
        state.formula_spaces.insert(formula_space);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::DismissCompletion);

        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert!(active
            .editor_surface_state
            .completion_anchor_offset
            .is_none());
        assert!(active
            .editor_surface_state
            .completion_selected_index
            .is_none());
    }

    // ----------------------------------------------------------------
    // Browser-measured caret-box metrics (bead dno-xcq.22)
    // ----------------------------------------------------------------

    #[test]
    fn editor_box_metrics_default_to_none_with_zero_tick() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("u-1"), "=SUM(1,2)");
        assert!(formula_space.editor_box_metrics.is_none());
        assert_eq!(formula_space.editor_box_metrics_tick, 0);
    }

    #[test]
    fn applying_editor_box_metrics_sets_state_and_increments_tick() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let changed = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        assert!(changed, "first metric application changes state");

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(active.editor_box_metrics, Some(metrics));
        assert_eq!(active.editor_box_metrics_tick, 1);
    }

    #[test]
    fn applying_identical_editor_box_metrics_is_a_noop_and_does_not_increment_tick() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let _ = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        let changed_again = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        assert!(!changed_again, "identical metric is a no-op");

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(
            active.editor_box_metrics_tick, 1,
            "tick stays at 1 across identical re-applications",
        );
    }

    #[test]
    fn changing_editor_box_metrics_increments_tick_and_updates_state() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let initial = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let resized = TextareaMeasurementMetrics {
            char_width_px: 8,
            line_height_px: 20,
            scroll_top_px: 22,
            scroll_left_px: 0,
        };
        let _ = apply_editor_box_metrics_to_active_formula_space(&mut state, initial);
        let changed = apply_editor_box_metrics_to_active_formula_space(&mut state, resized);
        assert!(changed);

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(active.editor_box_metrics, Some(resized));
        assert_eq!(active.editor_box_metrics_tick, 2);
    }

    #[test]
    fn applying_editor_box_metrics_without_an_active_space_returns_false() {
        let mut state = OneCalcHostState::default();
        let changed = apply_editor_box_metrics_to_active_formula_space(
            &mut state,
            TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            },
        );
        assert!(!changed, "no active space -> no change reported");
    }

    #[test]
    fn overlay_measurement_event_updates_geometry_on_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_overlay_measurement_to_active_formula_space(
            &mut state,
            EditorOverlayMeasurementEvent {
                snapshot: EditorOverlayGeometrySnapshot {
                    caret_box: Some(EditorMeasuredOverlayBox {
                        top_px: 40,
                        left_px: 64,
                        width_px: 2,
                        height_px: 22,
                        line_index: 0,
                        column_index: 4,
                    }),
                    selection_box: None,
                    completion_anchor_box: None,
                    signature_help_anchor_box: None,
                    completion_popup_box: None,
                    signature_help_popup_box: None,
                },
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(
            active
                .editor_overlay_geometry
                .as_ref()
                .and_then(|geometry| geometry.caret_box.as_ref())
                .map(|box_geometry| box_geometry.left_px),
            Some(64)
        );
    }

    #[test]
    fn open_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=3 excel=4".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog(&mut state, "artifact-1");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-1")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn importing_manual_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let imported = import_manual_retained_artifact_into_active_formula_space(
            &mut state,
            crate::services::retained_artifacts::ManualRetainedArtifactImportRequest {
                artifact_id: "artifact-2".to_string(),
                case_id: "case-2".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Blocked,
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        assert!(imported);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-2")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn open_retained_artifact_in_inspect_routes_shell_to_inspect_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-inspect".to_string(),
                    case_id: "case-inspect".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Blocked,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog_in_inspect(&mut state, "artifact-inspect");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-inspect")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Inspect
        );
    }

    #[test]
    fn view_mode_default_is_user() {
        let state = OneCalcHostState::default();
        assert_eq!(state.view_mode, crate::state::ViewMode::User);
    }

    #[test]
    fn toggle_view_mode_flips_and_returns_new_value() {
        let mut state = OneCalcHostState::default();
        let after_first = toggle_view_mode_on_workspace(&mut state);
        assert_eq!(after_first, crate::state::ViewMode::Developer);
        assert_eq!(state.view_mode, crate::state::ViewMode::Developer);
        let after_second = toggle_view_mode_on_workspace(&mut state);
        assert_eq!(after_second, crate::state::ViewMode::User);
        assert_eq!(state.view_mode, crate::state::ViewMode::User);
    }

    #[test]
    fn set_view_mode_returns_true_when_changed_false_when_unchanged() {
        let mut state = OneCalcHostState::default();
        assert!(set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::Developer
        ));
        assert_eq!(state.view_mode, crate::state::ViewMode::Developer);
        assert!(!set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::Developer
        ));
        assert!(set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::User
        ));
    }

    #[test]
    fn toggle_formula_drill_flips_state_and_returns_new_value() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), ""));

        // Default is closed.
        let formula_space = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("formula space");
        assert!(!formula_space.formula_drill_open);

        // First toggle opens.
        let now_open = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(now_open);
        assert!(
            state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );

        // Second toggle closes.
        let now_open = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(!now_open);
        assert!(
            !state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );
    }

    #[test]
    fn toggle_formula_drill_no_op_when_no_active_formula_space() {
        let mut state = OneCalcHostState::default();
        // No active formula space — toggle returns false (treated as
        // a no-op by the caller).
        let result = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(!result);
    }

    #[test]
    fn close_formula_drill_returns_false_when_already_closed() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, ""));

        let changed = close_formula_drill_on_active_formula_space(&mut state);
        assert!(!changed);
    }

    #[test]
    fn close_formula_drill_closes_open_panel() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "");
        formula_space.formula_drill_open = true;
        state.formula_spaces.insert(formula_space);

        let changed = close_formula_drill_on_active_formula_space(&mut state);
        assert!(changed);
        assert!(
            !state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );
    }
}
