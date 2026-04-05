use leptos::prelude::*;

use crate::ui::panels::workbench::{
    WorkbenchEvidenceClusterViewModel, WorkbenchOutcomeClusterViewModel,
};

#[component]
pub fn WorkbenchShell(
    outcome: WorkbenchOutcomeClusterViewModel,
    evidence: WorkbenchEvidenceClusterViewModel,
) -> impl IntoView {
    let outcome_summary = outcome
        .outcome_summary
        .clone()
        .unwrap_or_else(|| "No comparison outcome yet".to_string());
    let evidence_summary = evidence
        .evidence_summary
        .clone()
        .unwrap_or_else(|| "No retained evidence yet".to_string());

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

                <section class="onecalc-workbench-shell__evidence-card" data-panel="workbench-evidence">
                    <h2>"Evidence"</h2>
                    <pre>{evidence.raw_entered_cell_text}</pre>
                    <div>{evidence_summary}</div>
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
            />
        }
        .to_html();

        assert!(html.contains("Twin Oracle Workbench"));
        assert!(html.contains("Retain and compare"));
        assert!(html.contains("green=green-1"));
    }
}
