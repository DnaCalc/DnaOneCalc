use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaFormattingCfAverageRuleOptions,
    FormulaFormattingCfColorScaleRuleOptions, FormulaFormattingCfColorScaleStop,
    FormulaFormattingCfDataBarDirection, FormulaFormattingCfDataBarRuleOptions,
    FormulaFormattingCfIconSetRuleOptions, FormulaFormattingCfRank,
    FormulaFormattingCfRankRuleOptions, FormulaFormattingCfRule, FormulaFormattingCfThreshold,
    FormulaFormattingCfTypedRule, FormulaFormattingRequest, OxfmlEditorBridge,
    OxfmlEditorBridgeError, RecalcModeRequest, ScenarioPolicyRequest,
};
use crate::app::intents::ApplyFormulaEditIntent;
use crate::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
};
use crate::domain::ids::FormulaSpaceId;
use crate::services::completion_popup::{
    sync_completion_popup_with_proposals, CompletionPopupItem,
};
use crate::services::editor_session::{EditorSessionError, EditorSessionService};
use crate::state::{FormulaSpaceState, OneCalcHostState};
use crate::ui::editor::commands::{EditorCommand, EditorInputEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveEditError {
    NoActiveFormulaSpace,
    UnknownFormulaSpace(FormulaSpaceId),
    Session(EditorSessionError),
    Bridge(OxfmlEditorBridgeError),
}

pub fn apply_live_editor_input(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    input_event: EditorInputEvent,
) -> Result<bool, LiveEditError> {
    let formula_space_id =
        active_formula_space_id(state).ok_or(LiveEditError::NoActiveFormulaSpace)?;
    let input_kind = input_event.input_kind;
    let changed = apply_editor_input_to_active_formula_space(state, input_event);
    if !changed {
        return Ok(false);
    }
    let skip_runtime = !input_kind.mutates_text();
    refresh_active_formula_space_from_bridge(bridge, state, &formula_space_id, skip_runtime)?;
    Ok(true)
}

pub fn apply_live_editor_command(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    command: EditorCommand,
) -> Result<bool, LiveEditError> {
    let formula_space_id =
        active_formula_space_id(state).ok_or(LiveEditError::NoActiveFormulaSpace)?;
    let changed = apply_editor_command_to_active_formula_space(state, command.clone());
    if !changed {
        return Ok(false);
    }
    if matches!(
        command,
        EditorCommand::SelectPreviousCompletion
            | EditorCommand::SelectNextCompletion
            | EditorCommand::SelectCompletionByIndex(_)
            | EditorCommand::CommitEntry
            | EditorCommand::RequestProof
            | EditorCommand::ForceShowCompletion
            | EditorCommand::DismissCompletion
            | EditorCommand::ToggleExpandedHeight
            | EditorCommand::SendSelectionToInspect
            | EditorCommand::ToggleEditorSettingsPopover
            | EditorCommand::UpdateEditorSetting(_)
            | EditorCommand::ToggleConfigureDrawer
    ) {
        return Ok(true);
    }
    refresh_active_formula_space_from_bridge(bridge, state, &formula_space_id, false)?;
    Ok(true)
}

/// Re-run the live bridge for the active formula space without
/// going through an editor command or input event. Used by the
/// formatting-controls callbacks: when the user changes the
/// number-format code (or font / fill colour, or the 1904 toggle),
/// the editor text hasn't changed but the
/// `VerificationPublicationContext` we ship to OxFml has — and the
/// resulting `effective_display_text` needs to flow back to the
/// result hero immediately, not on the next keystroke.
///
/// Returns `Ok(true)` when the bridge ran, `Ok(false)` when there
/// was no active formula space (treated as a benign no-op by the
/// renderer), and `Err` only when the bridge itself failed.
pub fn refresh_active_formula_space(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
) -> Result<bool, LiveEditError> {
    let Some(formula_space_id) = active_formula_space_id(state) else {
        return Ok(false);
    };
    refresh_active_formula_space_from_bridge(bridge, state, &formula_space_id, false)?;
    Ok(true)
}

/// Force a runtime-evaluation pass on the active formula even when
/// the recalc mode is `Manual`. Hooked up to F9 / Calculate. Returns
/// `Ok(true)` when the bridge ran, `Ok(false)` for no active space,
/// `Err` only on bridge failure.
pub fn force_runtime_recalc_on_active_formula_space(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
) -> Result<bool, LiveEditError> {
    let Some(formula_space_id) = active_formula_space_id(state) else {
        return Ok(false);
    };
    refresh_active_formula_space_from_bridge_with_overrides(
        bridge,
        state,
        &formula_space_id,
        false,
        true,
    )?;
    Ok(true)
}

fn active_formula_space_id(state: &OneCalcHostState) -> Option<FormulaSpaceId> {
    state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())
}

