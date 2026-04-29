use std::collections::BTreeMap;
use std::sync::Mutex;

use oxfml_core::consumer::editor::{
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult as UpstreamEditorInteractionResult,
};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormulaRequest, RuntimeFormulaResult,
};
use oxfml_core::interface::{
    HostProviderOutcomeKind, InMemoryLibraryContextProvider, TypedContextQueryBundle,
};
use oxfml_core::semantics::{
    LibraryAvailabilityState, LibraryContextSnapshot, LibraryContextSnapshotEntry,
    RegistrationSourceKind,
};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::{BindContext, FormulaChannelKind};

use super::bridge::{
    FormulaEditRequest, FormulaEditResult, OxfmlEditorBridge, OxfmlEditorBridgeError,
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

/// Curated list of common Excel function names the home shell seeds
/// into its library context. Without a populated library context the
/// upstream `collect_completion_proposals` returns an empty list, so
/// completion popups, signature help, and function help would never
/// fire. The list is deliberately short and uncontroversial; richer
/// admission lifecycles (FeatureGated / CompatibilityGated, version
/// metadata, locale-specific names) are SEAM-pending for the eventual
/// "real catalog from OxFunc" plumbing — see
/// `SEAM-ONECALC-LIBRARY-CONTEXT-FROM-OXFUNC-CATALOG`.
const DEFAULT_FUNCTION_NAMES: &[&str] = &[
    "ABS", "AND", "AVERAGE", "AVERAGEIF", "CEILING", "CHOOSE", "CONCAT", "CONCATENATE", "COUNT",
    "COUNTA", "COUNTIF", "COUNTIFS", "DATE", "DAY", "FILTER", "FLOOR", "HOUR", "IF", "IFERROR",
    "IFNA", "IFS", "INDEX", "INDIRECT", "ISBLANK", "ISERROR", "ISNUMBER", "ISTEXT", "LEFT", "LEN",
    "LET", "LOOKUP", "LOWER", "MATCH", "MAX", "MID", "MIN", "MINUTE", "MOD", "MONTH", "NOT", "NOW",
    "OR", "RIGHT", "ROUND", "ROUNDDOWN", "ROUNDUP", "SEARCH", "SEQUENCE", "SORT", "SQRT", "SUBSTITUTE",
    "SUM", "SUMIF", "SUMIFS", "SUMPRODUCT", "TEXT", "TIME", "TODAY", "TRIM", "TRUE", "UNIQUE",
    "UPPER", "VALUE", "VLOOKUP", "XLOOKUP", "XMATCH", "YEAR",
];

/// Build a default library snapshot from [`DEFAULT_FUNCTION_NAMES`].
/// Each entry is registered as a `BuiltIn` with `CatalogKnown`
/// availability so the editor proposal collector treats it as a
/// reachable function name.
fn default_function_library_snapshot() -> LibraryContextSnapshot {
    LibraryContextSnapshot {
        snapshot_id: "dnaonecalc.default-functions".to_string(),
        snapshot_version: "v1".to_string(),
        entries: DEFAULT_FUNCTION_NAMES
            .iter()
            .map(|name| LibraryContextSnapshotEntry {
                surface_name: name.to_string(),
                canonical_id: Some(format!("FUNC.{name}")),
                surface_stable_id: Some(format!("surface.{}", name.to_ascii_lowercase())),
                name_resolution_table_ref: Some("name-table:default".to_string()),
                semantic_trait_profile_ref: Some(format!("trait:{}", name.to_ascii_lowercase())),
                gating_profile_ref: Some("gate:default".to_string()),
                metadata_status: Some("stable".to_string()),
                special_interface_kind: None,
                admission_interface_kind: Some("ordinary".to_string()),
                preparation_owner: Some("OxFunc".to_string()),
                runtime_boundary_kind: Some("ordinary".to_string()),
                arity_shape_note: Some("variadic".to_string()),
                interface_contract_ref: Some(format!("contract:{}", name.to_ascii_lowercase())),
                registration_source_kind: RegistrationSourceKind::BuiltIn,
                parse_bind_state: LibraryAvailabilityState::CatalogKnown,
                semantic_plan_state: LibraryAvailabilityState::CatalogKnown,
                runtime_capability_state: Some(LibraryAvailabilityState::CatalogKnown),
                post_dispatch_state: Some(LibraryAvailabilityState::CatalogKnown),
            })
            .collect(),
    }
}

fn default_library_context_provider() -> InMemoryLibraryContextProvider {
    InMemoryLibraryContextProvider::new(default_function_library_snapshot())
}

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
        let library_provider = default_library_context_provider();
        let environment = EditorEnvironment::new(BindContext::default())
            .with_library_context_provider(&library_provider);
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
        let runtime_result = RuntimeEnvironment::new()
            .execute(RuntimeFormulaRequest::new(
                source,
                TypedContextQueryBundle::default(),
            ))
            .ok();

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
            value_preview: Some(format_eval_value_for_display(
                &result.published_worksheet_value,
                None,
            )),
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
            label: format!("Prepared call: {}", call.function_name),
            value_preview: Some(format!(
                "args: {} · profile: {:?}",
                call.prepared_arguments.len(),
                call.arg_preparation_profile
            )),
            state: FormulaWalkNodeState::Evaluated,
            children: call
                .prepared_arguments
                .iter()
                .enumerate()
                .map(|(arg_ordinal, argument)| FormulaWalkNode {
                    node_id: format!("node:prepared:{index}:arg:{arg_ordinal}"),
                    label: format!("arg[{}]", argument.ordinal),
                    value_preview: argument
                        .reference_target
                        .clone()
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
            let max_rows = shape.rows.min(4);
            let max_cols = shape.cols.min(4);
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

    FormulaValuePresentation {
        evaluation_summary: evaluation_summary_from_value(&result.published_worksheet_value),
        effective_display_summary: Some(format_eval_value_for_display(
            &result.published_worksheet_value,
            array_preview.as_ref(),
        )),
        array_preview,
        blocked_reason,
        published_value: result.published_worksheet_value.clone(),
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
