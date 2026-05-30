use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::Mutex;

use oxfml_core::consumer::editor::{
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult as UpstreamEditorInteractionResult,
};
use oxfml_core::consumer::runtime::{
    FormulaDrillEvaluationState, FormulaDrillTrace, FormulaDrillTraceNode, RuntimeEnvironment,
    RuntimeFormalInputBinding, RuntimeFormulaRequest, RuntimeFormulaResult,
};
use oxfml_core::format::{oxfml_en_us_locale_context, oxfml_locale_context};
use oxfml_core::interface::{HostProviderOutcomeKind, TypedContextQueryBundle};
use oxfml_core::publication::{
    AverageRuleOptions, ColorScaleRuleOptions, ColorScaleRuleStop, ConditionalFormattingRank,
    ConditionalFormattingThreshold, ConditionalFormattingTypedRule, DataBarDirection,
    DataBarRuleOptions, IconSetRuleOptions, RankRuleOptions, VerificationConditionalFormattingRule,
    VerificationPublicationContext,
};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::{BindContext, FormulaChannelKind};
use oxfunc_core::functions::rand_fn::RandomProvider;
use oxfunc_core::locale_format::{format_profile, LocaleProfileId, WorkbookDateSystem};

use crate::services::vba_host::VbaHostRuntime;

use super::bridge::{
    FormulaEditRequest, FormulaEditResult, FormulaFormattingCfDataBarDirection,
    FormulaFormattingCfRank, FormulaFormattingCfThreshold, FormulaFormattingCfTypedRule,
    FormulaFormattingRequest, FormulaInputBindingRequest, OxfmlEditorBridge,
    OxfmlEditorBridgeError, RecalcModeRequest, ScenarioPolicyRequest,
};
use super::types::{
    worksheet_error_literal, ArrayCellValue, BindSummary, EditorDocument, EvalSummary, EvalValue,
    FormulaArrayPreview, FormulaValuePresentation, FormulaWalkNode, FormulaWalkNodeState,
    ParseSummary, ProvenanceSummary,
};

// Thread-local storage for the active VBA runtime. The OxVba
// `PreparedVbaProject` contains raw pointers (`Variant.ptr_val`)
// that make it `!Send`, so we cannot store it inside `Mutex` or
// `Arc`. Thread-local is sound because all UI + evaluation calls
// run on the main thread (WASM is single-threaded; native Leptos
// dispatches on the spawning thread).
thread_local! {
    static VBA_RUNTIME_SLOT: std::cell::RefCell<Option<VbaHostRuntime>> =
        const { std::cell::RefCell::new(None) };
}

#[derive(Debug, Default)]
pub struct LiveOxfmlBridge {
    cached_documents: Mutex<BTreeMap<String, UpstreamEditorDocument>>,
    /// Per-formula cache of the most recent fully-computed
    /// `EditorDocument` keyed by the request's
    /// `(entered_text, cursor_offset, formatting_request,
    /// scenario_policy, recalc_mode, skip_runtime_evaluation)`
    /// tuple. When the next request arrives with an identical
    /// fingerprint AND the runtime-evaluation pass would not
    /// change anything (volatile-free formulas, deterministic
    /// seeds, or skip-runtime mode), the bridge returns the
    /// cached document without re-running parse / bind / popups
    /// / runtime. Skipped automatically for `LiveRecalc` since
    /// volatile functions advance per round-trip.
    last_result: Mutex<BTreeMap<String, CachedFormulaEditResult>>,
}

#[derive(Debug, Clone)]
struct CachedFormulaEditResult {
    fingerprint: FormulaEditFingerprint,
    document: EditorDocument,
}

