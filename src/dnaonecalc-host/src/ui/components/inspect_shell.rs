use leptos::prelude::*;

use crate::ui::panels::inspect::{InspectSummaryClusterViewModel, InspectWalkClusterViewModel};

#[component]
fn InspectWalkNode(
    node: crate::services::inspect_mode::InspectFormulaWalkNodeView,
) -> impl IntoView {
    let state_label = format!("{:?}", node.state);
    let value_preview = node
        .value_preview
        .clone()
        .unwrap_or_else(|| "Unavailable".to_string());

    view! {
        <li class="onecalc-inspect-shell__walk-node" data-node-id=node.node_id.clone()>
            <div class="onecalc-inspect-shell__walk-node-header">
                <span class="onecalc-inspect-shell__walk-node-label">{node.label.clone()}</span>
                <span class="onecalc-inspect-shell__walk-node-state" data-node-state=state_label.clone()>{state_label.clone()}</span>
            </div>
            <div class="onecalc-inspect-shell__walk-node-value">
                <span class="onecalc-inspect-shell__walk-node-value-label">"Value preview"</span>
                <strong>{value_preview}</strong>
            </div>
            {if node.children.is_empty() {
                view! { <></> }.into_any()
            } else {
                view! {
                    <ul class="onecalc-inspect-shell__walk-node-children">
                        {node
                            .children
                            .into_iter()
                            .map(|child| view! { <InspectWalkNode node=child /> })
                            .collect_view()}
                    </ul>
                }
                .into_any()
            }}
        </li>
    }
}

#[component]
fn InspectSummaryCard(
    title: &'static str,
    value: String,
    data_panel: &'static str,
) -> impl IntoView {
    view! {
        <section class="onecalc-inspect-shell__summary-card" data-panel=data_panel>
            <div class="onecalc-inspect-shell__eyebrow">"Summary"</div>
            <h3>{title}</h3>
            <div>{value}</div>
        </section>
    }
}