fn refresh_active_formula_space_from_bridge(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
    skip_runtime_evaluation: bool,
) -> Result<(), LiveEditError> {
    refresh_active_formula_space_from_bridge_with_overrides(
        bridge,
        state,
        formula_space_id,
        skip_runtime_evaluation,
        false,
    )
}

/// Inner refresh path. `skip_runtime_evaluation` is the per-event
/// flag (true for caret-only events). `force_runtime` overrides the
/// per-formula recalc mode — used by F9 / Calculate to bypass
/// `ManualRecalc` for one round-trip.
fn refresh_active_formula_space_from_bridge_with_overrides(
    bridge: &dyn OxfmlEditorBridge,
    state: &mut OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
    skip_runtime_evaluation: bool,
    force_runtime: bool,
) -> Result<(), LiveEditError> {
    let formula_space = state
        .formula_spaces
        .get(formula_space_id)
        .ok_or_else(|| LiveEditError::UnknownFormulaSpace(formula_space_id.clone()))?;
    let intent = build_live_edit_intent(formula_space, skip_runtime_evaluation, force_runtime);
    EditorSessionService::handle_formula_edit_intent(bridge, &mut state.formula_spaces, intent)
        .map_err(|error| match error {
            EditorSessionError::UnknownFormulaSpace(id) => LiveEditError::UnknownFormulaSpace(id),
            EditorSessionError::Bridge(bridge_error) => LiveEditError::Bridge(bridge_error),
        })?;
    apply_presentation_hint_when_general(state, formula_space_id);
    sync_completion_popup_after_bridge_refresh(state, formula_space_id);
    Ok(())
}

/// When the bridge returned a presentation hint and the formula's
/// formatting state is empty (General default), apply the canonical
/// default format code derived from the workspace's
/// `AmbientAppContext` so a fresh `=NOW()` reads as date+time in
/// the user's preferred shape without them having to pick a format
/// manually. Once the user picks an explicit code, the hint stops
/// winning — explicit host state always trumps the function-emitted
/// hint.
///
/// One-shot per bridge round: set the format the moment a hint
/// becomes available and the field is empty, then leave it alone.
fn apply_presentation_hint_when_general(
    state: &mut OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
) {
    let ambient = state.ambient_app_context.clone();
    let Some(formula_space) = state.formula_spaces.get_mut(formula_space_id) else {
        return;
    };
    if !formula_space
        .formatting
        .number_format_code
        .trim()
        .is_empty()
    {
        return;
    }
    let Some(presentation) = formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.value_presentation.as_ref())
    else {
        return;
    };
    let Some(hint) = presentation.number_format_hint else {
        return;
    };
    let value_kind = ValueShapeForHint::from_eval_value(&presentation.published_value);
    let Some(default_code) = default_format_code_for_hint(hint, &ambient, value_kind) else {
        return;
    };
    formula_space.formatting.number_format_code = default_code;
}

/// Disambiguator for the `DateLike` hint. Both `=NOW()` and
/// `=TODAY()` emit `NumberFormatHint::DateLike`, but the user wants
/// `=TODAY()` to render as date-only (`yyyy/mm/dd`) and `=NOW()` as
/// datetime (`yyyy/mm/dd HH:mm:ss`). Excel serials make this trivial
/// to spot from the value: TODAY returns an integer serial, NOW
/// returns a fractional one. The heuristic rounds-trip via
/// `f64::fract` so any arithmetic that produces an integer-valued
/// serial (`=DATE(...)`, `=NOW()-MOD(NOW(),1)`, etc.) routes
/// correctly to the date-only default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueShapeForHint {
    IntegerSerial,
    FractionalSerial,
    Other,
}

