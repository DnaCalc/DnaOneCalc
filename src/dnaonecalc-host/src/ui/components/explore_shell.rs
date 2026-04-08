use leptos::prelude::*;

use crate::ui::components::formula_editor_surface::FormulaEditorSurface;
use crate::ui::editor::commands::{EditorCommand, EditorInputEvent};
use crate::ui::editor::geometry::EditorOverlayMeasurementEvent;
use crate::ui::panels::explore::{ExploreEditorClusterViewModel, ExploreResultClusterViewModel};

#[component]
fn ExploreEditorPanel(
    editor: ExploreEditorClusterViewModel,
    on_input_event: Option<Callback<EditorInputEvent>>,
    on_command: Option<Callback<EditorCommand>>,
    on_overlay_measurement: Option<Callback<EditorOverlayMeasurementEvent>>,
) -> impl IntoView {
    let function_help = editor
        .function_help_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());
    let diagnostics_label = if editor.diagnostics.is_empty() {
        "Clean".to_string()
    } else {
        format!("{} issue(s)", editor.diagnostics.len())
    };

    view! {
        <section class="onecalc-explore-shell__editor-panel" data-panel="explore-editor">
            <div class="onecalc-explore-shell__panel-header">
                <div>
                    <div class="onecalc-explore-shell__section-accent"></div>
                    <h2>"Editor"</h2>
                    <div class="onecalc-explore-shell__eyebrow">"Primary authoring surface"</div>
                </div>
            </div>
            <div class="onecalc-explore-shell__panel-intro" data-role="explore-panel-intro">
                <div class="onecalc-explore-shell__eyebrow">"Formula authoring"</div>
                <p>
                    "Keep the cell entry dominant. Diagnostics stay close to the formula, while result and guided help remain visible without competing for focus."
                </p>
            </div>
            <div class="onecalc-explore-shell__editor-summary-row" data-role="explore-editor-summary">
                <div class="onecalc-explore-shell__status-card" data-role="explore-diagnostics-summary">
                    <span class="onecalc-explore-shell__status-label">"Diagnostics"</span>
                    <strong>{diagnostics_label}</strong>
                </div>
                <div class="onecalc-explore-shell__status-card" data-role="explore-completion-summary">
                    <span class="onecalc-explore-shell__status-label">"Assist"</span>
                    <strong>{format!("{} proposal(s)", editor.completion_count)}</strong>
                </div>
                <div class="onecalc-explore-shell__status-card" data-role="explore-authoring-summary">
                    <span class="onecalc-explore-shell__status-label">"Authoring state"</span>
                    <strong>{if editor.reused_green_tree { "Incremental reuse" } else { "Fresh analysis" }}</strong>
                </div>
            </div>
            <div class="onecalc-explore-shell__editor-note" data-role="explore-editor-note">
                <span class="onecalc-explore-shell__metric-label">"Current help target"</span>
                <strong>{function_help.clone()}</strong>
                {editor.trace_summary.as_ref().map(|trace_summary| view! {
                    <span class="onecalc-explore-shell__editor-note-detail">{trace_summary.clone()}</span>
                })}
            </div>
            {editor.blocked_reason.as_ref().map(|blocked_reason| view! {
                <div class="onecalc-explore-shell__blocked-reason" data-role="explore-blocked-reason">
                    {blocked_reason.clone()}
                </div>
            })}
            <FormulaEditorSurface
                editor=editor.clone()
                on_input_event=on_input_event
                on_command=on_command
                on_overlay_measurement=on_overlay_measurement
            />
        </section>
    }
}

