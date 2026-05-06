use std::collections::BTreeMap;
use std::sync::Mutex;

use oxfml_core::consumer::editor::{
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult as UpstreamEditorInteractionResult,
};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormulaRequest, RuntimeFormulaResult,
};
use oxfml_core::format::oxfml_en_us_locale_context;
use oxfml_core::interface::{HostProviderOutcomeKind, TypedContextQueryBundle};
use oxfml_core::publication::{
    AverageRuleOptions, ColorScaleRuleOptions, ColorScaleRuleStop, ConditionalFormattingRank,
    ConditionalFormattingThreshold, ConditionalFormattingTypedRule, DataBarDirection,
    DataBarRuleOptions, IconSetRuleOptions, RankRuleOptions, VerificationConditionalFormattingRule,
    VerificationPublicationContext,
};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::{BindContext, FormulaChannelKind};

use super::bridge::{
    FormulaEditRequest, FormulaEditResult, FormulaFormattingCfDataBarDirection,
    FormulaFormattingCfRank, FormulaFormattingCfThreshold, FormulaFormattingCfTypedRule,
    FormulaFormattingRequest, OxfmlEditorBridge, OxfmlEditorBridgeError, ScenarioPolicyRequest,
};
use super::types::{
    worksheet_error_literal, ArrayCellValue, BindSummary, EditorDocument, EvalSummary, EvalValue,
    FormulaArrayPreview, FormulaValuePresentation, FormulaWalkNode, FormulaWalkNodeState,
    ParseSummary, ProvenanceSummary,
};

#[derive(Debug, Default)]
pub struct LiveOxfmlBridge {
    cached_documents: Mutex<BTreeMap<String, UpstreamEditorDocument>>,
}

// Function-list ownership lives in OxFunc. After W068 (function help)
// and HANDOFF-DNAONECALC-004 (completion proposals), every editor
// surface inside OxFml reads `oxfunc_core::registry::builtin_registry()`
// by default through `EditorEnvironment::new(...)`. The host
// authors no function list, no library snapshot, no parallel
// catalog — those would all be mirrors of the OxFunc registry and
// exactly what these worksets retired.
//
// When DNA OneCalc grows UDF support, this site will build a
// `FunctionRegistry` clone via
// `FunctionRegistry::built_ins().register_udf(...)` and pass it via
// `EditorEnvironment::with_function_registry(...)`. Capability gating
// (e.g. RTD provider unavailable in browser builds) flows through a
// `CapabilityOverlay` rather than through removing entries.

impl OxfmlEditorBridge for LiveOxfmlBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        let source = FormulaSourceRecord::new(
            request.formula_stable_id.clone(),
            1,
            request.entered_text.clone(),
        )
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

        let previous_document = self.previous_document(&request)?;
        // `EditorEnvironment::new` binds the OxFunc built-in registry
        // as the default function registry. That is the canonical
        // source for both completion proposals and function help in
        // every editor surface — no `with_library_context_provider`,
        // no `with_function_registry`, no host-supplied catalog.
        let environment = EditorEnvironment::new(BindContext::default());
        let service = EditorEditService::new(environment);
        // `apply_edit` parses + binds; the returned `EditorInteractionResult`
        // does NOT carry completion proposals / signature help / function
        // help. Run `interact_at_cursor` on the resulting document at the
        // request's cursor offset to populate those — without this the
        // popup, signature line, and hover surfaces have nothing to
        // render. The two calls share the green-tree from `apply_edit`,
        // so re-running interaction is cheap.
        let edit_result = service.apply_edit(
            source.clone(),
            previous_document.as_ref(),
            request.analysis_stage,
            None,
        );
        let interaction = service.interact_at_cursor(&edit_result.document, request.cursor_offset);
        // Pre-MVP locale: en-US, 1900 calendar. The publication-context
        // formatter inside OxFml short-circuits to "no formatted text"
        // when `locale_ctx` is `None`, so we always pass the en-US
        // context here even when the user hasn't opened the format
        // panel — that way a CURRENCY / Date / Percent format code
        // applies the moment it's typed without needing the locale UI
        // to land first. Real locale switching (multi-locale month /
        // weekday tables, currency, decimal separator) is deferred
        // behind `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md`
        // (`SEAM-OXFML-LOCALE-EXPAND`).
        let locale_ctx = oxfml_en_us_locale_context();
        let (now_serial, random_value) = scenario_seeds(request.scenario_policy);
        let typed_context = TypedContextQueryBundle::new(
            None,
            None,
            Some(&locale_ctx),
            Some(now_serial),
            Some(random_value),
        );
        let mut runtime_request = RuntimeFormulaRequest::new(source, typed_context);
        if let Some(formatting) = request.formatting_request.as_ref() {
            if let Some(context) = build_publication_context(formatting) {
                runtime_request = runtime_request.with_verification_publication_context(context);
            }
        }
        let runtime_result = RuntimeEnvironment::new().execute(runtime_request).ok();

        let document = build_editor_document(
            &request.formula_stable_id,
            &interaction,
            runtime_result.as_ref(),
        );
        self.cache_document(request.formula_stable_id, interaction.document)?;

        Ok(FormulaEditResult { document })
    }
}

