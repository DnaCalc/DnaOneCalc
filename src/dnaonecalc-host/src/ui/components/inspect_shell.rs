use leptos::prelude::*;

use crate::services::inspect_mode::{InspectComparisonRecordView, InspectExplainRecordView};
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
                <span
                    class="onecalc-inspect-shell__walk-node-state"
                    data-node-state=state_label.clone()
                >
                    {state_label.clone()}
                </span>
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
fn InspectComparisonRecordCard(record: InspectComparisonRecordView) -> impl IntoView {
    view! {
        <article
            class="onecalc-inspect-shell__comparison-record"
            data-role="inspect-comparison-record"
            data-family=record.view_family.clone().unwrap_or_else(|| record.mismatch_kind.clone())
            data-projection-gap=if record.is_projection_gap { "true" } else { "false" }
        >
            <header class="onecalc-inspect-shell__comparison-record-header">
                <div>
                    <div class="onecalc-inspect-shell__eyebrow">"Comparison family"</div>
                    <h4 data-role="inspect-comparison-family">{record.family_label.clone()}</h4>
                </div>
                <div class="onecalc-inspect-shell__comparison-record-badges">
                    <span data-role="inspect-comparison-kind">{record.status_label.clone()}</span>
                    <span data-role="inspect-comparison-severity">{record.severity.clone()}</span>
                </div>
            </header>
            <p data-role="inspect-comparison-summary">{record.summary.clone()}</p>
            <div class="onecalc-inspect-shell__comparison-lane">
                <div class="onecalc-inspect-shell__comparison-lane-card" data-role="inspect-comparison-left">
                    <span class="onecalc-inspect-shell__walk-node-value-label">"OxFml"</span>
                    <strong>{record.left_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
                <div class="onecalc-inspect-shell__comparison-lane-card" data-role="inspect-comparison-right">
                    <span class="onecalc-inspect-shell__walk-node-value-label">"Excel / replay"</span>
                    <strong>{record.right_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
            </div>
            {record.detail.as_ref().map(|detail| view! {
                <div class="onecalc-inspect-shell__comparison-detail" data-role="inspect-comparison-detail">
                    {detail.clone()}
                </div>
            })}
        </article>
    }
}

#[component]
fn InspectExplainRecordCard(record: InspectExplainRecordView) -> impl IntoView {
    view! {
        <article
            class="onecalc-inspect-shell__explain-record"
            data-role="inspect-explain-record"
            data-family=record.view_family.clone().unwrap_or_else(|| record.mismatch_kind.clone())
        >
            <header class="onecalc-inspect-shell__explain-record-header">
                <div>
                    <div class="onecalc-inspect-shell__eyebrow">"Explain"</div>
                    <h4>{record.family_label.clone()}</h4>
                </div>
                {record.query_id.as_ref().map(|query_id| view! {
                    <span data-role="inspect-explain-query-id">{query_id.clone()}</span>
                })}
            </header>
            <p data-role="inspect-explain-summary">{record.summary.clone()}</p>
            <div class="onecalc-inspect-shell__comparison-lane">
                <div class="onecalc-inspect-shell__comparison-lane-card">
                    <span class="onecalc-inspect-shell__walk-node-value-label">"OxFml"</span>
                    <strong>{record.left_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
                <div class="onecalc-inspect-shell__comparison-lane-card">
                    <span class="onecalc-inspect-shell__walk-node-value-label">"Excel / replay"</span>
                    <strong>{record.right_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
            </div>
            {record.detail.as_ref().map(|detail| view! {
                <div class="onecalc-inspect-shell__comparison-detail">{detail.clone()}</div>
            })}
        </article>
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
                <div class="onecalc-inspect-shell__header-copy">
                    <div>
                        <div class="onecalc-inspect-shell__eyebrow">"Inspect"</div>
                        <h1>"Semantic Inspect"</h1>
                    </div>
                </div>
                <p class="onecalc-inspect-shell__lead">
                    "Dissect formula structure, replay evidence, comparison-family divergence, and retained discrepancy context through one semantic x-ray surface."
                </p>
                <div class="onecalc-inspect-shell__meta">
                    <span data-role="inspect-scenario-label">{walk.scenario_label.clone()}</span>
                    <span data-role="inspect-truth-source">{walk.truth_source_label.clone()}</span>
                    <span data-role="inspect-host-profile">{walk.host_profile_summary.clone()}</span>
                    <span>"Green tree: " {walk.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Result: " {walk.inspect_result_summary.clone().unwrap_or_else(|| "Unavailable".to_string())}</span>
                </div>
                <section class="onecalc-inspect-shell__overview-deck" data-role="inspect-overview-deck">
                    <article class="onecalc-inspect-shell__overview-card" data-role="inspect-overview-source">
                        <div class="onecalc-inspect-shell__eyebrow">"Source formula"</div>
                        <strong>{walk.raw_entered_cell_text.clone()}</strong>
                        <p>"Keep source and current result nearby while reading the semantic walk."</p>
                    </article>
                    <article class="onecalc-inspect-shell__overview-card" data-role="inspect-overview-result">
                        <div class="onecalc-inspect-shell__eyebrow">"Current result"</div>
                        <strong>{walk.inspect_result_summary.clone().unwrap_or_else(|| "Unavailable".to_string())}</strong>
                        <p>{parse_status.clone()}</p>
                    </article>
                    <article class="onecalc-inspect-shell__overview-card" data-role="inspect-overview-provenance">
                        <div class="onecalc-inspect-shell__eyebrow">"Provenance posture"</div>
                        <strong>{provenance_summary.clone()}</strong>
                        <p>{summary.blocked_reason.clone().unwrap_or_else(|| "No blocked dimension recorded".to_string())}</p>
                    </article>
                </section>
            </header>

            <div class="onecalc-inspect-shell__body">
                <div class="onecalc-inspect-shell__column onecalc-inspect-shell__column--left">
                    <section class="onecalc-inspect-shell__source-stack" data-panel="inspect-source-stack">
                        <div class="onecalc-inspect-shell__panel-header">
                            <div>
                                <div class="onecalc-inspect-shell__section-accent"></div>
                                <div class="onecalc-inspect-shell__eyebrow">"Orientation"</div>
                                <h2>"Source and result"</h2>
                            </div>
                        </div>
                        <div class="onecalc-inspect-shell__source-card">
                            <div class="onecalc-inspect-shell__source-label">"Cell entry"</div>
                            <pre class="onecalc-inspect-shell__source">{walk.raw_entered_cell_text.clone()}</pre>
                        </div>
                        <div class="onecalc-inspect-shell__source-card" data-role="inspect-result-anchor">
                            <div class="onecalc-inspect-shell__source-label">"Current result"</div>
                            <strong>{walk.inspect_result_summary.clone().unwrap_or_else(|| "Unavailable".to_string())}</strong>
                        </div>
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
                                        <header class="onecalc-inspect-shell__retained-context-header">
                                            <div>
                                                <div class="onecalc-inspect-shell__eyebrow">"Retained discrepancy"</div>
                                                <h3>"Artifact context"</h3>
                                            </div>
                                            <div class="onecalc-inspect-shell__retained-context-badges">
                                                <span data-role="inspect-retained-comparison-status">{context.comparison_status.clone()}</span>
                                                <span data-role="inspect-retained-value-match">
                                                    {match context.value_match {
                                                        Some(true) => "value matched".to_string(),
                                                        Some(false) => "value diverged".to_string(),
                                                        None => "value unknown".to_string(),
                                                    }}
                                                </span>
                                                <span data-role="inspect-retained-display-match">
                                                    {match context.display_match {
                                                        Some(true) => "display matched".to_string(),
                                                        Some(false) => "display diverged".to_string(),
                                                        None => "display unknown".to_string(),
                                                    }}
                                                </span>
                                                <span data-role="inspect-retained-replay-equivalent">
                                                    {match context.replay_equivalent {
                                                        Some(true) => "replay equivalent".to_string(),
                                                        Some(false) => "replay diverged".to_string(),
                                                        None => "replay unknown".to_string(),
                                                    }}
                                                </span>
                                            </div>
                                        </header>
                                        <div data-role="inspect-retained-artifact-id">
                                            "Artifact: "
                                            {context.artifact_id.clone()}
                                        </div>
                                        <div data-role="inspect-retained-case-id">
                                            "Case: "
                                            {context.case_id.clone()}
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
                                    </section>
                                }
                            })}
                    </section>
                </div>

                <div class="onecalc-inspect-shell__column onecalc-inspect-shell__column--walk">
                    <section class="onecalc-inspect-shell__walk-cluster" data-panel="inspect-walk">
                        <div class="onecalc-inspect-shell__panel-header">
                            <div>
                                <div class="onecalc-inspect-shell__section-accent"></div>
                                <div class="onecalc-inspect-shell__eyebrow">"Walk"</div>
                                <h2>"Formula Walk"</h2>
                            </div>
                        </div>
                        <div class="onecalc-inspect-shell__walk-intro" data-role="inspect-walk-intro">
                            "This central surface is the semantic reading path. Use it to understand structure and value flow before dropping into retained replay evidence."
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
                </div>

                <div class="onecalc-inspect-shell__column onecalc-inspect-shell__column--summary">
                    <section class="onecalc-inspect-shell__summary-cluster" data-panel="inspect-summary">
                        <div class="onecalc-inspect-shell__panel-header">
                            <div>
                                <div class="onecalc-inspect-shell__section-accent"></div>
                                <div class="onecalc-inspect-shell__eyebrow">"Summary"</div>
                                <h2>"Semantic and replay context"</h2>
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
                    {if summary.comparison_records.is_empty() {
                        view! { <></> }.into_any()
                    } else {
                        view! {
                            <section class="onecalc-inspect-shell__comparison-board" data-role="inspect-comparison-board">
                                <div class="onecalc-inspect-shell__panel-header">
                                    <div>
                                        <div class="onecalc-inspect-shell__eyebrow">"Replay diff"</div>
                                        <h3>"Comparison families"</h3>
                                    </div>
                                </div>
                                <div class="onecalc-inspect-shell__comparison-grid">
                                    {summary
                                        .comparison_records
                                        .into_iter()
                                        .map(|record| view! { <InspectComparisonRecordCard record=record /> })
                                        .collect_view()}
                                </div>
                            </section>
                        }
                        .into_any()
                    }}
                    {if summary.explain_records.is_empty() {
                        view! { <></> }.into_any()
                    } else {
                        view! {
                            <section class="onecalc-inspect-shell__explain-board" data-role="inspect-explain-board">
                                <div class="onecalc-inspect-shell__panel-header">
                                    <div>
                                        <div class="onecalc-inspect-shell__eyebrow">"Replay explain"</div>
                                        <h3>"Machine-readable explain evidence"</h3>
                                    </div>
                                </div>
                                <div class="onecalc-inspect-shell__explain-grid">
                                    {summary
                                        .explain_records
                                        .into_iter()
                                        .map(|record| view! { <InspectExplainRecordCard record=record /> })
                                        .collect_view()}
                                </div>
                            </section>
                        }
                        .into_any()
                    }}
                    {summary
                        .retained_artifact_context
                        .as_ref()
                        .map(|context| {
                            if context.upstream_gap_summary.is_empty() {
                                view! { <></> }.into_any()
                            } else {
                                view! {
                                    <section class="onecalc-inspect-shell__gap-board" data-role="inspect-gap-board">
                                        <div class="onecalc-inspect-shell__panel-header">
                                            <div>
                                                <div class="onecalc-inspect-shell__eyebrow">"Coverage"</div>
                                                <h3>"Projection and lane gaps"</h3>
                                            </div>
                                        </div>
                                        <ul data-role="inspect-retained-upstream-gap-summary">
                                            {context
                                                .upstream_gap_summary
                                                .iter()
                                                .map(|item| view! { <li>{item.clone()}</li> })
                                                .collect_view()}
                                        </ul>
                                    </section>
                                }
                                .into_any()
                            }
                        })}
                    <div class="onecalc-inspect-shell__summary-grid">
                        <InspectSummaryCard title="Parse" value=parse_status data_panel="inspect-parse" />
                        <InspectSummaryCard title="Bind" value=bind_summary data_panel="inspect-bind" />
                        <InspectSummaryCard title="Eval" value=eval_summary data_panel="inspect-eval" />
                        <InspectSummaryCard title="Provenance" value=provenance_summary data_panel="inspect-provenance" />
                    </div>
                    </section>
                </div>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::FormulaWalkNodeState;
    use crate::adapters::oxfml::{BindSummary, EvalSummary, ParseSummary, ProvenanceSummary};
    use crate::services::inspect_mode::{
        InspectComparisonRecordView, InspectExplainRecordView, InspectFormulaWalkNodeView,
        InspectRetainedArtifactContextView, InspectViewModel,
    };
    use crate::ui::panels::inspect::{build_inspect_summary_cluster, build_inspect_walk_cluster};

    #[test]
    fn inspect_shell_renders_walk_and_replay_content() {
        let view_model = InspectViewModel {
            scenario_label: "Blocked discrepancy".to_string(),
            truth_source_label: "live-backed".to_string(),
            host_profile_summary: "Windows desktop preview".to_string(),
            packet_kind_summary: "verification publication".to_string(),
            capability_floor_summary: "Inspect with retained replay evidence".to_string(),
            mode_availability_summary: "Explore / Inspect / Workbench".to_string(),
            trace_summary: Some("Replay compare captured for retained discrepancy".to_string()),
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
            retained_artifact_context: Some(InspectRetainedArtifactContextView {
                artifact_id: "artifact-1".to_string(),
                case_id: "case-1".to_string(),
                comparison_status: "blocked".to_string(),
                value_match: Some(true),
                display_match: Some(false),
                replay_equivalent: Some(false),
                discrepancy_summary: Some("excel lane unavailable".to_string()),
                bundle_report_path: Some("target/onecalc-verification/example".to_string()),
                xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
                display_comparison_summary: Some(
                    "Display divergence (effective_display_text): OxFml 6 vs Excel $6.00"
                        .to_string(),
                ),
                upstream_gap_summary: vec![
                    "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side".to_string(),
                ],
                comparison_records: vec![
                    InspectComparisonRecordView {
                        mismatch_kind: "effective_display_text".to_string(),
                        severity: "informational".to_string(),
                        view_family: Some("effective_display_text".to_string()),
                        family_label: "Effective display".to_string(),
                        status_label: "Display divergence".to_string(),
                        summary: "comparison view values diverged".to_string(),
                        left_value_repr: Some("6".to_string()),
                        right_value_repr: Some("$6.00".to_string()),
                        detail: Some("comparison view values diverged".to_string()),
                        is_projection_gap: false,
                    },
                    InspectComparisonRecordView {
                        mismatch_kind: "projection_coverage_gap".to_string(),
                        severity: "coverage".to_string(),
                        view_family: Some("formatting_view".to_string()),
                        family_label: "Formatting".to_string(),
                        status_label: "Coverage gap".to_string(),
                        summary: "comparison view family `formatting_view` is missing on one side".to_string(),
                        left_value_repr: None,
                        right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                        detail: Some("comparison view family `formatting_view` is missing on one side".to_string()),
                        is_projection_gap: true,
                    },
                ],
                explain_records: vec![InspectExplainRecordView {
                    query_id: Some("explain-01".to_string()),
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: "informational".to_string(),
                    view_family: Some("effective_display_text".to_string()),
                    family_label: "Effective display".to_string(),
                    summary: "comparison diverged on `effective_display_text`".to_string(),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                }],
            }),
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
        assert!(html.contains("artifact-1"));
        assert!(html.contains("data-role=\"inspect-retained-bundle-path\""));
        assert!(html.contains("data-role=\"inspect-retained-xml-source\""));
        assert!(html.contains("data-role=\"inspect-retained-display-comparison\""));
        assert!(html.contains("data-role=\"inspect-retained-upstream-gap-summary\""));
        assert!(html.contains("data-role=\"inspect-comparison-board\""));
        assert!(html.contains("data-role=\"inspect-comparison-record\""));
        assert!(html.contains("Display divergence"));
        assert!(html.contains("Coverage gap"));
        assert!(html.contains("data-role=\"inspect-explain-board\""));
        assert!(html.contains("data-role=\"inspect-explain-record\""));
        assert!(html.contains("comparison diverged on `effective_display_text`"));
        assert!(html.contains("data-role=\"inspect-retained-value-match\""));
        assert!(html.contains("data-role=\"inspect-retained-display-match\""));
        assert!(html.contains("data-role=\"inspect-retained-replay-equivalent\""));
    }
}
