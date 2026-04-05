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

    view! {
        <section class="onecalc-explore-shell__editor-panel" data-panel="explore-editor">
            <div class="onecalc-explore-shell__panel-header">
                <div>
                    <h2>"Editor"</h2>
                    <div class="onecalc-explore-shell__scenario-label" data-role="explore-scenario-label">
                        {editor.scenario_label.clone()}
                    </div>
                </div>
                <div class="onecalc-explore-shell__truth-chip" data-role="explore-truth-source">
                    {editor.truth_source_label.clone()}
                </div>
            </div>
            <div class="onecalc-explore-shell__context-strip" data-role="explore-context-strip">
                <span data-role="explore-host-profile">{editor.host_profile_summary.clone()}</span>
                <span data-role="explore-packet-kind">{editor.packet_kind_summary.clone()}</span>
                <span data-role="explore-capability-floor">{editor.capability_floor_summary.clone()}</span>
                <span data-role="explore-mode-availability">{editor.mode_availability_summary.clone()}</span>
            </div>
            {editor.trace_summary.as_ref().map(|trace_summary| view! {
                <div class="onecalc-explore-shell__trace-summary" data-role="explore-trace-summary">
                    {trace_summary.clone()}
                </div>
            })}
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
            <div class="onecalc-explore-shell__help-hint">
                "Function help target: "
                {function_help}
            </div>
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

    view! {
        <section class="onecalc-explore-shell__result-panel" data-panel="explore-result">
            <h2>"Result"</h2>
            <div class="onecalc-explore-shell__result-metric" data-role="explore-result-value">
                "Value: " {result_value}
            </div>
            <div class="onecalc-explore-shell__result-metric" data-role="explore-effective-display">
                "Effective display: " {effective_display}
            </div>
            <div class="onecalc-explore-shell__result-metric" data-role="explore-evaluation-summary">
                "Evaluation summary: " {evaluation_summary}
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
            <h2>"Help"</h2>
            <div>"Function target: " {function_help}</div>
            <div data-role="help-sync-lookup">"Help sync: " {help_sync_lookup}</div>
            <div>{help_summary}</div>
            <div>"Completion entries: " {editor.completion_count}</div>
            {editor
                .selected_completion_item
                .as_ref()
                .map(|item| {
                    view! {
                        <div class="onecalc-explore-shell__selected-proposal" data-role="selected-completion-summary">
                            <span data-role="selected-completion-label">{item.display_text.clone()}</span>
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
                <h1>"Formula Explorer"</h1>
            </header>

            <div class="onecalc-explore-shell__body">
                <ExploreEditorPanel
                    editor=editor.clone()
                    on_input_event=on_input_event
                    on_command=on_command
                    on_overlay_measurement=on_overlay_measurement
                />
                <ExploreResultPanel result=result />
                <ExploreHelpPanel editor=editor />
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::explore_mode::{
        ExploreCompletionItemView, ExploreCompletionKindView, ExploreDiagnosticView,
        ExploreFunctionHelpSignatureView, ExploreFunctionHelpView, ExploreSignatureHelpView,
        ExploreViewModel,
    };
    use crate::adapters::oxfml::FormulaTextSpan;
    use crate::ui::editor::render_projection::{SyntaxRun, SyntaxTokenRole};
    use crate::ui::panels::explore::{
        build_explore_editor_cluster, build_explore_result_cluster,
    };

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
        assert!(html.contains("data-role=\"explore-context-strip\""));
        assert!(html.contains("data-role=\"explore-truth-source\""));
        assert!(html.contains("data-component=\"formula-editor-surface\""));
        assert!(html.contains("data-role=\"editor-input\""));
        assert!(html.contains("data-token-role=\"function\""));
        assert!(html.contains("data-panel=\"explore-help\""));
        assert!(html.contains(">3<"));
        assert!(html.contains("Function target: "));
        assert!(html.contains("data-role=\"help-sync-lookup\""));
        assert!(html.contains("data-role=\"selected-completion-summary\""));
        assert!(html.contains("data-role=\"selected-completion-doc-ref\""));
        assert!(html.contains("preview:function:SUM"));
        assert!(html.contains("data-role=\"selected-completion-revalidation\""));
        assert!(html.contains("data-requires-revalidation=\"true\""));
        assert!(html.contains("SUM"));
        assert!(html.contains("Completion entries: "));
        assert!(html.contains("data-role=\"explore-array-preview\""));
        assert!(html.contains("2x2 spill preview"));
        assert!(html.contains("data-role=\"explore-result-value\""));
        assert!(html.contains("data-role=\"function-help-card\""));
        assert!(html.contains("data-role=\"function-help-signature\""));
        assert!(html.contains("data-role=\"function-help-signature-argument\""));
        assert!(html.contains("data-active=\"true\""));
    }
}