impl LiveOxfmlBridge {
    fn previous_document(
        &self,
        request: &FormulaEditRequest,
    ) -> Result<Option<UpstreamEditorDocument>, OxfmlEditorBridgeError> {
        let cached_documents = self.cached_documents.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string())
        })?;
        let previous = cached_documents.get(&request.formula_stable_id).cloned();
        Ok(previous.filter(|document| {
            request.previous_green_tree_key.as_deref()
                == Some(document.editor_syntax_snapshot.green_tree_key.as_str())
        }))
    }

    fn cache_document(
        &self,
        formula_stable_id: String,
        document: UpstreamEditorDocument,
    ) -> Result<(), OxfmlEditorBridgeError> {
        let mut cached_documents = self.cached_documents.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string())
        })?;
        cached_documents.insert(formula_stable_id, document);
        Ok(())
    }
}

/// Bundle the upstream editor-interaction result and (optional) runtime
/// result into the host's `EditorDocument`. The component types
/// (`EditorSyntaxSnapshot`, `LiveDiagnosticSnapshot`, `CompletionProposal`,
/// etc.) are upstream types re-exported from `super::types`, so component
/// fields pass through by `.clone()`. The host bundle layer adds the
/// runtime-derived projections (formula_walk, parse/bind/eval/provenance
/// summaries, value_presentation).
fn build_editor_document(
    _formula_stable_id: &str,
    interaction: &UpstreamEditorInteractionResult,
    runtime_result: Option<&RuntimeFormulaResult>,
) -> EditorDocument {
    let document = &interaction.document;
    let parse_status = if document.live_diagnostics.diagnostics.is_empty() {
        "Valid".to_string()
    } else {
        "Diagnostics".to_string()
    };
    let blocked_reason = runtime_result.and_then(blocked_reason_from_runtime);

    EditorDocument {
        source_text: document.source.entered_formula_text.clone(),
        text_change_range: document.text_change_range,
        editor_syntax_snapshot: document.editor_syntax_snapshot.clone(),
        live_diagnostics: document.live_diagnostics.clone(),
        reuse_summary: document.reuse_summary.clone(),
        signature_help: interaction.signature_help_context.clone(),
        function_help: interaction.function_help_packet.clone(),
        completion_proposals: interaction
            .completion_result
            .as_ref()
            .map(|result| result.proposals.clone())
            .unwrap_or_default(),
        formula_walk: runtime_result.map(map_formula_walk).unwrap_or_else(|| {
            vec![FormulaWalkNode {
                node_id: "node:source".to_string(),
                label: "CellEntry".to_string(),
                value_preview: Some(document.source.entered_formula_text.clone()),
                state: if blocked_reason.is_some() {
                    FormulaWalkNodeState::Blocked
                } else if document.bound_formula.is_some() {
                    FormulaWalkNodeState::Evaluated
                } else {
                    FormulaWalkNodeState::Opaque
                },
                children: Vec::new(),
            }]
        }),
        parse_summary: Some(ParseSummary {
            status: parse_status,
            token_count: document.editor_syntax_snapshot.tokens.len(),
        }),
        bind_summary: Some(BindSummary {
            variable_count: usize::from(document.bound_formula.is_some()),
            reference_count: runtime_result
                .map(|result| {
                    result
                        .evaluation
                        .trace
                        .prepared_calls
                        .iter()
                        .flat_map(|call| call.prepared_arguments.iter())
                        .filter(|argument| argument.reference_target.is_some())
                        .count()
                })
                .unwrap_or(0),
        }),
        eval_summary: Some(EvalSummary {
            step_count: runtime_result
                .map(|result| result.evaluation.trace.prepared_calls.len())
                .unwrap_or_else(|| usize::from(document.semantic_plan.is_some())),
            duration_text: runtime_result
                .map(|result| {
                    format!(
                        "{} prepared call(s)",
                        result.evaluation.trace.prepared_calls.len()
                    )
                })
                .unwrap_or_else(|| "edit-only".to_string()),
        }),
        provenance_summary: Some(ProvenanceSummary {
            profile_summary: runtime_result
                .map(|result| format!("OxFml runtime · {:?}", result.returned_value_surface.kind))
                .unwrap_or_else(|| "OxFml editor".to_string()),
            blocked_reason,
        }),
        value_presentation: runtime_result.map(map_value_presentation),
    }
}