/// Hashable fingerprint of a `FormulaEditRequest` minus the
/// fields that don't affect the output (`formula_stable_id` is
/// the cache key; `previous_green_tree_key` is an upstream
/// optimisation hint, not part of the result).
#[derive(Debug, Clone, PartialEq, Eq)]
struct FormulaEditFingerprint {
    entered_text: String,
    cursor_offset: usize,
    analysis_stage: super::types::EditorAnalysisStage,
    formatting_request: Option<FormulaFormattingRequest>,
    scenario_policy: ScenarioPolicyRequest,
    skip_runtime_evaluation: bool,
    recalc_mode: RecalcModeRequest,
    formal_input_bindings: Vec<String>,
    // Trace mode is part of the cache key — a `ValueOnly` cached
    // pass cannot satisfy a subsequent `PreparedCalls` request
    // (the rich walk tree requires the per-step trace, which the
    // cheap pass didn't produce).
    trace_mode: super::bridge::TraceModeRequest,
}

impl FormulaEditFingerprint {
    fn from_request(request: &FormulaEditRequest) -> Self {
        Self {
            entered_text: request.entered_text.clone(),
            cursor_offset: request.cursor_offset,
            analysis_stage: request.analysis_stage,
            formatting_request: request.formatting_request.clone(),
            scenario_policy: request.scenario_policy,
            skip_runtime_evaluation: request.skip_runtime_evaluation,
            recalc_mode: request.recalc_mode,
            formal_input_bindings: request
                .formal_input_bindings
                .iter()
                .map(formal_input_binding_fingerprint)
                .collect(),
            trace_mode: request.trace_mode,
        }
    }

    /// Whether a cached result with the same fingerprint can be
    /// returned without re-running the bridge. `LiveRecalc` runs
    /// volatile functions per round-trip, so a cached value would
    /// be wrong (the user expects `=NOW()` to advance every time).
    /// `Deterministic` and `ManualRecalc` use pinned / on-demand
    /// seeds and are safe to cache.
    fn is_cacheable(&self) -> bool {
        match self.scenario_policy {
            ScenarioPolicyRequest::LiveRecalc => false,
            ScenarioPolicyRequest::Deterministic | ScenarioPolicyRequest::ManualRecalc => true,
        }
    }
}

// Function-list ownership lives in OxFunc. After W068 (function help)
// and HANDOFF-DNAONECALC-004 (completion proposals), every editor
// surface inside OxFml reads `oxfunc_core::registry::builtin_registry()`
// by default through `EditorEnvironment::new(...)`. The host
// authors no function list, no library snapshot, no parallel
// catalog — those would all be mirrors of the OxFunc registry and
// exactly what these worksets retired.
//
// VBA UDFs are wired via `LibraryContextProvider` /
// `HostFunctionProvider` on the installed `VbaHostRuntime`. The
// editor sees them during bind (library context supplies the
// function surface), and the runtime invokes them via the host
// function provider. Capability gating (e.g. RTD provider
// unavailable in browser builds) flows through a
// `CapabilityOverlay` rather than through removing entries.

impl OxfmlEditorBridge for LiveOxfmlBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        let fingerprint = FormulaEditFingerprint::from_request(&request);

        // Input-equality short-circuit. When the request is identical
        // to the most recent successful one for this formula AND the
        // scenario policy doesn't depend on per-call randomness /
        // wall-clock seeds, return the cached document. This is the
        // fastest possible path — no parse, no bind, no popup
        // computation, no runtime — for the case the user hits often:
        // a redundant on:input firing for the same state, the same
        // event flowing through twice via formatting-knob refresh
        // chains, etc. `LiveRecalc` is excluded because `=NOW()` /
        // `=RAND()` should advance per round-trip.
        if fingerprint.is_cacheable() {
            if let Some(cached) = self.cached_result(&request.formula_stable_id, &fingerprint)? {
                return Ok(FormulaEditResult { document: cached });
            }
        }

        let source = FormulaSourceRecord::new(
            request.formula_stable_id.clone(),
            1,
            request.entered_text.clone(),
        )
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

        let previous_document = self.previous_document(&request)?;

        // The VBA runtime lives in a thread-local (`VBA_RUNTIME_SLOT`)
        // because OxVba's `PreparedVbaProject` contains raw pointers
        // that make it `!Send`. All evaluation runs on the UI thread,
        // so thread-local access is safe and avoids cross-thread
        // concerns. We borrow it for the full evaluation scope below.
        VBA_RUNTIME_SLOT.with(|slot| {
            let vba_guard = slot.borrow();
            let vba_runtime_ref = vba_guard.as_ref();
            self.apply_formula_edit_with_vba(
                request,
                fingerprint,
                source,
                previous_document,
                vba_runtime_ref,
            )
        })
    }
}

