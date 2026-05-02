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

    let formatting = &formula_space.formatting;
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
        // Slice 5 — formatting fields drive PublicationContext +
        // Locale. Other Context fields (host_profile,
        // scenario_policy beyond default) remain reserved schema
        // slots until their UI controls land.
        context: Context {
            host_profile: HostProfile::default(),
            locale: Locale {
                id: String::new(),
                date1904: formatting.date1904,
            },
            publication_context: PublicationContext {
                format_profile: String::new(),
                number_format_code: formatting.number_format_code.clone(),
                style_id: String::new(),
                font_color: formatting.font_color.clone(),
                fill_color: formatting.fill_color.clone(),
                style_hierarchy: Vec::new(),
                cf_rules: Vec::new(),
            },
            scenario_policy: ScenarioPolicy::Deterministic,
        },
        ui_preferences: UiPreferences {
            formula_drill_expanded: formula_space.formula_drill_open,
            // `result_drill_open` is not yet a state field; reserve.
            result_drill_expanded: false,
            expanded_editor: formula_space.expanded_editor,
        },
        // Compare bundles aren't yet attached to FormulaSpaceState
        // (no in-memory home for them yet — they live on the
        // persisted file). Slice 4 ships the format slot; the
        // workspace state field that mirrors them is a follow-up
        // alongside the Compare-with-Excel UI surface.
        bundles: Vec::new(),
        // Unknown-element preservation: not yet plumbed into the
        // host state. When the host opens a file via slice 1b's
        // file picker the full LoadedFormula (including the
        // unknowns vec) is dropped — first save loses them. A
        // follow-up bead lifts them onto FormulaSpaceState so
        // they survive the open→edit→save round-trip.
        unknown_root_xml: Vec::new(),
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
/// - `load_diagnostics` carries any loader warnings (slice 3).
pub fn apply_loaded_scenario_to_formula_space(
    formula_space: &mut FormulaSpaceState,
    scenario: Scenario,
) {
    apply_loaded_scenario_with_diagnostics(formula_space, scenario, Vec::new());
}

/// Variant that also stamps the loader's diagnostics into the
/// formula-space state. The status-foot renders a warning chip
/// while `load_diagnostics` is non-empty; cleared on save.
pub fn apply_loaded_scenario_with_diagnostics(
    formula_space: &mut FormulaSpaceState,
    scenario: Scenario,
    diagnostics: Vec<crate::persistence::LoadDiagnostic>,
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
    formula_space.load_diagnostics = diagnostics;

    // Slice 5: formatting state mirrors the persisted PublicationContext
    // + Locale so the UI's formatting-controls row reflects what was
    // saved.
    formula_space.formatting = crate::state::FormulaFormattingState {
        number_format_code: scenario.context.publication_context.number_format_code,
        font_color: scenario.context.publication_context.font_color,
        fill_color: scenario.context.publication_context.fill_color,
        date1904: scenario.context.locale.date1904,
    };
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
            ui_preferences: UiPreferences {
                formula_drill_expanded: true,
                result_drill_expanded: false,
                expanded_editor: true,
            },
            ..Scenario::default()
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
                ..Identity::default()
            },
            entry: Entry {
                mode: EntryMode::Empty,
                text: String::new(),
            },
            ..Scenario::default()
        };
        apply_loaded_scenario_to_formula_space(&mut formula_space, scenario);
        assert_eq!(
            formula_space.context.scenario_label,
            "imported-from-disk",
        );
    }

    #[test]
    fn formatting_state_round_trips_through_publication_context_and_locale() {
        // Slice 5: FormulaFormattingState mutations must travel into
        // the persisted Scenario's PublicationContext + Locale, then
        // come back unchanged on load.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("f-1"), "=A1");
        formula_space.formatting = crate::state::FormulaFormattingState {
            number_format_code: "$#,##0.00".to_string(),
            font_color: "#112233".to_string(),
            fill_color: "#445566".to_string(),
            date1904: true,
        };

        let scenario = formula_space_to_scenario(
            &formula_space,
            "now".to_string(),
            "now".to_string(),
        );
        assert_eq!(
            scenario.context.publication_context.number_format_code,
            "$#,##0.00",
        );
        assert_eq!(scenario.context.publication_context.font_color, "#112233");
        assert_eq!(scenario.context.publication_context.fill_color, "#445566");
        assert!(scenario.context.locale.date1904);

        // Apply the same scenario back into a fresh formula space —
        // the formatting state must round-trip verbatim.
        let mut destination =
            FormulaSpaceState::new(FormulaSpaceId::new("f-2"), "");
        apply_loaded_scenario_to_formula_space(&mut destination, scenario);
        assert_eq!(destination.formatting.number_format_code, "$#,##0.00");
        assert_eq!(destination.formatting.font_color, "#112233");
        assert_eq!(destination.formatting.fill_color, "#445566");
        assert!(destination.formatting.date1904);
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