fn map_formula_walk(result: &RuntimeFormulaResult) -> Vec<FormulaWalkNode> {
    if result.evaluation.trace.prepared_calls.is_empty() {
        return vec![FormulaWalkNode {
            node_id: "node:formula".to_string(),
            label: "Formula".to_string(),
            value_preview: Some(format_walk_value_preview(&result.published_worksheet_value)),
            state: FormulaWalkNodeState::Evaluated,
            children: Vec::new(),
        }];
    }

    result
        .evaluation
        .trace
        .prepared_calls
        .iter()
        .enumerate()
        .map(|(index, call)| FormulaWalkNode {
            node_id: format!("node:prepared:{index}"),
            label: call.function_name.clone(),
            value_preview: call
                .returned_value
                .as_ref()
                .map(format_walk_value_preview)
                .or_else(|| {
                    Some(format!(
                        "args: {} · profile: {:?}",
                        call.prepared_arguments.len(),
                        call.arg_preparation_profile
                    ))
                }),
            state: FormulaWalkNodeState::Evaluated,
            children: call
                .prepared_arguments
                .iter()
                .enumerate()
                .map(|(arg_ordinal, argument)| FormulaWalkNode {
                    node_id: format!("node:prepared:{index}:arg:{arg_ordinal}"),
                    label: format!("arg[{}]", argument.ordinal),
                    value_preview: argument
                        .resolved_value
                        .as_ref()
                        .map(format_walk_value_preview)
                        .or_else(|| argument.reference_target.clone())
                        .or_else(|| Some(format!("eval={:?}", argument.evaluation_mode))),
                    state: if argument.reference_target.is_some() {
                        FormulaWalkNodeState::Bound
                    } else {
                        FormulaWalkNodeState::Evaluated
                    },
                    children: Vec::new(),
                })
                .collect(),
        })
        .collect()
}