impl LiveOxfmlBridge {
    fn apply_formula_edit_with_vba(
        &self,
        request: FormulaEditRequest,
        fingerprint: FormulaEditFingerprint,
        source: FormulaSourceRecord,
        previous_document: Option<UpstreamEditorDocument>,
        vba_runtime_ref: Option<&VbaHostRuntime>,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        let environment = if let Some(runtime) = vba_runtime_ref {
            EditorEnvironment::new(BindContext::default()).with_library_context_provider(runtime)
        } else {
            EditorEnvironment::new(BindContext::default())
        };
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

        // Decide whether to run the runtime-evaluation pass. The
        // runtime pass is the dominant cost on a heavy formula
        // (`HANDOFF_OXFUNC_REDUCE_HOTLOOP_PERF.md`,
        // `HANDOFF_OXFML_LAMBDA_INVOCATION_PERF.md`); skipping it
        // is the host's lever for keeping the editor responsive.
        // - `skip_runtime_evaluation` is set per-event by the host
        //   (caret-only navigation skips, text-input doesn't).
        // - `RecalcModeRequest::Manual` skips on every event — only
        //   an explicit Calculate / F9 surfaces a runtime pass.
        let should_run_runtime = !request.skip_runtime_evaluation
            && !matches!(request.recalc_mode, RecalcModeRequest::Manual);

        let runtime_result = if should_run_runtime {
            // Resolve the runtime locale from the workspace's
            // BCP-47 language tag (W094 OxFunc + OxFml landed
            // 2026-05-06). An unset / unknown tag falls back to
            // en-US so the runtime always has a usable locale.
            // The `date1904` flag flows through the formatting
            // request — when the user toggles 1904 dates in the
            // formatting panel, the locale context picks up the
            // matching `WorkbookDateSystem`.
            let locale_ctx = build_runtime_locale_context(
                &request.language_tag,
                request
                    .formatting_request
                    .as_ref()
                    .map(|formatting| formatting.date1904)
                    .unwrap_or(false),
            );
            let (now_serial, random_provider) = scenario_runtime_context(request.scenario_policy);
            let typed_context = TypedContextQueryBundle::new(
                None,
                None,
                Some(&locale_ctx),
                Some(now_serial),
                Some(random_provider.as_ref()),
            )
            .with_host_function_provider(
                vba_runtime_ref.map(|r| r as &dyn oxfml_core::interface::HostFunctionProvider),
            );
            let mut runtime_request = RuntimeFormulaRequest::new(source.clone(), typed_context);
            if let Some(formatting) = request.formatting_request.as_ref() {
                if let Some(context) = build_publication_context(formatting) {
                    runtime_request =
                        runtime_request.with_verification_publication_context(context);
                }
            }
            // Trace mode: opt into per-prepared-call tracing only
            // when the caller asks (the formula-drill panel needs
            // it to render the walk tree). The cheap default (the
            // upstream's `EvaluationTraceMode::ValueOnly` since
            // OxFml W075) skips per-step bookkeeping.
            let upstream_trace_mode = match request.trace_mode {
                super::bridge::TraceModeRequest::ValueOnly => {
                    oxfml_core::EvaluationTraceMode::ValueOnly
                }
                super::bridge::TraceModeRequest::PreparedCalls => {
                    oxfml_core::EvaluationTraceMode::PreparedCalls
                }
            };
            runtime_request = runtime_request.with_trace_mode(upstream_trace_mode);
            let mut runtime_env = RuntimeEnvironment::new().with_formal_input_bindings(
                runtime_formal_input_bindings(&request.formal_input_bindings),
            );
            if let Some(runtime) = vba_runtime_ref {
                runtime_env = runtime_env.with_library_context_provider(runtime);
            }
            runtime_env.execute(runtime_request).ok()
        } else {
            None
        };

        let projection_drill_trace = if runtime_result.is_none() {
            Some(RuntimeEnvironment::new().formula_drill_trace_for_source(source.clone()))
        } else {
            None
        };

        let document = build_editor_document(
            &request.formula_stable_id,
            &interaction,
            runtime_result.as_ref(),
            projection_drill_trace.as_ref(),
        );
        if fingerprint.is_cacheable() {
            self.cache_result(
                request.formula_stable_id.clone(),
                fingerprint,
                document.clone(),
            )?;
        }
        self.cache_document(request.formula_stable_id, interaction.document)?;

        Ok(FormulaEditResult { document })
    }
}