impl ValueShapeForHint {
    fn from_eval_value(value: &crate::adapters::oxfml::EvalValue) -> Self {
        use crate::adapters::oxfml::EvalValue;
        match value {
            EvalValue::Number(number) if number.is_finite() => {
                if number.fract() == 0.0 {
                    Self::IntegerSerial
                } else {
                    Self::FractionalSerial
                }
            }
            _ => Self::Other,
        }
    }
}

/// Canonical Excel format code for each hint variant, sourced from
/// the workspace's `AmbientAppContext` for date / time-bearing
/// hints and from a small pre-MVP table of stable defaults for the
/// purely numeric hints. `None` means the host has nothing better
/// to apply than General.
///
/// `DateLike` is the only hint that goes through the ambient
/// context today, and it splits two ways based on the value's
/// fractional part: integer Excel serials (TODAY) land on
/// `date_format_code`; fractional serials (NOW) land on
/// `datetime_format_code`.
fn default_format_code_for_hint(
    hint: crate::adapters::oxfml::NumberFormatHint,
    ambient: &crate::state::AmbientAppContext,
    value_kind: ValueShapeForHint,
) -> Option<String> {
    use crate::adapters::oxfml::NumberFormatHint as H;
    match hint {
        H::DateLike => Some(match value_kind {
            ValueShapeForHint::IntegerSerial => ambient.date_format_code.clone(),
            ValueShapeForHint::FractionalSerial => ambient.datetime_format_code.clone(),
            // Non-Number values with a DateLike hint are unusual;
            // fall back to datetime which is the more general
            // shape (it strictly contains the date-only shape).
            ValueShapeForHint::Other => ambient.datetime_format_code.clone(),
        }),
        H::Currency => Some("$#,##0.00".to_string()),
        H::Percentage => Some("0.00%".to_string()),
        H::Scientific => Some("0.00E+00".to_string()),
        H::Fraction => Some("# ?/?".to_string()),
        H::General | H::Custom => None,
    }
}

/// Apply the popup auto-open / auto-close policy after a successful
/// bridge refresh. Pulls `editor_document.completion_proposals` off
/// the formula space, lifts each proposal to a [`CompletionPopupItem`]
/// at the popup-state layer, and dispatches
/// [`sync_completion_popup_with_proposals`].
///
/// Anchor offset comes from the proposal's `replacement_span.start`
/// when present (matches Excel's authoring behaviour where the popup
/// anchors at the start of the partial token being completed); falls
/// back to the current caret offset otherwise. The same anchor is
/// used for every item in the list because the bridge returns
/// proposals for one trigger position.
///
/// Host-side filter: when the raw textarea text is empty OR when no
/// proposal has a non-zero replacement-span length, drop the
/// proposals to an empty list before syncing. The upstream bridge
/// returns ALL function names for an empty prefix (67 items at the
/// time of writing); surfacing that as a popup is poor UX. The
/// filter mirrors Excel's behaviour of suppressing the function
/// dropdown until the user has typed at least one character of a
/// function name.
///
/// No-op when the formula space is missing (bridge already errored).
fn sync_completion_popup_after_bridge_refresh(
    state: &mut OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
) {
    let Some(formula_space) = state.formula_spaces.get_mut(formula_space_id) else {
        return;
    };
    let document = match &formula_space.editor_document {
        Some(document) => document,
        None => {
            // No document yet means no proposals to drive the popup;
            // ensure it is closed.
            let _ = sync_completion_popup_with_proposals(
                &mut formula_space.completion_popup,
                formula_space.editor_surface_state.caret.offset,
                Vec::new(),
            );
            return;
        }
    };
    let raw_is_empty = formula_space.raw_entered_cell_text.is_empty();
    let has_useful_prefix = document
        .completion_proposals
        .iter()
        .any(|proposal| proposal.replacement_span.is_some_and(|span| span.len > 0));
    let was_suppressed = formula_space.completion_popup_suppressed_until_next_input;
    formula_space.completion_popup_suppressed_until_next_input = false;
    let items: Vec<CompletionPopupItem> = if raw_is_empty || !has_useful_prefix || was_suppressed {
        Vec::new()
    } else {
        document
            .completion_proposals
            .iter()
            .cloned()
            .map(CompletionPopupItem::from_proposal)
            .collect()
    };
    let anchor_offset = document
        .completion_proposals
        .first()
        .and_then(|proposal| proposal.replacement_span)
        .map(|span| span.start)
        .unwrap_or(formula_space.editor_surface_state.caret.offset);
    let _ = sync_completion_popup_with_proposals(
        &mut formula_space.completion_popup,
        anchor_offset,
        items,
    );
}