fn map_value_presentation(result: &RuntimeFormulaResult) -> FormulaValuePresentation {
    let blocked_reason = blocked_reason_from_runtime(result);
    let array_preview = match &result.published_worksheet_value {
        EvalValue::Array(array) => {
            let shape = array.shape();
            // Result-hero array browser cap. The user's intent is
            // "don't truncate after a sliver — show the full array
            // for any practical formula, only cut off at genuinely
            // huge sizes". The cap below is 1000 rows × 100 cols =
            // 100k cells — plenty for any real formula
            // (FILTER / SEQUENCE / BYROW / RANDARRAY at sane sizes
            // are well under this). Above the cap, the truncated
            // chip stays as a safety valve for pathological cases
            // like `=RANDARRAY(1e6,1e6)` where transmitting all
            // formatted strings through the bridge would dominate
            // a keystroke. A virtualised browser is the correct
            // long-term answer for unlimited-size arrays — see
            // the WS-14 follow-on tracked under
            // `SEAM-ONECALC-ARRAY-BROWSER-VIRTUALIZATION`.
            let max_rows = shape.rows.min(1000);
            let max_cols = shape.cols.min(100);
            let mut rows = Vec::with_capacity(max_rows);
            for row in 0..max_rows {
                let cells = array
                    .row_slice(row)
                    .unwrap_or(&[])
                    .iter()
                    .take(max_cols)
                    .map(format_array_cell_value)
                    .collect::<Vec<_>>();
                rows.push(cells);
            }

            Some(FormulaArrayPreview {
                label: format!("{}x{} spill preview", shape.rows, shape.cols),
                rows,
                truncated: shape.rows > max_rows || shape.cols > max_cols,
            })
        }
        _ => None,
    };

    // Prefer the upstream-formatted effective_display_text when the
    // RuntimeFormulaResult carries a publication surface (the
    // VerificationPublicationContext we passed in turned the
    // surface on). Falls back to the host's plain
    // format_eval_value_for_display when the surface is absent or
    // its effective_display_text is empty (the default-no-context
    // path).
    let effective_display_summary = if result
        .verification_publication_surface
        .has_publication_context
        && !result
            .verification_publication_surface
            .effective_display_text
            .is_empty()
    {
        Some(
            result
                .verification_publication_surface
                .effective_display_text
                .clone(),
        )
    } else {
        Some(format_eval_value_for_display(
            &result.published_worksheet_value,
            array_preview.as_ref(),
        ))
    };

    let number_format_hint = result
        .returned_value_surface
        .presentation_hint
        .and_then(|hint| hint.number_format)
        .map(map_number_format_hint);

    // Lift CF-applied colours from the publication surface. OxFml's
    // `evaluate_conditional_formatting_rule` already populates these
    // — when a CF rule fires, `effective_font_color` /
    // `effective_fill_color` carry the rule's overrides. Outside a
    // rule fire (or when no rules exist), both are `None` and the
    // host renders in default chrome. Future colour-token publication
    // will route through the same fields without host churn.
    let effective_font_color = result
        .verification_publication_surface
        .effective_font_color
        .clone();
    let effective_fill_color = result
        .verification_publication_surface
        .effective_fill_color
        .clone();
    let array_cell_format = result
        .verification_publication_surface
        .array_cell_format
        .as_ref()
        .map(map_array_cell_format_grid);

    FormulaValuePresentation {
        evaluation_summary: evaluation_summary_from_value(&result.published_worksheet_value),
        effective_display_summary,
        array_preview,
        blocked_reason,
        published_value: result.published_worksheet_value.clone(),
        number_format_hint,
        effective_font_color,
        effective_fill_color,
        array_cell_format,
    }
}

/// Mirror the upstream per-cell CF outcome grid into the host's
/// adapter-level shape. Pure 1:1 mapping; the host carries its
/// own copies of the cell-level structs so view-models and tests
/// don't take a transitive dependency on `oxfml_core::publication`.
fn map_array_cell_format_grid(
    grid: &oxfml_core::publication::ArrayCellFormatGrid,
) -> super::types::ArrayCellFormatGrid {
    super::types::ArrayCellFormatGrid {
        rows: grid
            .rows
            .iter()
            .map(|row| row.iter().map(map_array_cell_format).collect())
            .collect(),
    }
}

fn map_array_cell_format(
    cell: &oxfml_core::publication::ArrayCellFormat,
) -> super::types::ArrayCellFormat {
    super::types::ArrayCellFormat {
        effective_display_text: cell.effective_display_text.clone(),
        effective_font_color: cell.effective_font_color.clone(),
        effective_fill_color: cell.effective_fill_color.clone(),
        data_bar: cell.data_bar.as_ref().map(map_data_bar_fill),
        icon: cell.icon.as_ref().map(map_cf_icon),
    }
}

fn map_data_bar_fill(fill: &oxfml_core::publication::DataBarFill) -> super::types::DataBarFill {
    super::types::DataBarFill {
        fill_ratio: fill.fill_ratio,
        bar_color: fill.bar_color.clone(),
        direction: match fill.direction {
            oxfml_core::publication::DataBarDirection::Left => super::types::DataBarDirection::Left,
            oxfml_core::publication::DataBarDirection::Right => {
                super::types::DataBarDirection::Right
            }
        },
        show_bar_only: fill.show_bar_only,
    }
}

fn map_cf_icon(icon: &oxfml_core::publication::CfIcon) -> super::types::CfIcon {
    super::types::CfIcon {
        set_kind: icon.set_kind.clone(),
        icon_index: icon.icon_index,
    }
}