#[component]
fn ExploreResultPanel(result: ExploreResultClusterViewModel) -> impl IntoView {
    let result_value = result
        .result_value_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
    let effective_display = result
        .effective_display_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
    let evaluation_summary = result
        .latest_evaluation_summary
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());
    let has_array_preview = result.array_preview.is_some();

    view! {
        <section class="onecalc-explore-shell__result-panel" data-panel="explore-result">
            <div class="onecalc-explore-shell__panel-header">
                <div>
                    <div class="onecalc-explore-shell__section-accent"></div>
                    <h2>"Result"</h2>
                    <div class="onecalc-explore-shell__eyebrow">"Runtime view"</div>
                </div>
                <div class="onecalc-explore-shell__result-state-chip" data-role="explore-result-state-chip">
                    {if has_array_preview { "Array" } else { "Scalar" }}
                </div>
            </div>
            <section class="onecalc-explore-shell__hero-result" data-role="explore-hero-result">
                <div class="onecalc-explore-shell__hero-result-copy">
                    <div class="onecalc-explore-shell__hero-result-label">"Calculated value"</div>
                    <div class="onecalc-explore-shell__hero-result-value" data-role="explore-result-value">
                        {result_value}
                    </div>
                    <p class="onecalc-explore-shell__hero-result-caption">
                        "This is the current OxFml-visible result surface for the active cell entry."
                    </p>
                </div>
                <div class="onecalc-explore-shell__hero-result-sidecar">
                    <div class="onecalc-explore-shell__hero-pill">
                        <span>"Display"</span>
                        <strong>{effective_display.clone()}</strong>
                    </div>
                    <div class="onecalc-explore-shell__hero-pill">
                        <span>"Evaluation"</span>
                        <strong>{evaluation_summary.clone()}</strong>
                    </div>
                </div>
            </section>
            <div class="onecalc-explore-shell__result-grid">
                <div class="onecalc-explore-shell__result-metric" data-role="explore-effective-display">
                    <span class="onecalc-explore-shell__metric-label">"Effective display"</span>
                    <strong>{effective_display}</strong>
                </div>
                <div class="onecalc-explore-shell__result-metric" data-role="explore-evaluation-summary">
                    <span class="onecalc-explore-shell__metric-label">"Evaluation summary"</span>
                    <strong>{evaluation_summary}</strong>
                </div>
            </div>
            {result.array_preview.as_ref().map(|array_preview| view! {
                <section class="onecalc-explore-shell__array-preview" data-role="explore-array-preview">
                    <header class="onecalc-explore-shell__array-preview-header">
                        <h3>{array_preview.label.clone()}</h3>
                        <span class="onecalc-explore-shell__array-preview-badge">
                            {if array_preview.truncated { "truncated" } else { "bounded" }}
                        </span>
                    </header>
                    <div class="onecalc-explore-shell__array-grid">
                        {array_preview.rows.iter().map(|row| view! {
                            <div class="onecalc-explore-shell__array-row">
                                {row.iter().map(|cell| view! {
                                    <span class="onecalc-explore-shell__array-cell">{cell.clone()}</span>
                                }).collect_view()}
                            </div>
                        }).collect_view()}
                    </div>
                </section>
            })}
        </section>
    }
}