fn build_live_edit_intent(
    formula_space: &FormulaSpaceState,
    skip_runtime_evaluation: bool,
    force_runtime: bool,
) -> ApplyFormulaEditIntent {
    let formula_stable_id = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.formula_stable_id.clone())
        .unwrap_or_else(|| formula_space.formula_space_id.as_str().to_string());

    // Full-semantic-plan stage so live diagnostics include
    // SemanticPlan-stage codes such as `unknown_function` and
    // `function_gated_or_unavailable` (W067). With only
    // `SyntaxAndBind` the editor surfaces `unknown_name` (Bind) but
    // not `unknown_function` (SemanticPlan), which is a fidelity
    // gap the user notices on day one.
    //
    // Runtime-pass gating happens through `skip_runtime_evaluation`
    // (per-event, set on caret-only navigation) and `recalc_mode`
    // (per-formula, `Manual` skips runtime on every event). The
    // bridge runs parse / bind / popup / signature-help every time
    // either way — only the value-evaluation pass is gated.
    let recalc_mode = if force_runtime {
        // F9 / Calculate force a recalc even when in Manual mode.
        RecalcModeRequest::Auto
    } else {
        recalc_mode_request_for(formula_space)
    };
    ApplyFormulaEditIntent {
        formula_space_id: formula_space.formula_space_id.clone(),
        formula_stable_id,
        entered_text: formula_space.raw_entered_cell_text.clone(),
        cursor_offset: formula_space.editor_surface_state.caret.offset,
        analysis_stage: EditorAnalysisStage::FullSemanticPlan,
        formatting_request: formatting_request_for(formula_space),
        scenario_policy: scenario_policy_request_for(formula_space),
        skip_runtime_evaluation,
        recalc_mode,
    }
}

fn scenario_policy_request_for(formula_space: &FormulaSpaceState) -> ScenarioPolicyRequest {
    use crate::persistence::ScenarioPolicy;
    match formula_space.formatting.scenario_policy {
        ScenarioPolicy::Deterministic => ScenarioPolicyRequest::Deterministic,
        ScenarioPolicy::LiveRecalc => ScenarioPolicyRequest::LiveRecalc,
        ScenarioPolicy::ManualRecalc => ScenarioPolicyRequest::ManualRecalc,
    }
}

fn recalc_mode_request_for(formula_space: &FormulaSpaceState) -> RecalcModeRequest {
    use crate::persistence::ScenarioPolicy;
    match formula_space.formatting.scenario_policy {
        ScenarioPolicy::Deterministic | ScenarioPolicy::LiveRecalc => RecalcModeRequest::Auto,
        ScenarioPolicy::ManualRecalc => RecalcModeRequest::Manual,
    }
}

/// Lift the formula's editable formatting state to a bridge-payload
/// `FormulaFormattingRequest`. Returns `None` when every editable
/// surface is empty (no number-format code, no font / fill colours,
/// no CF rules, and the calendar still on 1900) — the runtime pass
/// then skips the `verification_publication_surface.effective_display_text`
/// computation entirely and the result hero falls back to the raw
/// value.
///
/// `style_id` stays `None` here because slice-5 only exposes the four
/// scalar surfaces; richer style hierarchies arrive with the
/// formatting-controls panel (see WS-14 plan §5.3, item 8).
fn formatting_request_for(formula_space: &FormulaSpaceState) -> Option<FormulaFormattingRequest> {
    let formatting = &formula_space.formatting;
    let number_format_code = empty_to_none(&formatting.number_format_code);
    let font_color = empty_to_none(&formatting.font_color);
    let fill_color = empty_to_none(&formatting.fill_color);
    let date1904 = formatting.date1904;
    let cf_rules: Vec<FormulaFormattingCfRule> = formatting
        .conditional_formatting_rules
        .iter()
        .map(|rule| FormulaFormattingCfRule {
            rule_kind: rule.rule_kind.clone(),
            operator: rule.operator.clone(),
            thresholds: rule.thresholds.clone(),
            font_color: rule.font_color.clone(),
            fill_color: rule.fill_color.clone(),
            typed_rule: rule.typed_rule.as_ref().map(host_typed_rule_to_bridge),
        })
        .collect();
    if number_format_code.is_none()
        && font_color.is_none()
        && fill_color.is_none()
        && !date1904
        && cf_rules.is_empty()
    {
        return None;
    }
    Some(FormulaFormattingRequest {
        number_format_code,
        font_color,
        fill_color,
        style_id: None,
        date1904,
        conditional_formatting_rules: cf_rules,
    })
}