/// Translate the upstream `oxfunc_core::value::NumberFormatHint`
/// (re-exported from `oxfunc_value_types`) to the host's mirrored
/// enum. Kept as a 1:1 mapping; if upstream grows a new variant the
/// compiler flags this site.
fn map_number_format_hint(
    hint: oxfunc_core::value::NumberFormatHint,
) -> super::types::NumberFormatHint {
    use super::types::NumberFormatHint as Host;
    use oxfunc_core::value::NumberFormatHint as Up;
    match hint {
        Up::General => Host::General,
        Up::DateLike => Host::DateLike,
        Up::Percentage => Host::Percentage,
        Up::Currency => Host::Currency,
        Up::Scientific => Host::Scientific,
        Up::Fraction => Host::Fraction,
        Up::Custom => Host::Custom,
    }
}

fn blocked_reason_from_runtime(result: &RuntimeFormulaResult) -> Option<String> {
    result
        .returned_value_surface
        .host_provider_outcome
        .as_ref()
        .and_then(|outcome| match outcome.outcome_kind {
            HostProviderOutcomeKind::CapabilityDenied => Some(
                outcome
                    .detail
                    .clone()
                    .unwrap_or_else(|| "Host capability denied".to_string()),
            ),
            _ => None,
        })
}

/// Produce the host's compact `evaluation_summary` string directly from
/// the upstream typed `EvalValue`. Replaces an earlier helper that
/// re-parsed `RuntimeFormulaResult.evaluation.result.payload_summary`
/// with `strip_prefix("Number(")` etc., which silently lost the typed
/// discriminator and forced a downstream string round-trip to recover it.
fn evaluation_summary_from_value(value: &EvalValue) -> String {
    match value {
        EvalValue::Number(number) => format!("Number · {}", format_number(*number)),
        EvalValue::Text(text) => format!("Text · {}", text.to_string_lossy()),
        EvalValue::Logical(true) => "Logical · TRUE".to_string(),
        EvalValue::Logical(false) => "Logical · FALSE".to_string(),
        EvalValue::Error(code) => format!("Error · {}", worksheet_error_literal(*code)),
        EvalValue::Array(array) => {
            let shape = array.shape();
            format!("Array · {}x{} dynamic result", shape.rows, shape.cols)
        }
        EvalValue::Reference(reference) => format!("Reference · {}", reference.target),
        EvalValue::Lambda(lambda) => format!("Lambda · {}", lambda.callable_token),
    }
}

/// Map the host's `FormulaFormattingRequest` into the upstream
/// `VerificationPublicationContext` shape. Returns `None` when none
/// of the formatting fields are populated (so we skip the publication-
/// context lane and OxFml falls back to the default visible-value
/// rendering).
fn build_publication_context(
    formatting: &FormulaFormattingRequest,
) -> Option<VerificationPublicationContext> {
    let any = formatting
        .number_format_code
        .as_deref()
        .is_some_and(|s| !s.is_empty())
        || formatting
            .font_color
            .as_deref()
            .is_some_and(|s| !s.is_empty())
        || formatting
            .fill_color
            .as_deref()
            .is_some_and(|s| !s.is_empty())
        || formatting
            .style_id
            .as_deref()
            .is_some_and(|s| !s.is_empty())
        || formatting.date1904
        || !formatting.conditional_formatting_rules.is_empty();
    if !any {
        return None;
    }
    let conditional_formatting_rules: Vec<VerificationConditionalFormattingRule> = formatting
        .conditional_formatting_rules
        .iter()
        .map(|rule| VerificationConditionalFormattingRule {
            target_ranges: Vec::new(),
            rule_kind: rule.rule_kind.clone(),
            operator: rule.operator.clone(),
            thresholds: rule.thresholds.clone(),
            typed_rule: rule.typed_rule.as_ref().map(bridge_typed_rule_to_upstream),
            font_color: rule.font_color.clone(),
            fill_color: rule.fill_color.clone(),
            effective_display_text: None,
            applies: None,
            effective_font_color: None,
            effective_fill_color: None,
        })
        .collect();
    Some(VerificationPublicationContext {
        format_profile: None,
        number_format_code: formatting
            .number_format_code
            .clone()
            .filter(|value| !value.is_empty()),
        style_id: formatting
            .style_id
            .clone()
            .filter(|value| !value.is_empty()),
        style_hierarchy: Vec::new(),
        font_color: formatting
            .font_color
            .clone()
            .filter(|value| !value.is_empty()),
        fill_color: formatting
            .fill_color
            .clone()
            .filter(|value| !value.is_empty()),
        conditional_formatting_rules,
    })
}