fn formal_input_binding_fingerprint(binding: &FormulaInputBindingRequest) -> String {
    format!(
        "{}|{}|{}|{}",
        binding.label,
        binding.reference_descriptor,
        binding.reference_handle.as_deref().unwrap_or(""),
        eval_value_fingerprint(&binding.value)
    )
}

fn eval_value_fingerprint(value: &EvalValue) -> String {
    format!("{value:?}")
}

fn runtime_formal_input_bindings(
    bindings: &[FormulaInputBindingRequest],
) -> Vec<RuntimeFormalInputBinding> {
    bindings
        .iter()
        .map(|binding| RuntimeFormalInputBinding {
            reference_handle: binding.reference_handle.clone(),
            reference_descriptor: binding.reference_descriptor.clone(),
            binding: oxfml_core::DefinedNameBinding::Value(binding.value.clone()),
        })
        .collect()
}

impl LiveOxfmlBridge {
    /// Install a compiled VBA runtime for UDF resolution and invocation.
    /// Replaces any previously installed runtime. Invalidates all cached
    /// binding state since the function surface has changed.
    pub fn install_vba_runtime(&self, runtime: VbaHostRuntime) {
        VBA_RUNTIME_SLOT.with(|slot| {
            *slot.borrow_mut() = Some(runtime);
        });
        // Invalidate caches — the function surface just changed.
        let _ = self.invalidate_all_binding_state();
    }

    /// Remove the installed VBA runtime (if any). The bridge reverts to
    /// built-in-only function resolution.
    pub fn clear_vba_runtime(&self) {
        VBA_RUNTIME_SLOT.with(|slot| {
            *slot.borrow_mut() = None;
        });
        let _ = self.invalidate_all_binding_state();
    }