fn empty_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn host_typed_rule_to_bridge(
    rule: &crate::state::FormulaConditionalFormattingTypedRule,
) -> FormulaFormattingCfTypedRule {
    use crate::state::{
        FormulaConditionalFormattingRank, FormulaConditionalFormattingThreshold,
        FormulaDataBarDirection,
    };
    let host_threshold = |threshold: &FormulaConditionalFormattingThreshold| match threshold {
        FormulaConditionalFormattingThreshold::Min => FormulaFormattingCfThreshold::Min,
        FormulaConditionalFormattingThreshold::Mid => FormulaFormattingCfThreshold::Mid,
        FormulaConditionalFormattingThreshold::Max => FormulaFormattingCfThreshold::Max,
        FormulaConditionalFormattingThreshold::Percent(value) => {
            FormulaFormattingCfThreshold::Percent(*value)
        }
        FormulaConditionalFormattingThreshold::Percentile(value) => {
            FormulaFormattingCfThreshold::Percentile(*value)
        }
        FormulaConditionalFormattingThreshold::Number(value) => {
            FormulaFormattingCfThreshold::Number(*value)
        }
    };
    FormulaFormattingCfTypedRule {
        color_scale: rule.color_scale.as_ref().map(|options| {
            FormulaFormattingCfColorScaleRuleOptions {
                stops: options
                    .stops
                    .iter()
                    .map(|stop| FormulaFormattingCfColorScaleStop {
                        position: host_threshold(&stop.position),
                        color: stop.color.clone(),
                    })
                    .collect(),
            }
        }),
        data_bar: rule
            .data_bar
            .as_ref()
            .map(|options| FormulaFormattingCfDataBarRuleOptions {
                minimum: options.minimum.as_ref().map(&host_threshold),
                maximum: options.maximum.as_ref().map(&host_threshold),
                bar_color: options.bar_color.clone(),
                direction: options.direction.map(|direction| match direction {
                    FormulaDataBarDirection::Left => FormulaFormattingCfDataBarDirection::Left,
                    FormulaDataBarDirection::Right => FormulaFormattingCfDataBarDirection::Right,
                }),
                show_bar_only: options.show_bar_only,
            }),
        icon_set: rule
            .icon_set
            .as_ref()
            .map(|options| FormulaFormattingCfIconSetRuleOptions {
                set_kind: options.set_kind.clone(),
                thresholds: options.thresholds.iter().map(&host_threshold).collect(),
            }),
        rank: rule
            .rank
            .as_ref()
            .map(|options| FormulaFormattingCfRankRuleOptions {
                rank: match &options.rank {
                    FormulaConditionalFormattingRank::Count(count) => {
                        FormulaFormattingCfRank::Count(*count)
                    }
                    FormulaConditionalFormattingRank::Percent(value) => {
                        FormulaFormattingCfRank::Percent(*value)
                    }
                },
            }),
        average: rule
            .average
            .as_ref()
            .map(|options| FormulaFormattingCfAverageRuleOptions {
                include_equal: options.include_equal,
                stddev_multiplier: options.stddev_multiplier,
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{FormulaEditRequest, FormulaEditResult};
    use crate::state::FormulaSpaceState;
    use crate::test_support::sample_editor_document;
    use crate::ui::editor::commands::EditorInputKind;

    struct FakeBridge {
        document: crate::adapters::oxfml::EditorDocument,
    }

    impl OxfmlEditorBridge for FakeBridge {
        fn apply_formula_edit(
            &self,
            request: FormulaEditRequest,
        ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
            assert_eq!(
                request.analysis_stage,
                EditorAnalysisStage::FullSemanticPlan
            );
            Ok(FormulaEditResult {
                document: self.document.clone(),
            })
        }
    }

    #[test]
    fn live_input_refreshes_active_formula_space_through_bridge() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let bridge = FakeBridge {
            document: sample_editor_document("=SUM(1,2,3)"),
        };

        let changed = apply_live_editor_input(
            &bridge,
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: Some(11),
                selection_end: Some(11),
                input_kind: EditorInputKind::InsertText,
                inserted_text: Some("3".to_string()),
            },
        )
        .expect("live edit should refresh active formula space");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert!(active.editor_document.is_some());
        assert_eq!(active.completion_help.completion_count, 1);
    }

    #[test]
    fn live_command_refreshes_after_local_command_path() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let bridge = FakeBridge {
            document: sample_editor_document("=UM(1,2)"),
        };

        let changed = apply_live_editor_command(&bridge, &mut state, EditorCommand::Delete)
            .expect("live command should refresh active formula space");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=UM(1,2)");
        assert!(active.editor_document.is_some());
    }

    #[test]
    fn live_completion_navigation_stays_local_without_bridge_refresh() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.editor_surface_state.completion_selected_index = Some(0);
        state.formula_spaces.insert(formula_space);

        let bridge = FakeBridge {
            document: sample_editor_document("=SUM(1,2)"),
        };

        let changed =
            apply_live_editor_command(&bridge, &mut state, EditorCommand::SelectNextCompletion)
                .expect("completion navigation should stay local");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(
            active.editor_surface_state.completion_selected_index,
            Some(0)
        );
    }

    /// §11.2 invariant 4: `apply_live_editor_command` skips bridge
    /// refresh for every command in the skip list. A bridge that panics
    /// on any call fails the test the moment a non-skipped command is
    /// dispatched, which makes this test a precise pin of the skip list.
    #[test]
    fn bridge_refresh_skip_list_covers_full_command_set() {
        use crate::ui::editor::state::{CompletionAggressiveness, EditorSettingUpdate};

        struct PanicBridge;
        impl OxfmlEditorBridge for PanicBridge {
            fn apply_formula_edit(
                &self,
                _request: FormulaEditRequest,
            ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
                panic!("bridge refresh should be skipped for this command");
            }
        }

        let skip_list = [
            EditorCommand::SelectPreviousCompletion,
            EditorCommand::SelectNextCompletion,
            EditorCommand::SelectCompletionByIndex(0),
            EditorCommand::CommitEntry,
            EditorCommand::RequestProof,
            EditorCommand::ForceShowCompletion,
            EditorCommand::DismissCompletion,
            EditorCommand::ToggleExpandedHeight,
            EditorCommand::SendSelectionToInspect,
            EditorCommand::ToggleEditorSettingsPopover,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::SetCompletionAggressiveness(
                CompletionAggressiveness::Always,
            )),
            EditorCommand::ToggleConfigureDrawer,
        ];

        for command in skip_list {
            let formula_space_id = FormulaSpaceId::new("space-1");
            let mut state = OneCalcHostState::default();
            state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
            let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
            formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
            formula_space.editor_surface_state.completion_selected_index = Some(0);
            state.formula_spaces.insert(formula_space);

            let _ = apply_live_editor_command(&PanicBridge, &mut state, command.clone())
                .expect("skip-listed command should not touch the bridge");
        }
    }

    #[test]
    fn live_caret_movement_refreshes_signature_help_argument_index() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_surface_state.caret.offset = 6;
        formula_space.editor_surface_state.selection =
            crate::ui::editor::state::EditorSelection::collapsed(6);
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let mut document = sample_editor_document("=SUM(1,2)");
        if let Some(signature_help) = document.signature_help.as_mut() {
            signature_help.active_argument_index = 1;
        }
        let bridge = FakeBridge { document };

        let changed = apply_live_editor_command(&bridge, &mut state, EditorCommand::MoveCaretRight)
            .expect("caret move should refresh live signature help");

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(
            active
                .editor_document
                .as_ref()
                .and_then(|document| document.signature_help.as_ref())
                .map(|help| help.active_argument_index),
            Some(1)
        );
    }
}
