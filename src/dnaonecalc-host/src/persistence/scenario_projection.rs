//! Pure projection between `FormulaSpaceState` and the persisted
//! `Scenario` shape. No I/O — those live in `browser_file_io.rs` and
//! the (eventual) Tauri equivalent.
//!
//! For slice 1b, this layer maps the live state's actually-populated
//! fields into the schema. Fields that exist in the schema but are
//! not yet wired in the host (most of `Context.publication_context`,
//! the host_profile detail, locale id, etc.) round-trip as their
//! defaults — slice 2 (Excel-native fidelity) and the eventual
//! formatting-controls work fill them in. The schema slot is
//! reserved either way.

use crate::persistence::formula_file::{
    Context, Entry, EntryMode, HostProfile, Identity, Locale, PublicationContext, Scenario,
    ScenarioPolicy, UiPreferences,
};
use crate::state::FormulaSpaceState;
use crate::ui::editor::state::EditorEntryMode;

/// Project the live `FormulaSpaceState` into a `Scenario` ready for
/// serialisation. The caller is responsible for supplying the
/// timestamps (which depend on platform clock — see browser/tauri
/// adapters).
///
/// `created_at` should be threaded through from the formula's
/// existing identity when one exists, and supplied as "now" only on
/// the very first save. `modified_at` is "now" on every save.
pub fn formula_space_to_scenario(
    formula_space: &FormulaSpaceState,
    created_at_iso8601_utc: String,
    modified_at_iso8601_utc: String,
) -> Scenario {
    let entry_mode = match EditorEntryMode::classify(&formula_space.raw_entered_cell_text) {
        EditorEntryMode::Formula => EntryMode::Formula,
        EditorEntryMode::Value => EntryMode::Value,
        EditorEntryMode::Text => EntryMode::Text,
        EditorEntryMode::Empty => EntryMode::Empty,
    };

    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let display_name = if synthetic_default_label {
        // Synthetic labels (the `untitled-N` ids the host auto-generates
        // when a formula has no user-given name) are not user-meaningful;
        // empty `name` is more honest in the persisted file. The Identity
        // `id` still carries the synthetic id for stability.
        String::new()
    } else {
        formula_space.context.scenario_label.clone()
    };

    Scenario {
        identity: Identity {
            id: formula_space.formula_space_id.as_str().to_string(),
            name: display_name,
            created_at: created_at_iso8601_utc,
            modified_at: modified_at_iso8601_utc,
        },
        entry: Entry {
            mode: entry_mode,
            text: formula_space.raw_entered_cell_text.clone(),
        },
        // Slice 1b — most context fields aren't yet wired into
        // FormulaSpaceState. They are reserved schema slots; slice 2
        // and the formatting-controls slice fill them in. The
        // ScenarioPolicy default (`Deterministic`) matches the WS-14
        // plan §6.4 default.
        context: Context {
            host_profile: HostProfile::default(),
            locale: Locale::default(),
            publication_context: PublicationContext::default(),
            scenario_policy: ScenarioPolicy::Deterministic,
        },
        ui_preferences: UiPreferences {
            formula_drill_expanded: formula_space.formula_drill_open,
            // `result_drill_open` is not yet a state field; reserve.
            result_drill_expanded: false,
            expanded_editor: formula_space.expanded_editor,
        },
    }
}