    /// Clear cached editor and runtime projections for one formula. Future
    /// UDF registration, unregister, disable, and source-change events should
    /// call this after publishing a bind-visible function-surface generation.
    pub fn invalidate_formula_binding_state(
        &self,
        formula_stable_id: &str,
    ) -> Result<usize, OxfmlEditorBridgeError> {
        let mut invalidated = 0;
        {
            let mut cached_documents = self.cached_documents.lock().map_err(|_| {
                OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string())
            })?;
            invalidated += usize::from(cached_documents.remove(formula_stable_id).is_some());
        }
        {
            let mut last_result = self.last_result.lock().map_err(|_| {
                OxfmlEditorBridgeError::UpstreamFailure(
                    "Live bridge result cache poisoned".to_string(),
                )
            })?;
            invalidated += usize::from(last_result.remove(formula_stable_id).is_some());
        }
        Ok(invalidated)
    }

    /// Clear all cached editor and runtime projections. This is the coarse
    /// invalidation hook for a workspace-level function-surface generation
    /// change until formula-level dependency targeting is available.
    pub fn invalidate_all_binding_state(&self) -> Result<usize, OxfmlEditorBridgeError> {
        let mut cached_documents = self.cached_documents.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge cache poisoned".to_string())
        })?;
        let mut last_result = self.last_result.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge result cache poisoned".to_string())
        })?;
        let invalidated = cached_documents.len() + last_result.len();
        cached_documents.clear();
        last_result.clear();
        Ok(invalidated)
    }

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

    /// Returns the cached `EditorDocument` for `formula_stable_id`
    /// when its fingerprint matches the supplied one. `None` when
    /// no entry exists or the fingerprint differs.
    fn cached_result(
        &self,
        formula_stable_id: &str,
        fingerprint: &FormulaEditFingerprint,
    ) -> Result<Option<EditorDocument>, OxfmlEditorBridgeError> {
        let last_result = self.last_result.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge result cache poisoned".to_string())
        })?;
        Ok(last_result
            .get(formula_stable_id)
            .filter(|cached| &cached.fingerprint == fingerprint)
            .map(|cached| cached.document.clone()))
    }

    fn cache_result(
        &self,
        formula_stable_id: String,
        fingerprint: FormulaEditFingerprint,
        document: EditorDocument,
    ) -> Result<(), OxfmlEditorBridgeError> {
        let mut last_result = self.last_result.lock().map_err(|_| {
            OxfmlEditorBridgeError::UpstreamFailure("Live bridge result cache poisoned".to_string())
        })?;
        last_result.insert(
            formula_stable_id,
            CachedFormulaEditResult {
                fingerprint,
                document,
            },
        );
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
    projection_drill_trace: Option<&FormulaDrillTrace>,
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
        formula_walk: runtime_result
            .and_then(|result| result.formula_drill_trace.as_ref())
            .or(projection_drill_trace)
            .map(map_formula_walk_from_trace)
            .filter(|nodes| !nodes.is_empty())
            .or_else(|| runtime_result.map(map_formula_walk))
            .unwrap_or_else(|| {
                vec![fallback_formula_walk_node(
                    "node:source",
                    "CellEntry",
                    Some(document.source.entered_formula_text.clone()),
                    if blocked_reason.is_some() {
                        FormulaWalkNodeState::Blocked
                    } else if document.bound_formula.is_some() {
                        FormulaWalkNodeState::Evaluated
                    } else {
                        FormulaWalkNodeState::Opaque
                    },
                )]
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
                .and_then(|result| result.formula_drill_trace.as_ref())
                .map(|trace| trace.evaluation_order.len())
                .or_else(|| projection_drill_trace.map(|trace| trace.nodes.len()))
                .or_else(|| {
                    runtime_result.map(|result| result.evaluation.trace.prepared_calls.len())
                })
                .unwrap_or_else(|| usize::from(document.semantic_plan.is_some())),
            duration_text: runtime_result
                .and_then(|result| result.formula_drill_trace.as_ref())
                .map(|trace| format!("{} trace node(s)", trace.nodes.len()))
                .or_else(|| {
                    projection_drill_trace
                        .map(|trace| format!("{} projected trace node(s)", trace.nodes.len()))
                })
                .or_else(|| {
                    runtime_result.map(|result| {
                        format!(
                            "{} prepared call(s)",
                            result.evaluation.trace.prepared_calls.len()
                        )
                    })
                })
                .unwrap_or_else(|| "edit-only".to_string()),
        }),
        provenance_summary: Some(ProvenanceSummary {
            profile_summary: runtime_result
                .map(|result| format!("OxFml runtime · {:?}", result.returned_value_surface.kind))
                .unwrap_or_else(|| {
                    if projection_drill_trace.is_some() {
                        "OxFml formula drill projection".to_string()
                    } else {
                        "OxFml editor".to_string()
                    }
                }),
            blocked_reason,
        }),
        value_presentation: runtime_result.map(map_value_presentation),
    }
}

