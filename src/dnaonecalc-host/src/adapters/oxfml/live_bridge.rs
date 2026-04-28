use std::collections::BTreeMap;
use std::sync::Mutex;

use oxfml_core::consumer::editor::{
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult as UpstreamEditorInteractionResult,
};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormulaRequest, RuntimeFormulaResult,
};
use oxfml_core::interface::{HostProviderOutcomeKind, TypedContextQueryBundle};
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
        let service = EditorEditService::new(EditorEnvironment::new(BindContext::default()));
        let interaction = service.apply_edit(
            source.clone(),
            previous_document.as_ref(),
            request.analysis_stage,
            None,
        );
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
        evaluation_summary: format_eval_summary(&result.evaluation.result.payload_summary),
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

fn format_eval_summary(payload_summary: &str) -> String {
    if let Some(inner) = payload_summary
        .strip_prefix("Number(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Number · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Text(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Text · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Logical(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Logical · {inner}");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Array(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Array · {inner} dynamic result");
    }
    if let Some(inner) = payload_summary
        .strip_prefix("Error(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Error · {}", worksheet_error_literal_from_name(inner));
    }
    payload_summary.to_string()
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

fn worksheet_error_literal_from_name(name: &str) -> String {
    use oxfunc_core::value::WorksheetErrorCode as Code;
    let code = match name {
        "Null" => Code::Null,
        "Div0" => Code::Div0,
        "Value" => Code::Value,
        "Ref" => Code::Ref,
        "Name" => Code::Name,
        "Num" => Code::Num,
        "NA" => Code::NA,
        "Busy" => Code::Busy,
        "GettingData" => Code::GettingData,
        "Spill" => Code::Spill,
        "Calc" => Code::Calc,
        "Field" => Code::Field,
        "Blocked" => Code::Blocked,
        "Connect" => Code::Connect,
        _ => return name.to_string(),
    };
    worksheet_error_literal(code).to_string()
}

fn format_number(number: f64) -> String {
    if number == number.trunc() && number.abs() < 1e16 {
        format!("{number:.0}")
    } else {
        format!("{number}")
    }
}