/// Map the host bridge's typed CF rule shape to the upstream
/// `oxfml_core::publication::ConditionalFormattingTypedRule`. Only
/// the populated sub-options are forwarded; OxFml W073 reads the
/// typed rule preferentially when set and falls back to the bounded-
/// string `thresholds` convention otherwise.
fn bridge_typed_rule_to_upstream(
    rule: &FormulaFormattingCfTypedRule,
) -> ConditionalFormattingTypedRule {
    ConditionalFormattingTypedRule {
        color_scale: rule
            .color_scale
            .as_ref()
            .map(|options| ColorScaleRuleOptions {
                stops: options
                    .stops
                    .iter()
                    .map(|stop| ColorScaleRuleStop {
                        position: bridge_threshold_to_upstream(&stop.position),
                        color: stop.color.clone(),
                    })
                    .collect(),
            }),
        data_bar: rule.data_bar.as_ref().map(|options| DataBarRuleOptions {
            minimum: options.minimum.as_ref().map(bridge_threshold_to_upstream),
            maximum: options.maximum.as_ref().map(bridge_threshold_to_upstream),
            bar_color: options.bar_color.clone(),
            direction: options.direction.map(|direction| match direction {
                FormulaFormattingCfDataBarDirection::Left => DataBarDirection::Left,
                FormulaFormattingCfDataBarDirection::Right => DataBarDirection::Right,
            }),
            show_bar_only: options.show_bar_only,
        }),
        icon_set: rule.icon_set.as_ref().map(|options| IconSetRuleOptions {
            set_kind: options.set_kind.clone(),
            thresholds: options
                .thresholds
                .iter()
                .map(bridge_threshold_to_upstream)
                .collect(),
        }),
        rank: rule.rank.as_ref().map(|options| RankRuleOptions {
            rank: match &options.rank {
                FormulaFormattingCfRank::Count(count) => ConditionalFormattingRank::Count(*count),
                FormulaFormattingCfRank::Percent(value) => {
                    ConditionalFormattingRank::Percent(*value)
                }
            },
        }),
        average: rule.average.as_ref().map(|options| AverageRuleOptions {
            include_equal: options.include_equal,
            stddev_multiplier: options.stddev_multiplier,
        }),
    }
}

fn bridge_threshold_to_upstream(
    threshold: &FormulaFormattingCfThreshold,
) -> ConditionalFormattingThreshold {
    match threshold {
        FormulaFormattingCfThreshold::Min => ConditionalFormattingThreshold::Min,
        FormulaFormattingCfThreshold::Mid => ConditionalFormattingThreshold::Mid,
        FormulaFormattingCfThreshold::Max => ConditionalFormattingThreshold::Max,
        FormulaFormattingCfThreshold::Percent(value) => {
            ConditionalFormattingThreshold::Percent(*value)
        }
        FormulaFormattingCfThreshold::Percentile(value) => {
            ConditionalFormattingThreshold::Percentile(*value)
        }
        FormulaFormattingCfThreshold::Number(value) => {
            ConditionalFormattingThreshold::Number(*value)
        }
    }
}

/// Derive `(now_serial, random_value)` for the runtime context bundle
/// from the active formula's calc-options policy.
///
/// **Deterministic** mode pins both values so a formula re-runs
/// identically on every keystroke. The constants match OxFml's host
/// defaults (`now_serial = 46000.0` ≈ 2025-12-09, `random_value =
/// 0.5`) so the host's deterministic mode reproduces what users
/// already see in OxFml's own test fixtures.
///
/// **LiveRecalc** mode reads a fresh `now_serial` from the platform
/// clock and a fresh `random_value` from the platform RNG on every
/// bridge round-trip. `=NOW()` advances per keystroke and `=RAND()`
/// rolls a new value per round-trip.
fn scenario_seeds(policy: ScenarioPolicyRequest) -> (f64, f64) {
    match policy {
        ScenarioPolicyRequest::Deterministic => (46000.0, 0.5),
        ScenarioPolicyRequest::LiveRecalc => (current_excel_serial(), current_random_value()),
    }
}