#[component]
pub fn InspectShell(
    walk: InspectWalkClusterViewModel,
    summary: InspectSummaryClusterViewModel,
) -> impl IntoView {
    let parse_status = summary
        .parse_summary
        .as_ref()
        .map(|value| value.status.clone())
        .unwrap_or_else(|| "Unavailable".to_string());
    let bind_summary = summary
        .bind_summary
        .as_ref()
        .map(|value| {
            format!(
                "vars={}, refs={}",
                value.variable_count, value.reference_count
            )
        })
        .unwrap_or_else(|| "Unavailable".to_string());
    let eval_summary = summary
        .eval_summary
        .as_ref()
        .map(|value| {
            format!(
                "steps={}, duration={}",
                value.step_count, value.duration_text
            )
        })
        .unwrap_or_else(|| "Unavailable".to_string());
    let provenance_summary = summary
        .provenance_summary
        .as_ref()
        .map(|value| value.profile_summary.clone())
        .unwrap_or_else(|| "Unavailable".to_string());

    view! {
        <section class="onecalc-inspect-shell" data-screen="inspect">
            <header class="onecalc-inspect-shell__header">
                <div>
                    <div class="onecalc-inspect-shell__eyebrow">"Inspect"</div>
                    <h1>"Semantic Inspect"</h1>
                </div>
                <p class="onecalc-inspect-shell__lead">
                    "Dissect parse, bind, eval, provenance, and retained discrepancy context through a semantic x-ray of the entered cell text."
                </p>
                <div class="onecalc-inspect-shell__meta">
                    <span data-role="inspect-scenario-label">{walk.scenario_label.clone()}</span>
                    <span data-role="inspect-truth-source">{walk.truth_source_label.clone()}</span>
                    <span data-role="inspect-host-profile">{walk.host_profile_summary.clone()}</span>
                    <span>"Green tree: " {walk.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Result: " {walk.inspect_result_summary.unwrap_or_else(|| "Unavailable".to_string())}</span>
                </div>
            </header>

            <div class="onecalc-inspect-shell__body">
                <section class="onecalc-inspect-shell__walk-cluster" data-panel="inspect-walk">
                    <div class="onecalc-inspect-shell__panel-header">
                        <div>
                            <div class="onecalc-inspect-shell__eyebrow">"Walk"</div>
                            <h2>"Formula Walk"</h2>
                        </div>
                    </div>
                    <div class="onecalc-inspect-shell__source-card">
                        <div class="onecalc-inspect-shell__source-label">"Cell entry"</div>
                        <pre class="onecalc-inspect-shell__source">{walk.raw_entered_cell_text}</pre>
                    </div>
                    <ul class="onecalc-inspect-shell__walk">
                        {if walk.formula_walk_nodes.is_empty() {
                            view! { <li>"No formula walk"</li> }.into_any()
                        } else {
                            view! {
                                {walk
                                    .formula_walk_nodes
                                    .into_iter()
                                    .map(|node| view! { <InspectWalkNode node=node /> })
                                    .collect_view()}
                            }
                            .into_any()
                        }}
                    </ul>
                </section>

                <section class="onecalc-inspect-shell__summary-cluster" data-panel="inspect-summary">
                    <div class="onecalc-inspect-shell__panel-header">
                        <div>
                            <div class="onecalc-inspect-shell__eyebrow">"Context"</div>
                            <h2>"Summaries"</h2>
                        </div>
                    </div>
                    <section class="onecalc-inspect-shell__context-card" data-role="inspect-context-card">
                        <div data-role="inspect-packet-kind">{summary.packet_kind_summary.clone()}</div>
                        <div data-role="inspect-capability-floor">{summary.capability_floor_summary.clone()}</div>
                        <div data-role="inspect-mode-availability">{summary.mode_availability_summary.clone()}</div>
                        {summary.trace_summary.as_ref().map(|trace| view! {
                            <div data-role="inspect-trace-summary">{trace.clone()}</div>
                        })}
                        {summary.blocked_reason.as_ref().map(|blocked_reason| view! {
                            <div data-role="inspect-blocked-reason">{blocked_reason.clone()}</div>
                        })}
                    </section>
                    {summary
                        .retained_artifact_context
                        .as_ref()
                        .map(|context| {
                            view! {
                                <section
                                    class="onecalc-inspect-shell__retained-context"
                                    data-role="inspect-retained-context"
                                    data-artifact-id=context.artifact_id.clone()
                                    data-comparison-status=context.comparison_status.clone()
                                >
                                    <h3>"Retained Artifact Context"</h3>
                                    <div data-role="inspect-retained-artifact-id">
                                        "Artifact: "
                                        {context.artifact_id.clone()}
                                    </div>
                                    <div data-role="inspect-retained-case-id">
                                        "Case: "
                                        {context.case_id.clone()}
                                    </div>
                                    <div data-role="inspect-retained-comparison-status">
                                        "Status: "
                                        {context.comparison_status.clone()}
                                    </div>
                                    {context
                                        .bundle_report_path
                                        .as_ref()
                                        .map(|bundle_path| view! {
                                            <div data-role="inspect-retained-bundle-path">
                                                "Bundle: "
                                                {bundle_path.clone()}
                                            </div>
                                        })}
                                    {context
                                        .xml_source_summary
                                        .as_ref()
                                        .map(|summary| view! {
                                            <div data-role="inspect-retained-xml-source">
                                                {summary.clone()}
                                            </div>
                                        })}
                                    {context
                                        .display_comparison_summary
                                        .as_ref()
                                        .map(|summary| view! {
                                            <div data-role="inspect-retained-display-comparison">
                                                {summary.clone()}
                                            </div>
                                        })}
                                    {context
                                        .discrepancy_summary
                                        .as_ref()
                                        .map(|summary| view! {
                                            <div data-role="inspect-retained-discrepancy-summary">
                                                {summary.clone()}
                                            </div>
                                        })}
                                    {if context.upstream_gap_summary.is_empty() {
                                        view! { <></> }.into_any()
                                    } else {
                                        view! {
                                            <ul data-role="inspect-retained-upstream-gap-summary">
                                                {context
                                                    .upstream_gap_summary
                                                    .iter()
                                                    .map(|item| view! { <li>{item.clone()}</li> })
                                                    .collect_view()}
                                            </ul>
                                        }
                                            .into_any()
                                    }}
                                </section>
                            }
                        })}
                    <InspectSummaryCard title="Parse" value=parse_status data_panel="inspect-parse" />
                    <InspectSummaryCard title="Bind" value=bind_summary data_panel="inspect-bind" />
                    <InspectSummaryCard title="Eval" value=eval_summary data_panel="inspect-eval" />
                    <InspectSummaryCard title="Provenance" value=provenance_summary data_panel="inspect-provenance" />
                </section>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::FormulaWalkNodeState;
    use crate::adapters::oxfml::{BindSummary, EvalSummary, ParseSummary, ProvenanceSummary};
    use crate::services::inspect_mode::{InspectFormulaWalkNodeView, InspectViewModel};
    use crate::ui::panels::inspect::{build_inspect_summary_cluster, build_inspect_walk_cluster};

    #[test]
    fn inspect_shell_renders_walk_and_summary_content() {
        let view_model = InspectViewModel {
            scenario_label: "Blocked discrepancy".to_string(),
            truth_source_label: "preview-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            packet_kind_summary: "preview blocked packet".to_string(),
            capability_floor_summary: "Inspect with blocked reason".to_string(),
            mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
            trace_summary: Some("Preview trace captured for retained discrepancy".to_string()),
            blocked_reason: Some("Excel comparison lane unavailable on this host".to_string()),
            raw_entered_cell_text: "=LET(x,1,x)".to_string(),
            inspect_result_summary: Some("Number".to_string()),
            green_tree_key: Some("green-1".to_string()),
            formula_walk_nodes: vec![InspectFormulaWalkNodeView {
                node_id: "node-1".to_string(),
                label: "LET".to_string(),
                value_preview: Some("1".to_string()),
                state: FormulaWalkNodeState::Evaluated,
                children: vec![],
            }],
            parse_summary: Some(ParseSummary {
                status: "Valid".to_string(),
                token_count: 7,
            }),
            bind_summary: Some(BindSummary {
                variable_count: 1,
                reference_count: 0,
            }),
            eval_summary: Some(EvalSummary {
                step_count: 2,
                duration_text: "1.2ms".to_string(),
            }),
            provenance_summary: Some(ProvenanceSummary {
                profile_summary: "OC-H0".to_string(),
                blocked_reason: None,
            }),
            retained_artifact_context: Some(
                crate::services::inspect_mode::InspectRetainedArtifactContextView {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: "blocked".to_string(),
                    discrepancy_summary: Some("excel lane unavailable".to_string()),
                    bundle_report_path: Some("target/onecalc-verification/example".to_string()),
                    xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
                    display_comparison_summary: Some("OxFml 6 vs Excel $6.00".to_string()),
                    upstream_gap_summary: vec![
                        "OxXlPlay missing: effective_display_text".to_string(),
                    ],
                },
            ),
        };

        let html = view! {
            <InspectShell
                walk=build_inspect_walk_cluster(&view_model)
                summary=build_inspect_summary_cluster(&view_model)
            />
        }
        .to_html();

        assert!(html.contains("Semantic Inspect"));
        assert!(html.contains("data-role=\"inspect-context-card\""));
        assert!(html.contains("data-role=\"inspect-truth-source\""));
        assert!(html.contains("=LET(x,1,x)"));
        assert!(html.contains("data-panel=\"inspect-walk\""));
        assert!(html.contains("data-node-id=\"node-1\""));
        assert!(html.contains("Parse"));
        assert!(html.contains("Valid"));
        assert!(html.contains("data-role=\"inspect-retained-context\""));
        assert!(html.contains("Artifact: "));
        assert!(html.contains("artifact-1"));
        assert!(html.contains("excel lane unavailable"));
        assert!(html.contains("data-role=\"inspect-retained-bundle-path\""));
        assert!(html.contains("data-role=\"inspect-retained-xml-source\""));
        assert!(html.contains("data-role=\"inspect-retained-display-comparison\""));
        assert!(html.contains("data-role=\"inspect-retained-upstream-gap-summary\""));
    }
}
