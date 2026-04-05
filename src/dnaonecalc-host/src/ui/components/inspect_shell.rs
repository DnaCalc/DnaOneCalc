use leptos::prelude::*;

use crate::ui::panels::inspect::{
    InspectSummaryClusterViewModel, InspectWalkClusterViewModel,
};

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
    let walk_text = if walk.formula_walk_nodes.is_empty() {
        "No formula walk".to_string()
    } else {
        walk.formula_walk_nodes
            .iter()
            .map(|node| match &node.value_preview {
                Some(value) => format!("{} -> {}", node.label, value),
                None => node.label.clone(),
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };

    view! {
        <section class="onecalc-inspect-shell">
            <header class="onecalc-inspect-shell__header">
                <h1>"Semantic Inspect"</h1>
                <div class="onecalc-inspect-shell__meta">
                    <span>"Green tree: " {walk.green_tree_key.unwrap_or_else(|| "none".to_string())}</span>
                    <span>"Result: " {walk.inspect_result_summary.unwrap_or_else(|| "Unavailable".to_string())}</span>
                </div>
            </header>

            <div class="onecalc-inspect-shell__body">
                <section class="onecalc-inspect-shell__walk-cluster">
                    <h2>"Formula Walk"</h2>
                    <pre class="onecalc-inspect-shell__source">{walk.raw_entered_cell_text}</pre>
                    <div class="onecalc-inspect-shell__walk">{walk_text}</div>
                </section>

                <section class="onecalc-inspect-shell__summary-cluster">
                    <h2>"Summaries"</h2>
                    <div>"Parse: " {parse_status}</div>
                    <div>"Bind: " {bind_summary}</div>
                    <div>"Eval: " {eval_summary}</div>
                    <div>"Provenance: " {provenance_summary}</div>
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
        assert!(html.contains("LET -&gt; 1") || html.contains("LET -&gt;"));
        assert!(html.contains("Parse: "));
        assert!(html.contains("Valid"));
    }
}