#[cfg(target_arch = "wasm32")]
fn current_excel_serial() -> f64 {
    // Excel `=NOW()` reports the user's *local* wall-clock time, not
    // UTC. `js_sys::Date::now()` is UTC milliseconds since Unix epoch;
    // we subtract the timezone offset (in minutes, sign-flipped per
    // the JS Date API: UTC+1 returns `-60`) to get local milliseconds.
    let utc_ms = js_sys::Date::now();
    let tz_offset_minutes = js_sys::Date::new_0().get_timezone_offset();
    let local_ms = utc_ms - tz_offset_minutes * 60_000.0;
    // Excel serial 25569 = 1970-01-01 (Unix epoch under the
    // 1900-leap-year-bug calendar; the host always passes the en-US
    // 1900 system today).
    local_ms / 86_400_000.0 + 25_569.0
}

#[cfg(target_arch = "wasm32")]
fn current_random_value() -> f64 {
    js_sys::Math::random()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_excel_serial() -> f64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_secs_f64() / 86_400.0 + 25_569.0
}

#[cfg(not(target_arch = "wasm32"))]
fn current_random_value() -> f64 {
    // Lightweight non-wasm RNG: derive a u64 from the current
    // nanosecond clock and run it through a small splittable
    // mixer. Sufficient for a `=RAND()` seed; SSR / desktop builds
    // are not running cryptographic RNGs out of this site.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0);
    let mut z = nanos.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    // Map to [0, 1) the same way the JavaScript Math.random
    // contract does — use the upper 53 bits.
    (z >> 11) as f64 / ((1u64 << 53) as f64)
}

/// Format a value preview for the formula walk-tree drill-down.
/// Single-cell scalars use the same formatting as the result hero
/// (number / text / TRUE / FALSE / #ERR). Arrays render as
/// `Array[rows×cols] {a, b, c, …}` so the drill stays compact even
/// when the formula returns thousands of cells. The first six cells
/// in row-major order are shown; "…" when the array has more.
fn format_walk_value_preview(value: &EvalValue) -> String {
    match value {
        EvalValue::Array(array) => {
            let shape = array.shape();
            let total = shape.rows.saturating_mul(shape.cols);
            let preview_cap = 6usize;
            let mut cells: Vec<String> = Vec::with_capacity(preview_cap);
            'outer: for row in 0..shape.rows {
                let row_slice = array.row_slice(row).unwrap_or(&[]);
                for cell in row_slice {
                    if cells.len() >= preview_cap {
                        break 'outer;
                    }
                    cells.push(format_array_cell_value(cell));
                }
            }
            let truncated = total > cells.len();
            let body = cells.join(", ");
            if truncated {
                format!("Array[{}×{}] {{{}, …}}", shape.rows, shape.cols, body)
            } else {
                format!("Array[{}×{}] {{{}}}", shape.rows, shape.cols, body)
            }
        }
        _ => format_eval_value_for_display(value, None),
    }
}

fn format_eval_value_for_display(
    value: &EvalValue,
    array_preview: Option<&FormulaArrayPreview>,
) -> String {
    match value {
        EvalValue::Number(number) => format_number(*number),
        EvalValue::Text(text) => text.to_string_lossy(),
        EvalValue::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        EvalValue::Error(code) => worksheet_error_literal(*code).to_string(),
        EvalValue::Array(_) => array_preview
            .map(|preview| {
                format!(
                    "{{{}}}",
                    preview
                        .rows
                        .iter()
                        .map(|row| row.join(","))
                        .collect::<Vec<_>>()
                        .join(";")
                )
            })
            .unwrap_or_else(|| "Array result".to_string()),
        EvalValue::Reference(reference) => reference.target.clone(),
        EvalValue::Lambda(lambda) => format!("Lambda({})", lambda.callable_token),
    }
}

fn format_array_cell_value(cell: &ArrayCellValue) -> String {
    match cell {
        ArrayCellValue::Number(number) => format_number(*number),
        ArrayCellValue::Text(text) => text.to_string_lossy(),
        ArrayCellValue::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        ArrayCellValue::Error(code) => worksheet_error_literal(*code).to_string(),
        ArrayCellValue::EmptyCell => String::new(),
    }
}

fn format_number(number: f64) -> String {
    if number == number.trunc() && number.abs() < 1e16 {
        format!("{number:.0}")
    } else {
        format!("{number}")
    }
}