#[component]
fn ExploreHelpPanel(editor: ExploreEditorClusterViewModel) -> impl IntoView {
    let function_help = editor
        .function_help_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());
    let help_summary = if editor.has_signature_help {
        "Signature help available"
    } else {
        "Signature help unavailable"
    };
    let help_sync_lookup = editor
        .help_sync_lookup_key
        .clone()
        .unwrap_or_else(|| "None".to_string());

    view! {
        <section class="onecalc-explore-shell__help-panel" data-panel="explore-help">
            <div class="onecalc-explore-shell__panel-header">
                <div>
                    <div class="onecalc-explore-shell__section-accent"></div>
                    <h2>"Assist"</h2>
                    <div class="onecalc-explore-shell__eyebrow">"Guided entry"</div>
                </div>
            </div>
            <div class="onecalc-explore-shell__assist-intro">
                "Use this rail as guided reference. It should help the author move forward without displacing the formula or result surfaces."
            </div>
            <div class="onecalc-explore-shell__assist-callout" data-role="explore-assist-callout">
                <div>
                    <div class="onecalc-explore-shell__eyebrow">"Inspect handoff"</div>
                    <strong>"Switch to semantic inspection when guidance is no longer enough and the formula needs explanation."</strong>
                </div>
                <span class="onecalc-explore-shell__assist-callout-state">"Inspect-ready"</span>
            </div>
            <div class="onecalc-explore-shell__assist-meta" data-role="explore-assist-meta">
                <div class="onecalc-explore-shell__assist-metric">
                    <span class="onecalc-explore-shell__metric-label">"Function target"</span>
                    <strong>{function_help}</strong>
                </div>
                <div class="onecalc-explore-shell__assist-metric" data-role="help-sync-lookup">
                    <span class="onecalc-explore-shell__metric-label">"Help sync"</span>
                    <strong>{help_sync_lookup}</strong>
                </div>
                <div class="onecalc-explore-shell__assist-metric">
                    <span class="onecalc-explore-shell__metric-label">"Signature help"</span>
                    <strong>{help_summary}</strong>
                </div>
                <div class="onecalc-explore-shell__assist-metric">
                    <span class="onecalc-explore-shell__metric-label">"Completion entries"</span>
                    <strong>{editor.completion_count}</strong>
                </div>
            </div>
            {editor
                .selected_completion_item
                .as_ref()
                .map(|item| {
                    view! {
                        <div class="onecalc-explore-shell__selected-proposal" data-role="selected-completion-summary">
                            <div class="onecalc-explore-shell__selected-proposal-header">
                                <span class="onecalc-explore-shell__eyebrow">"Selected proposal"</span>
                                <span data-role="selected-completion-kind">
                                {match item.proposal_kind {
                                    crate::services::explore_mode::ExploreCompletionKindView::Function => "function",
                                    crate::services::explore_mode::ExploreCompletionKindView::DefinedName => "defined-name",
                                    crate::services::explore_mode::ExploreCompletionKindView::TableName => "table-name",
                                    crate::services::explore_mode::ExploreCompletionKindView::TableColumn => "table-column",
                                    crate::services::explore_mode::ExploreCompletionKindView::StructuredSelector => "structured-selector",
                                    crate::services::explore_mode::ExploreCompletionKindView::SyntaxAssist => "syntax-assist",
                                }}
                                </span>
                            </div>
                            <strong data-role="selected-completion-label">{item.display_text.clone()}</strong>
                            <span data-role="selected-completion-doc-ref">
                                {item
                                    .documentation_ref
                                    .clone()
                                    .unwrap_or_else(|| "no-doc-ref".to_string())}
                            </span>
                            <span
                                data-role="selected-completion-revalidation"
                                data-requires-revalidation=if item.requires_revalidation { "true" } else { "false" }
                            >
                                {if item.requires_revalidation {
                                    "requires revalidation"
                                } else {
                                    "stable"
                                }}
                            </span>
                        </div>
                    }
                })}
            {editor
                .function_help
                .as_ref()
                .map(|help| {
                    let active_argument_index = editor
                        .signature_help
                        .as_ref()
                        .map(|signature_help| signature_help.active_argument_index);
                    view! {
                        <article class="onecalc-explore-shell__function-help" data-role="function-help-card">
                            <header class="onecalc-explore-shell__function-help-header">
                                <div data-role="function-help-display-name">{help.display_name.clone()}</div>
                                <div
                                    class=("onecalc-explore-shell__function-help-status", true)
                                    class=("onecalc-explore-shell__function-help-status--limited", help.deferred_or_profile_limited)
                                    data-role="function-help-availability"
                                >
                                    {help
                                        .availability_summary
                                        .clone()
                                        .unwrap_or_else(|| "availability unknown".to_string())}
                                </div>
                            </header>
                            {help
                                .short_description
                                .as_ref()
                                .map(|description| view! {
                                    <p class="onecalc-explore-shell__function-help-description" data-role="function-help-description">
                                        {description.clone()}
                                    </p>
                                })}
                            <div class="onecalc-explore-shell__function-help-signatures" data-role="function-help-signatures">
                                {help
                                    .signature_forms
                                    .iter()
                                    .enumerate()
                                    .map(|(index, form)| {
                                        view! {
                                            <div
                                                class="onecalc-explore-shell__function-help-signature"
                                                data-role="function-help-signature"
                                                data-signature-index=index
                                            >
                                                {render_function_help_signature(
                                                    &form.display_signature,
                                                    active_argument_index,
                                                )}
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="onecalc-explore-shell__function-help-arguments" data-role="function-help-arguments">
                                {help
                                    .argument_help
                                    .iter()
                                    .enumerate()
                                    .map(|(index, argument)| {
                                        let active = active_argument_index == Some(index);
                                        view! {
                                            <div
                                                class=("onecalc-explore-shell__function-help-argument", true)
                                                class=("onecalc-explore-shell__function-help-argument--active", active)
                                                data-role="function-help-argument"
                                                data-active=if active { "true" } else { "false" }
                                            >
                                                {argument.clone()}
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </article>
                    }
                })}
        </section>
    }
}

fn render_function_help_signature(
    display_signature: &str,
    active_argument_index: Option<usize>,
) -> AnyView {
    let (prefix, arguments, suffix) = split_signature(display_signature);

    view! {
        <span class="onecalc-signature-form">
            <span data-role="function-help-signature-prefix">{prefix}</span>
            {arguments
                .into_iter()
                .enumerate()
                .map(|(index, argument)| {
                    let active = active_argument_index == Some(index);
                    view! {
                        <>
                            {if index > 0 {
                                view! { <span data-role="function-help-signature-separator">{", "}</span> }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                            <span
                                class=("onecalc-signature-argument", true)
                                class=("onecalc-signature-argument--active", active)
                                data-role="function-help-signature-argument"
                                data-active=if active { "true" } else { "false" }
                            >
                                {argument}
                            </span>
                        </>
                    }
                })
                .collect_view()}
            <span data-role="function-help-signature-suffix">{suffix}</span>
        </span>
    }
    .into_any()
}

fn split_signature(display_signature: &str) -> (String, Vec<String>, String) {
    let Some(open_index) = display_signature.find('(') else {
        return (display_signature.to_string(), Vec::new(), String::new());
    };
    let Some(close_index) = display_signature.rfind(')') else {
        return (display_signature.to_string(), Vec::new(), String::new());
    };
    if close_index <= open_index {
        return (display_signature.to_string(), Vec::new(), String::new());
    }

    let prefix = display_signature[..=open_index].to_string();
    let inner = &display_signature[(open_index + 1)..close_index];
    let suffix = display_signature[close_index..].to_string();
    let arguments = inner
        .split(',')
        .map(|argument| argument.trim().to_string())
        .filter(|argument| !argument.is_empty())
        .collect();
    (prefix, arguments, suffix)
}

#[component]
pub fn ExploreShell(
    editor: ExploreEditorClusterViewModel,
    result: ExploreResultClusterViewModel,
    #[prop(default = None)] on_input_event: Option<Callback<EditorInputEvent>>,
    #[prop(default = None)] on_command: Option<Callback<EditorCommand>>,
    #[prop(default = None)] on_overlay_measurement: Option<Callback<EditorOverlayMeasurementEvent>>,
) -> impl IntoView {
    view! {
        <section class="onecalc-explore-shell" data-screen="explore">
            <header class="onecalc-explore-shell__header">
                <div class="onecalc-explore-shell__header-copy">
                    <div>
                        <div class="onecalc-explore-shell__eyebrow">"Explore"</div>
                        <h1>"Formula Explorer"</h1>
                    </div>
                    <div class="onecalc-explore-shell__hero-badges">
                        <span>{editor.scenario_label.clone()}</span>
                        <span>{editor.truth_source_label.clone()}</span>
                    </div>
                </div>
                <p class="onecalc-explore-shell__lead">
                    "Author the cell entry on the left, read the evaluated result in the center, and keep guided help on the right. The screen should support sustained formula work without making you hunt for truth."
                </p>
                <section class="onecalc-explore-shell__overview-deck" data-role="explore-overview-deck">
                    <article class="onecalc-explore-shell__overview-card" data-role="explore-overview-primary">
                        <div class="onecalc-explore-shell__eyebrow">"Current formula space"</div>
                        <strong>{editor.scenario_label.clone()}</strong>
                        <p>
                            "The left surface is the primary authoring space for the active scenario."
                        </p>
                    </article>
                    <article class="onecalc-explore-shell__overview-card" data-role="explore-overview-result">
                        <div class="onecalc-explore-shell__eyebrow">"Visible display"</div>
                        <strong>{result.effective_display_summary.clone().unwrap_or_else(|| "Unavailable".to_string())}</strong>
                        <p>{result.result_value_summary.clone().unwrap_or_else(|| "No result surface yet".to_string())}</p>
                    </article>
                    <article class="onecalc-explore-shell__overview-card" data-role="explore-overview-capability">
                        <div class="onecalc-explore-shell__eyebrow">"Authoring posture"</div>
                        <strong>{if editor.diagnostics.is_empty() { "Ready to iterate" } else { "Needs repair" }}</strong>
                        <p>
                            {format!(
                                "{} diagnostic(s), {} completion proposal(s)",
                                editor.diagnostics.len(),
                                editor.completion_count
                            )}
                        </p>
                    </article>
                </section>
            </header>

            <div class="onecalc-explore-shell__body">
                <div class="onecalc-explore-shell__body-column onecalc-explore-shell__body-column--editor">
                    <ExploreEditorPanel
                        editor=editor.clone()
                        on_input_event=on_input_event
                        on_command=on_command
                        on_overlay_measurement=on_overlay_measurement
                    />
                </div>
                <div class="onecalc-explore-shell__body-column onecalc-explore-shell__body-column--result">
                    <ExploreResultPanel result=result />
                </div>
                <div class="onecalc-explore-shell__body-column onecalc-explore-shell__body-column--help">
                    <ExploreHelpPanel editor=editor />
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::FormulaTextSpan;
    use crate::services::explore_mode::{
        ExploreCompletionItemView, ExploreCompletionKindView, ExploreDiagnosticView,
        ExploreFunctionHelpSignatureView, ExploreFunctionHelpView, ExploreSignatureHelpView,
        ExploreViewModel,
    };
    use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
    use crate::ui::panels::explore::{build_explore_editor_cluster, build_explore_result_cluster};

    #[test]
    fn explore_shell_renders_editor_and_result_content() {
        let view_model = ExploreViewModel {
            scenario_label: "Success · SUM result".to_string(),
            truth_source_label: "preview-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            packet_kind_summary: "preview edit packet".to_string(),
            capability_floor_summary: "Explore + Inspect".to_string(),
            mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
            trace_summary: Some("Preview packet reused green=false, bind complete".to_string()),
            blocked_reason: None,
            raw_entered_cell_text: "=SUM(1,2)".to_string(),
            editor_surface_state: crate::ui::editor::state::EditorSurfaceState {
                completion_selected_index: Some(0),
                completion_anchor_offset: Some(4),
                signature_help_anchor_offset: Some(4),
                ..crate::ui::editor::state::EditorSurfaceState::for_text("=SUM(1,2)")
            },
            overlay_geometry: None,
            syntax_runs: vec![SyntaxRun {
                text: "SUM".to_string(),
                span_start: 1,
                span_len: 3,
                role: SyntaxTokenRole::Function,
            }],
            diagnostics: vec![ExploreDiagnosticView {
                diagnostic_id: "diag-1".to_string(),
                message: "sample".to_string(),
                span_start: 1,
                span_len: 3,
            }],
            completion_count: 2,
            completion_items: vec![ExploreCompletionItemView {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: ExploreCompletionKindView::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 3 }),
                documentation_ref: Some("preview:function:SUM".to_string()),
                requires_revalidation: true,
            }],
            has_signature_help: true,
            signature_help: Some(ExploreSignatureHelpView {
                callee_text: "SUM".to_string(),
                call_span: FormulaTextSpan { start: 0, len: 9 },
                active_argument_index: 1,
            }),
            function_help: Some(ExploreFunctionHelpView {
                lookup_key: "SUM".to_string(),
                display_name: "SUM".to_string(),
                signature_forms: vec![ExploreFunctionHelpSignatureView {
                    display_signature: "SUM(number1, number2, ...)".to_string(),
                    min_arity: 1,
                    max_arity: None,
                }],
                argument_help: vec!["number1".to_string(), "number2".to_string()],
                short_description: Some("Adds numbers".to_string()),
                availability_summary: Some("supported".to_string()),
                deferred_or_profile_limited: false,
            }),
            function_help_lookup_key: Some("SUM".to_string()),
            result_value_summary: Some("Number · 3".to_string()),
            effective_display_summary: Some("3".to_string()),
            latest_evaluation_summary: Some("Number".to_string()),
            array_preview: Some(crate::services::explore_mode::ExploreArrayPreviewView {
                label: "2x2 spill preview".to_string(),
                rows: vec![
                    vec!["1".to_string(), "2".to_string()],
                    vec!["3".to_string(), "4".to_string()],
                ],
                truncated: false,
            }),
            green_tree_key: Some("green-1".to_string()),
            reused_green_tree: true,
        };

        let html = view! {
            <ExploreShell
                editor=build_explore_editor_cluster(&view_model)
                result=build_explore_result_cluster(&view_model)
            />
        }
        .to_html();

        assert!(html.contains("Formula Explorer"));
        assert!(html.contains("data-panel=\"explore-editor\""));
        assert!(html.contains("data-role=\"explore-editor-summary\""));
        assert!(html.contains("data-role=\"explore-editor-note\""));
        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("data-token-role=\"function\""));
        assert!(html.contains("data-panel=\"explore-help\""));
        assert!(html.contains(">3<"));
        assert!(html.contains("data-role=\"explore-assist-meta\""));
        assert!(html.contains("Function target"));
        assert!(html.contains("data-role=\"help-sync-lookup\""));
        assert!(html.contains("data-role=\"selected-completion-summary\""));
        assert!(html.contains("data-role=\"selected-completion-doc-ref\""));
        assert!(html.contains("preview:function:SUM"));
        assert!(html.contains("data-role=\"selected-completion-revalidation\""));
        assert!(html.contains("data-requires-revalidation=\"true\""));
        assert!(html.contains("SUM"));
        assert!(html.contains("Completion entries"));
        assert!(html.contains("data-role=\"explore-array-preview\""));
        assert!(html.contains("2x2 spill preview"));
        assert!(html.contains("data-role=\"explore-result-value\""));
        assert!(html.contains("data-role=\"explore-hero-result\""));
        assert!(html.contains("data-role=\"function-help-card\""));
        assert!(html.contains("data-role=\"function-help-signature\""));
        assert!(html.contains("data-role=\"function-help-signature-argument\""));
        assert!(html.contains("data-active=\"true\""));
    }
}