/// Apply a loaded `Scenario` to an existing `FormulaSpaceState`,
/// overwriting the live fields. The caller decides whether to apply
/// to the active formula space (replacing it) or to insert a new
/// space and switch to it; this helper just mutates a given target.
///
/// Post-condition:
/// - `raw_entered_cell_text` is the loaded entry text.
/// - `committed_cell_text` is set equal to `raw_entered_cell_text`,
///   so the breadcrumb's dirty marker reads `false` immediately
///   after loading (the user has not yet edited).
/// - `context.scenario_label` is the loaded display name when the
///   loaded `name` is non-empty; otherwise the `id` is used as the
///   label so the breadcrumb has something to render.
/// - UI prefs follow the loaded scenario.
pub fn apply_loaded_scenario_to_formula_space(
    formula_space: &mut FormulaSpaceState,
    scenario: Scenario,
) {
    formula_space.raw_entered_cell_text = scenario.entry.text.clone();
    formula_space.committed_cell_text = Some(scenario.entry.text.clone());
    formula_space.proofed_cell_text = Some(scenario.entry.text.clone());
    formula_space.editor_surface_state =
        crate::ui::editor::state::EditorSurfaceState::for_text(&scenario.entry.text);
    formula_space.editor_document = None;
    formula_space.completion_help = crate::state::CompletionHelpState::default();
    formula_space.completion_popup =
        crate::services::completion_popup::CompletionPopupState::default();
    formula_space.completion_popup_suppressed_until_next_input = false;
    formula_space.array_preview = None;
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;

    let label = if scenario.identity.name.is_empty() {
        scenario.identity.id.clone()
    } else {
        scenario.identity.name.clone()
    };
    formula_space.context.scenario_label = label;

    formula_space.formula_drill_open = scenario.ui_preferences.formula_drill_expanded;
    formula_space.expanded_editor = scenario.ui_preferences.expanded_editor;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::FormulaSpaceState;

    #[test]
    fn formula_space_with_user_label_round_trips_into_identity_name() {
        let mut formula_space = FormulaSpaceState::new(
            FormulaSpaceId::new("untitled-1"),
            "=SUM(1,2,3)",
        );
        formula_space.context.scenario_label = "invoice-eu-tax".to_string();

        let scenario = formula_space_to_scenario(
            &formula_space,
            "2026-04-22T10:14:22Z".to_string(),
            "2026-04-22T10:14:22Z".to_string(),
        );

        assert_eq!(scenario.identity.id, "untitled-1");
        assert_eq!(scenario.identity.name, "invoice-eu-tax");
        assert_eq!(scenario.entry.text, "=SUM(1,2,3)");
        assert_eq!(scenario.entry.mode, EntryMode::Formula);
    }

    #[test]
    fn synthetic_default_label_projects_to_empty_name() {
        // FormulaSpaceState::new auto-sets scenario_label = formula_space_id.
        // The persisted file should NOT carry the synthetic id as the
        // name — empty is more honest, and the breadcrumb fallback to
        // `id` happens at apply-time.
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        let scenario = formula_space_to_scenario(
            &formula_space,
            "now".to_string(),
            "now".to_string(),
        );
        assert_eq!(scenario.identity.id, "untitled-1");
        assert_eq!(scenario.identity.name, "");
    }

    #[test]
    fn apply_loaded_scenario_clears_dirty_marker() {
        let mut formula_space = FormulaSpaceState::new(
            FormulaSpaceId::new("untitled-1"),
            "live edit text",
        );
        // Simulate the user having committed an earlier text.
        formula_space.committed_cell_text = Some("live edit text".to_string());

        let scenario = Scenario {
            identity: Identity {
                id: "loaded-id".to_string(),
                name: "loaded-name".to_string(),
                created_at: "2026-04-22T10:14:22Z".to_string(),
                modified_at: "2026-04-22T10:14:22Z".to_string(),
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=A1+B1".to_string(),
            },
            context: Context::default(),
            ui_preferences: UiPreferences {
                formula_drill_expanded: true,
                result_drill_expanded: false,
                expanded_editor: true,
            },
        };
        apply_loaded_scenario_to_formula_space(&mut formula_space, scenario);

        // Loaded text replaces both raw and committed → dirty=false
        // immediately after loading.
        assert_eq!(formula_space.raw_entered_cell_text, "=A1+B1");
        assert_eq!(
            formula_space.committed_cell_text.as_deref(),
            Some("=A1+B1"),
        );
        assert_eq!(formula_space.context.scenario_label, "loaded-name");
        assert!(formula_space.formula_drill_open);
        assert!(formula_space.expanded_editor);
        assert!(formula_space.editor_document.is_none());
    }

    #[test]
    fn apply_loaded_scenario_with_empty_name_falls_back_to_id_label() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let scenario = Scenario {
            identity: Identity {
                id: "imported-from-disk".to_string(),
                name: String::new(),
                created_at: String::new(),
                modified_at: String::new(),
            },
            entry: Entry {
                mode: EntryMode::Empty,
                text: String::new(),
            },
            context: Context::default(),
            ui_preferences: UiPreferences::default(),
        };
        apply_loaded_scenario_to_formula_space(&mut formula_space, scenario);
        assert_eq!(
            formula_space.context.scenario_label,
            "imported-from-disk",
        );
    }

    #[test]
    fn entry_mode_classifier_drives_projection() {
        let cases = [
            ("=SUM(1)", EntryMode::Formula),
            ("'hello", EntryMode::Text),
            ("42", EntryMode::Value),
            ("", EntryMode::Empty),
        ];
        for (text, expected_mode) in cases {
            let formula_space =
                FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), text);
            let scenario = formula_space_to_scenario(
                &formula_space,
                "now".to_string(),
                "now".to_string(),
            );
            assert_eq!(
                scenario.entry.mode, expected_mode,
                "text {text:?} should classify to {expected_mode:?}",
            );
            assert_eq!(scenario.entry.text, text);
        }
    }
}
