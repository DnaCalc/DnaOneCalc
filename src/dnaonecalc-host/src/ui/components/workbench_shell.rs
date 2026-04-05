use leptos::prelude::*;

use crate::ui::panels::workbench::{
    WorkbenchActionsClusterViewModel, WorkbenchEvidenceClusterViewModel,
    WorkbenchLineageClusterViewModel, WorkbenchOutcomeClusterViewModel,
};

#[component]
pub fn WorkbenchShell(
    outcome: WorkbenchOutcomeClusterViewModel,
    evidence: WorkbenchEvidenceClusterViewModel,
    lineage: WorkbenchLineageClusterViewModel,
    actions: WorkbenchActionsClusterViewModel,
) -> impl IntoView {
    let outcome_summary = outcome
        .outcome_summary
        .clone()
        .unwrap_or_else(|| "No comparison outcome yet".to_string());
    let evidence_summary = evidence
        .evidence_summary
        .clone()
        .unwrap_or_else(|| "No retained evidence yet".to_string());
    let compare_summary = format!(
        "Compare against baseline: {}",
        outcome
            .outcome_summary
            .clone()
            .unwrap_or_else(|| "pending".to_string())
    );
    let replay_summary = lineage
        .lineage_items
        .last()
        .cloned()
        .unwrap_or_else(|| "No replay state yet".to_string());

    view! {
        <section class="onecalc-workbench-shell" data-screen="workbench">
            <header class="onecalc-workbench-shell__header">
                <h1>"Twin Oracle Workbench"</h1>
                <div class="onecalc-workbench-shell__meta">
                    <span>"Outcome: " {outcome_summary}</span>
                </div>
            </header>

            <div class="onecalc-workbench-shell__body">
                <section class="onecalc-workbench-shell__outcome-card" data-panel="workbench-outcome">
                    <h2>"Outcome"</h2>
                    <div>{outcome.outcome_summary.unwrap_or_else(|| "Unavailable".to_string())}</div>
                    <div>{outcome.recommended_action}</div>
                </section>

                <section class="onecalc-workbench-shell__compare-card" data-panel="workbench-compare">
                    <h2>"Compare"</h2>
                    <div>{compare_summary}</div>
                </section>

                <section class="onecalc-workbench-shell__replay-card" data-panel="workbench-replay">
                    <h2>"Replay"</h2>
                    <div>{replay_summary}</div>
                </section>

                <section class="onecalc-workbench-shell__evidence-card" data-panel="workbench-evidence">
                    <h2>"Evidence"</h2>
                    <pre class="onecalc-workbench-shell__evidence-source">{evidence.raw_entered_cell_text}</pre>
                    <div>{evidence_summary}</div>
                </section>

                <section class="onecalc-workbench-shell__lineage-card" data-panel="workbench-lineage">
                    <h2>"Lineage"</h2>
                    <ul>
                        {lineage
                            .lineage_items
                            .into_iter()
                            .map(|item| view! { <li>{item}</li> })
                            .collect_view()}
                    </ul>
                </section>

                <section class="onecalc-workbench-shell__actions-card" data-panel="workbench-actions">
                    <h2>"Actions"</h2>
                    <div>{actions.recommended_action}</div>
                    <ul>
                        {actions
                            .action_items
                            .into_iter()
                            .map(|item| view! { <li>{item}</li> })
                            .collect_view()}
                    </ul>
                </section>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbench_shell_renders_outcome_and_evidence_cards() {
        let html = view! {
            <WorkbenchShell
                outcome=WorkbenchOutcomeClusterViewModel {
                    outcome_summary: Some("Number".to_string()),
                    recommended_action: "Retain and compare".to_string(),
                }
                evidence=WorkbenchEvidenceClusterViewModel {
                    raw_entered_cell_text: "=SUM(1,2)".to_string(),
                    evidence_summary: Some("green=green-1, diagnostics=1".to_string()),
                }
                lineage=WorkbenchLineageClusterViewModel {
                    lineage_items: vec!["Scenario opened".to_string(), "Evaluation captured".to_string()],
                }
                actions=WorkbenchActionsClusterViewModel {
                    action_items: vec!["Retain snapshot".to_string(), "Prepare handoff".to_string()],
                    recommended_action: "Retain and compare".to_string(),
                }
            />
        }
        .to_html();

        assert!(html.contains("Twin Oracle Workbench"));
        assert!(html.contains("Retain and compare"));
        assert!(html.contains("green=green-1"));
        assert!(html.contains("data-panel=\"workbench-lineage\""));
        assert!(html.contains("data-panel=\"workbench-compare\""));
        assert!(html.contains("data-panel=\"workbench-replay\""));
        assert!(html.contains("Prepare handoff"));
    }
}
