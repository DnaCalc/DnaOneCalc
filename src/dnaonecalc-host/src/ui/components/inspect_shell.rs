use leptos::prelude::*;

use crate::ui::panels::inspect::{
    InspectSummaryClusterViewModel, InspectWalkClusterViewModel,
};

#[component]
fn InspectWalkNode(node: crate::services::inspect_mode::InspectFormulaWalkNodeView) -> impl IntoView {
    let state_label = format!("{:?}", node.state);
    let value_preview = node.value_preview.clone().unwrap_or_else(|| "Unavailable".to_string());

    view! {
        <li class="onecalc-inspect-shell__walk-node" data-node-id=node.node_id.clone()>
            <div class="onecalc-inspect-shell__walk-node-header">
                <span class="onecalc-inspect-shell__walk-node-label">{node.label.clone()}</span>
                <span class="onecalc-inspect-shell__walk-node-state">{state_label}</span>
            </div>
            <div class="onecalc-inspect-shell__walk-node-value">{value_preview}</div>
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
fn InspectSummaryCard(title: &'static str, value: String, data_panel: &'static str) -> impl IntoView {
    view! {
        <section class="onecalc-inspect-shell__summary-card" data-panel=data_panel>
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
        .map(|value| format!("vars={}, refs={}", value.variable_count, value.reference_count))
        .unwrap_or_else(|| "Unavailable".to_string());
    let eval_summary = summary
        .eval_summary
        .as_ref()
        .map(|value| format!("steps={}, duration={}", value.step_count, value.duration_text))
        .unwrap_or_else(|| "Unavailable".to_string());
    let provenance_summary = summary
        .provenance_summary
        .as_ref()
        .map(|value| value.profile_summary.clone())
        .unwrap_or_else(|| "Unavailable".to_string());

    view! {
        <section class="onecalc-inspect-shell" data-screen="inspect">
            <header class="onecalc-inspect-shell__header">
                <h1>"Semantic Inspect"</h1>
                <div class="onecalc-inspect-shell__meta">
                    <span>"Green tree: " {walk.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Result: " {walk.inspect_result_summary.unwrap_or_else(|| "Unavailable".to_string())}</span>
                </div>
            </header>

            <div class="onecalc-inspect-shell__body">
                <section class="onecalc-inspect-shell__walk-cluster" data-panel="inspect-walk">
                    <h2>"Formula Walk"</h2>
                    <pre class="onecalc-inspect-shell__source">{walk.raw_entered_cell_text}</pre>
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
                    <h2>"Summaries"</h2>
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
    use crate::adapters::oxfml::{BindSummary, EvalSummary, ParseSummary, ProvenanceSummary};
    use crate::services::inspect_mode::{InspectFormulaWalkNodeView, InspectViewModel};
    use crate::ui::panels::inspect::{
        build_inspect_summary_cluster, build_inspect_walk_cluster,
    };
    use crate::adapters::oxfml::FormulaWalkNodeState;

    #[test]
    fn inspect_shell_renders_walk_and_summary_content() {
        let view_model = InspectViewModel {
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
            retained_artifact_context: Some(crate::services::inspect_mode::InspectRetainedArtifactContextView {
                artifact_id: "artifact-1".to_string(),
                case_id: "case-1".to_string(),
                comparison_status: "blocked".to_string(),
                discrepancy_summary: Some("excel lane unavailable".to_string()),
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
        assert!(html.contains("=LET(x,1,x)"));
        assert!(html.contains("data-panel=\"inspect-walk\""));
        assert!(html.contains("data-node-id=\"node-1\""));
        assert!(html.contains("Parse"));
        assert!(html.contains("Valid"));
        assert!(html.contains("data-role=\"inspect-retained-context\""));
        assert!(html.contains("Artifact: "));
        assert!(html.contains("artifact-1"));
        assert!(html.contains("excel lane unavailable"));
    }
}