fn map_formula_walk(result: &RuntimeFormulaResult) -> Vec<FormulaWalkNode> {
    if result.evaluation.trace.prepared_calls.is_empty() {
        return vec![fallback_formula_walk_node(
            "node:formula",
            "Formula",
            Some(format_walk_value_preview(&result.published_worksheet_value)),
            FormulaWalkNodeState::Evaluated,
        )];
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
            developer_label: None,
            expression_text: None,
            kind: Some("PreparedCall".to_string()),
            source_span_start: None,
            source_span_len: None,
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
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
                    developer_label: None,
                    expression_text: None,
                    kind: Some("PreparedArgument".to_string()),
                    source_span_start: None,
                    source_span_len: None,
                    branch_disposition: None,
                    argument_name: Some(format!("arg[{}]", argument.ordinal)),
                    argument_role: None,
                    error_message: None,
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

fn map_formula_walk_from_trace(trace: &FormulaDrillTrace) -> Vec<FormulaWalkNode> {
    let root = trace
        .nodes
        .iter()
        .find(|node| node.node_id == trace.root_node_id);
    match root {
        Some(root) => vec![map_formula_drill_trace_node(trace, root)],
        None => trace
            .nodes
            .iter()
            .filter(|node| node.parent_node_id.is_none())
            .map(|node| map_formula_drill_trace_node(trace, node))
            .collect(),
    }
}

fn map_formula_drill_trace_node(
    trace: &FormulaDrillTrace,
    node: &FormulaDrillTraceNode,
) -> FormulaWalkNode {
    let children = node
        .child_node_ids
        .iter()
        .filter_map(|child_id| {
            trace
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *child_id)
        })
        .map(|child| map_formula_drill_trace_node(trace, child))
        .collect();
    let source_span = node.source_span;
    FormulaWalkNode {
        node_id: node.node_id.0.clone(),
        label: node.label_user.clone(),
        developer_label: Some(node.label_developer.clone()),
        expression_text: node.expression_text.clone(),
        kind: Some(format!("{:?}", node.kind)),
        source_span_start: source_span.map(|span| span.start),
        source_span_len: source_span.map(|span| span.len),
        branch_disposition: node
            .branch_disposition
            .as_ref()
            .map(|disposition| format!("{disposition:?}")),
        argument_name: node.argument_name.clone(),
        argument_role: node.argument_role.as_ref().map(|role| format!("{role:?}")),
        error_message: node.error.as_ref().map(|error| {
            error
                .code
                .as_ref()
                .map(|code| format!("{code}: {}", error.message))
                .unwrap_or_else(|| error.message.clone())
        }),
        value_preview: node
            .value_preview
            .as_ref()
            .map(format_drill_value_preview)
            .or_else(|| node.returned_value.as_ref().map(format_walk_value_preview))
            .or_else(|| node.published_value.as_ref().map(format_walk_value_preview))
            .or_else(|| {
                node.value_after_coercion
                    .as_ref()
                    .map(format_walk_value_preview)
            })
            .or_else(|| {
                node.value_before_coercion
                    .as_ref()
                    .map(format_walk_value_preview)
            })
            .or_else(|| node.error.as_ref().map(|error| error.message.clone())),
        state: map_drill_evaluation_state(node.evaluation_state),
        children,
    }
}

fn map_drill_evaluation_state(state: FormulaDrillEvaluationState) -> FormulaWalkNodeState {
    match state {
        FormulaDrillEvaluationState::Pending => FormulaWalkNodeState::Pending,
        FormulaDrillEvaluationState::Bound => FormulaWalkNodeState::Bound,
        FormulaDrillEvaluationState::Evaluated => FormulaWalkNodeState::Evaluated,
        FormulaDrillEvaluationState::Skipped
        | FormulaDrillEvaluationState::ShortCircuited
        | FormulaDrillEvaluationState::Omitted => FormulaWalkNodeState::Skipped,
        FormulaDrillEvaluationState::Blocked => FormulaWalkNodeState::Blocked,
        FormulaDrillEvaluationState::Error => FormulaWalkNodeState::Error,
    }
}

fn format_drill_value_preview(
    preview: &oxfml_core::consumer::runtime::FormulaDrillValuePreview,
) -> String {
    let mut text = if let Some(shape) = preview.array_shape {
        format!(
            "{} {}x{} [{}]",
            preview.value_kind,
            shape.rows,
            shape.cols,
            preview.preview.join(", ")
        )
    } else {
        preview.preview.join(", ")
    };
    if text.is_empty() {
        text = preview.value_kind.clone();
    }
    if preview.truncated {
        text.push_str(" ...");
    }
    if let Some(type_name) = preview.rich_value_type_name.as_ref() {
        text.push_str(&format!(" ({type_name})"));
    }
    text
}

fn fallback_formula_walk_node(
    node_id: &str,
    label: &str,
    value_preview: Option<String>,
    state: FormulaWalkNodeState,
) -> FormulaWalkNode {
    FormulaWalkNode {
        node_id: node_id.to_string(),
        label: label.to_string(),
        developer_label: None,
        expression_text: None,
        kind: None,
        source_span_start: None,
        source_span_len: None,
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview,
        state,
        children: Vec::new(),
    }
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
        semantic_kernel_metadata_version: result
            .prepared_formula_identity
            .semantic_kernel_metadata_version
            .clone(),
        arg_admission_metadata_version: result
            .prepared_formula_identity
            .arg_admission_metadata_version
            .clone(),
        producer_capability_set_keys: result
            .returned_value_surface
            .producer_capability_set_keys
            .clone(),
        exercised_capability_keys: result
            .returned_value_surface
            .exercised_capability_keys
            .clone(),
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

/// Derive the runtime volatile context from the active calc-options
/// policy. Deterministic mode pins the clock and uses a reproducible
/// provider stream; live/manual modes read the clock at dispatch time
/// and let each volatile random function consume its own provider draw.
/// Resolve the runtime locale context for a bridge call. Pulls
/// the profile via `LocaleProfileId::from_bcp47_language_tag` —
/// unrecognised / empty tags fall back to en-US. The
/// `WorkbookDateSystem` follows the formula's `date1904` flag.
///
/// Reusing the static `oxfml_en_us_locale_context()` for the
/// fallback path keeps the en-US case allocation-free; non-en-US
/// locales build a fresh context each call (cheap — `FormatProfile`
/// is `Copy`, the parser/formatter trait objects are `'static`).
fn build_runtime_locale_context(
    language_tag: &str,
    date1904: bool,
) -> oxfunc_core::locale_format::LocaleFormatContext<'static> {
    let date_system = if date1904 {
        WorkbookDateSystem::System1904
    } else {
        WorkbookDateSystem::System1900
    };
    let profile_id = LocaleProfileId::from_bcp47_language_tag(language_tag.trim())
        .unwrap_or(LocaleProfileId::EnUs);
    if profile_id == LocaleProfileId::EnUs && !date1904 {
        return oxfml_en_us_locale_context();
    }
    oxfml_locale_context(format_profile(profile_id), date_system)
}

fn scenario_runtime_context(policy: ScenarioPolicyRequest) -> (f64, Box<dyn RandomProvider>) {
    match policy {
        ScenarioPolicyRequest::Deterministic => (
            46000.0,
            Box::new(SplitMixRandomProvider::new(0xD1A0_0ECA_1C5E_ED5E)),
        ),
        ScenarioPolicyRequest::LiveRecalc | ScenarioPolicyRequest::ManualRecalc => {
            (current_excel_serial(), current_random_provider())
        }
    }
}

struct SplitMixRandomProvider {
    state: Cell<u64>,
}

impl SplitMixRandomProvider {
    fn new(seed: u64) -> Self {
        Self {
            state: Cell::new(seed),
        }
    }
}

impl RandomProvider for SplitMixRandomProvider {
    fn random_unit(&self) -> f64 {
        let mut z = self.state.get().wrapping_add(0x9E3779B97F4A7C15);
        self.state.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        (z >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

#[cfg(target_arch = "wasm32")]
struct JsMathRandomProvider;

#[cfg(target_arch = "wasm32")]
impl RandomProvider for JsMathRandomProvider {
    fn random_unit(&self) -> f64 {
        js_sys::Math::random()
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
fn current_random_provider() -> Box<dyn RandomProvider> {
    Box::new(JsMathRandomProvider)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_excel_serial() -> f64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    elapsed.as_secs_f64() / 86_400.0 + 25_569.0
}

#[cfg(not(target_arch = "wasm32"))]
fn current_random_provider() -> Box<dyn RandomProvider> {
    // Lightweight non-wasm RNG: derive a u64 from the current
    // nanosecond clock and run it through a small splittable
    // mixer. Sufficient for volatile formula previews; SSR / desktop builds
    // are not running cryptographic RNGs out of this site.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0);
    Box::new(SplitMixRandomProvider::new(nanos))
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
