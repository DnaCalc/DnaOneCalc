use leptos::prelude::*;

use crate::services::programmatic_testing::ProgrammaticComparisonStatus;
use crate::services::retained_artifacts::{
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use crate::ui::panels::workbench::{
    WorkbenchActionsClusterViewModel, WorkbenchCatalogClusterViewModel,
    WorkbenchEvidenceClusterViewModel, WorkbenchLineageClusterViewModel,
    WorkbenchOutcomeClusterViewModel,
};

#[component]
pub fn WorkbenchShell(
    outcome: WorkbenchOutcomeClusterViewModel,
    evidence: WorkbenchEvidenceClusterViewModel,
    lineage: WorkbenchLineageClusterViewModel,
    actions: WorkbenchActionsClusterViewModel,
    catalog: WorkbenchCatalogClusterViewModel,
    #[prop(default = None)] on_open_retained_artifact: Option<Callback<String>>,
    #[prop(default = None)] on_open_retained_artifact_in_inspect: Option<Callback<String>>,
    #[prop(default = None)] on_import_retained_artifact: Option<
        Callback<ManualRetainedArtifactImportRequest>,
    >,
    #[prop(default = None)] on_import_verification_bundle: Option<
        Callback<VerificationBundleImportRequest>,
    >,
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
    let (artifact_id, set_artifact_id) = signal(String::new());
    let (case_id, set_case_id) = signal(String::new());
    let (discrepancy_summary, set_discrepancy_summary) = signal(String::new());
    let (verification_bundle_json, set_verification_bundle_json) = signal(String::new());

    view! {
        <section class="onecalc-workbench-shell" data-screen="workbench">
            <header class="onecalc-workbench-shell__header">
                <h1>"Twin Oracle Workbench"</h1>
                <div class="onecalc-workbench-shell__meta">
                    <span data-role="workbench-scenario-label">{outcome.scenario_label.clone()}</span>
                    <span data-role="workbench-truth-source">{outcome.truth_source_label.clone()}</span>
                    <span data-role="workbench-host-profile">{outcome.host_profile_summary.clone()}</span>
                    <span>"Outcome: " {outcome_summary}</span>
                </div>
            </header>

            <div class="onecalc-workbench-shell__body">
                <section class="onecalc-workbench-shell__outcome-card" data-panel="workbench-outcome">
                    <h2>"Outcome"</h2>
                    <div>{outcome.outcome_summary.unwrap_or_else(|| "Unavailable".to_string())}</div>
                    <div>{outcome.recommended_action}</div>
                    <div data-role="workbench-capability-floor">{outcome.capability_floor_summary.clone()}</div>
                    {outcome
                        .retained_artifact_id
                        .clone()
                        .map(|artifact_id| view! {
                            <div data-role="retained-artifact-id">"Artifact: " {artifact_id}</div>
                        })}
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
                    {evidence.imported_bundle_summary.as_ref().map(|summary| view! {
                        <div data-role="workbench-imported-bundle-summary">{summary.clone()}</div>
                    })}
                    {evidence.xml_source_summary.as_ref().map(|summary| view! {
                        <div data-role="workbench-xml-source-summary">{summary.clone()}</div>
                    })}
                    {evidence.display_comparison_summary.as_ref().map(|summary| view! {
                        <div data-role="workbench-display-comparison-summary">{summary.clone()}</div>
                    })}
                    {evidence.trace_summary.as_ref().map(|trace_summary| view! {
                        <div data-role="workbench-trace-summary">{trace_summary.clone()}</div>
                    })}
                    {evidence
                        .retained_discrepancy_summary
                        .clone()
                        .map(|summary| view! {
                            <div data-role="retained-discrepancy-summary">{summary}</div>
                        })}
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

                <section class="onecalc-workbench-shell__catalog-card" data-panel="workbench-catalog">
                    <h2>"Retained Catalog"</h2>
                    <ul data-role="retained-catalog-list">
                        {catalog
                            .retained_catalog_items
                            .into_iter()
                            .map(|item| {
                                let on_open_retained_artifact = on_open_retained_artifact.clone();
                                let on_open_retained_artifact_in_inspect = on_open_retained_artifact_in_inspect.clone();
                                let artifact_id = item.artifact_id.clone();
                                let inspect_artifact_id = item.artifact_id.clone();
                                view! {
                                    <li
                                        data-role="retained-catalog-item"
                                        data-artifact-id=item.artifact_id.clone()
                                        data-comparison-status=item.comparison_status.clone()
                                        data-open=if item.is_open { "true" } else { "false" }
                                    >
                                        <span data-role="retained-catalog-label">{item.artifact_id.clone()}</span>
                                        <span data-role="retained-catalog-case-id">{item.case_id.clone()}</span>
                                        <span data-role="retained-catalog-status">{item.comparison_status.clone()}</span>
                                        {item.xml_source_summary.as_ref().map(|summary| view! {
                                            <span data-role="retained-catalog-xml-source">{summary.clone()}</span>
                                        })}
                                        {item
                                            .discrepancy_summary
                                            .as_ref()
                                            .map(|summary| view! {
                                                <span data-role="retained-catalog-summary">{summary.clone()}</span>
                                            })}
                                        <button
                                            type="button"
                                            data-role="retained-catalog-open"
                                            on:click=move |_| {
                                                if let Some(callback) = on_open_retained_artifact.as_ref() {
                                                    callback.run(artifact_id.clone());
                                                }
                                            }
                                        >
                                            "Open"
                                        </button>
                                        <button
                                            type="button"
                                            data-role="retained-catalog-open-inspect"
                                            on:click=move |_| {
                                                if let Some(callback) = on_open_retained_artifact_in_inspect.as_ref() {
                                                    callback.run(inspect_artifact_id.clone());
                                                }
                                            }
                                        >
                                            "Inspect"
                                        </button>
                                    </li>
                                }
                            })
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
                    <div class="onecalc-workbench-shell__import-surface" data-role="retained-import-surface">
                        <h3>"Import Retained Artifact"</h3>
                        <p data-role="retained-import-description">
                            "Capture a retained discrepancy from the programmatic comparison path so it can be reopened in Workbench or escalated into Inspect."
                        </p>
                        <div class="onecalc-workbench-shell__import-field-group" data-role="retained-import-metadata-group">
                            <label class="onecalc-workbench-shell__import-label" for="retained-import-artifact-id">
                                "Artifact Id"
                            </label>
                            <div class="onecalc-workbench-shell__import-help">
                                "Stable retained artifact key emitted by the headless comparison path."
                            </div>
                        </div>
                        <input
                            type="text"
                            id="retained-import-artifact-id"
                            data-role="retained-import-artifact-id"
                            prop:value=artifact_id
                            on:input=move |ev| {
                                set_artifact_id.set(event_target_value(&ev));
                            }
                        />
                        <div class="onecalc-workbench-shell__import-field-group" data-role="retained-import-case-group">
                            <label class="onecalc-workbench-shell__import-label" for="retained-import-case-id">
                                "Case Id"
                            </label>
                            <div class="onecalc-workbench-shell__import-help">
                                "Programmatic corpus or replay case identifier for this retained artifact."
                            </div>
                        </div>
                        <input
                            type="text"
                            id="retained-import-case-id"
                            data-role="retained-import-case-id"
                            prop:value=case_id
                            on:input=move |ev| {
                                set_case_id.set(event_target_value(&ev));
                            }
                        />
                        <div class="onecalc-workbench-shell__import-field-group" data-role="retained-import-summary-group">
                            <label class="onecalc-workbench-shell__import-label" for="retained-import-summary">
                                "Discrepancy Summary"
                            </label>
                            <div class="onecalc-workbench-shell__import-help">
                                "Short explanation of the mismatch or blocked condition to seed triage."
                            </div>
                        </div>
                        <input
                            type="text"
                            id="retained-import-summary"
                            data-role="retained-import-summary"
                            prop:value=discrepancy_summary
                            on:input=move |ev| {
                                set_discrepancy_summary.set(event_target_value(&ev));
                            }
                        />
                        <div class="onecalc-workbench-shell__import-outcome-guide" data-role="retained-import-outcome-guide">
                            <div data-role="retained-import-outcome-matched">
                                "Matched: retain a replay packet for later inspect browsing."
                            </div>
                            <div data-role="retained-import-outcome-mismatched">
                                "Mismatched: open a discrepancy for compare-first workbench review."
                            </div>
                            <div data-role="retained-import-outcome-blocked">
                                "Blocked: capture the host or capability limitation that prevented comparison."
                            </div>
                        </div>
                        {if evidence.upstream_gap_summary.is_empty() {
                            view! { <></> }.into_any()
                        } else {
                            view! {
                                <ul data-role="retained-import-upstream-gap-summary">
                                    {evidence
                                        .upstream_gap_summary
                                        .iter()
                                        .map(|item| view! { <li>{item.clone()}</li> })
                                        .collect_view()}
                                </ul>
                            }.into_any()
                        }}
                        <div class="onecalc-workbench-shell__import-buttons">
                            {[
                                ("matched", ProgrammaticComparisonStatus::Matched),
                                ("mismatched", ProgrammaticComparisonStatus::Mismatched),
                                ("blocked", ProgrammaticComparisonStatus::Blocked),
                            ]
                                .into_iter()
                                .map(|(label, comparison_status)| {
                                    let on_import_retained_artifact = on_import_retained_artifact.clone();
                                    let artifact_id = artifact_id.clone();
                                    let case_id = case_id.clone();
                                    let discrepancy_summary = discrepancy_summary.clone();
                                    view! {
                                        <button
                                            type="button"
                                            data-role="retained-import-submit"
                                            data-import-status=label
                                            on:click=move |_| {
                                                if let Some(callback) = on_import_retained_artifact.as_ref() {
                                                    let artifact_id = artifact_id.get_untracked();
                                                    let case_id = case_id.get_untracked();
                                                    if artifact_id.is_empty() || case_id.is_empty() {
                                                        return;
                                                    }
                                                    let discrepancy_summary = discrepancy_summary.get_untracked();
                                                    callback.run(ManualRetainedArtifactImportRequest {
                                                        artifact_id,
                                                        case_id,
                                                        comparison_status,
                                                        discrepancy_summary: if discrepancy_summary.is_empty() {
                                                            None
                                                        } else {
                                                            Some(discrepancy_summary)
                                                        },
                                                    });
                                                }
                                            }
                                        >
                                            {format!("Import {}", label)}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <div class="onecalc-workbench-shell__bundle-import-surface" data-role="verification-bundle-import-surface">
                            <h3>"Import Verification Bundle"</h3>
                            <p data-role="verification-bundle-import-description">
                                "Paste the exact verification-bundle-report.json content emitted by the CLI path. OneCalc will import the cases and preserve XML and upstream gap context."
                            </p>
                            <textarea
                                class="onecalc-workbench-shell__bundle-import-textarea"
                                data-role="verification-bundle-import-json"
                                prop:value=verification_bundle_json
                                on:input=move |ev| {
                                    set_verification_bundle_json.set(event_target_value(&ev));
                                }
                            />
                            <button
                                type="button"
                                data-role="verification-bundle-import-submit"
                                on:click=move |_| {
                                    if let Some(callback) = on_import_verification_bundle.as_ref() {
                                        let report_json = verification_bundle_json.get_untracked();
                                        if !report_json.trim().is_empty() {
                                            callback.run(VerificationBundleImportRequest { report_json });
                                        }
                                    }
                                }
                            >
                                "Import verification bundle"
                            </button>
                        </div>
                    </div>
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
                    scenario_label: "Mismatch · Retained discrepancy".to_string(),
                    truth_source_label: "preview-backed".to_string(),
                    host_profile_summary: "Windows desktop preview".to_string(),
                    capability_floor_summary: "Workbench with retained artifacts".to_string(),
                    outcome_summary: Some("Number".to_string()),
                    recommended_action: "Retain and compare".to_string(),
                    retained_artifact_id: Some("artifact-1".to_string()),
                }
                evidence=WorkbenchEvidenceClusterViewModel {
                    raw_entered_cell_text: "=SUM(1,2)".to_string(),
                    evidence_summary: Some("green=green-1, diagnostics=1".to_string()),
                    retained_discrepancy_summary: Some("dna=1 excel=2".to_string()),
                    trace_summary: Some("Preview trace captured for retained mismatch".to_string()),
                    imported_bundle_summary: Some("Imported bundle: target/example".to_string()),
                    xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
                    display_comparison_summary: Some("OxFml 6 vs Excel $6.00".to_string()),
                    upstream_gap_summary: vec!["OxXlPlay missing: effective_display_text".to_string()],
                }
                lineage=WorkbenchLineageClusterViewModel {
                    lineage_items: vec!["Scenario opened".to_string(), "Evaluation captured".to_string()],
                }
                actions=WorkbenchActionsClusterViewModel {
                    action_items: vec!["Retain snapshot".to_string(), "Prepare handoff".to_string()],
                    recommended_action: "Retain and compare".to_string(),
                }
                catalog=WorkbenchCatalogClusterViewModel {
                    retained_catalog_items: vec![crate::services::workbench_mode::WorkbenchRetainedCatalogItemView {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: "mismatched".to_string(),
                        discrepancy_summary: Some("dna=1 excel=2".to_string()),
                        xml_source_summary: Some("Input @ Input!A1".to_string()),
                        is_open: true,
                    }],
                }
                on_open_retained_artifact=None
                on_open_retained_artifact_in_inspect=None
                on_import_retained_artifact=None
                on_import_verification_bundle=None
            />
        }
        .to_html();

        assert!(html.contains("Twin Oracle Workbench"));
        assert!(html.contains("data-role=\"workbench-truth-source\""));
        assert!(html.contains("Retain and compare"));
        assert!(html.contains("green=green-1"));
        assert!(html.contains("data-role=\"workbench-capability-floor\""));
        assert!(html.contains("data-role=\"workbench-trace-summary\""));
        assert!(html.contains("data-role=\"retained-artifact-id\""));
        assert!(html.contains("artifact-1"));
        assert!(html.contains("data-role=\"retained-catalog-case-id\""));
        assert!(html.contains("data-role=\"retained-discrepancy-summary\""));
        assert!(html.contains("dna=1 excel=2"));
        assert!(html.contains("data-panel=\"workbench-catalog\""));
        assert!(html.contains("data-role=\"retained-catalog-item\""));
        assert!(html.contains("data-role=\"retained-catalog-open\""));
        assert!(html.contains("data-role=\"retained-catalog-open-inspect\""));
        assert!(html.contains("data-open=\"true\""));
        assert!(html.contains("data-role=\"retained-import-surface\""));
        assert!(html.contains("data-role=\"retained-import-description\""));
        assert!(html.contains("data-role=\"retained-import-outcome-guide\""));
        assert!(html.contains("data-role=\"workbench-imported-bundle-summary\""));
        assert!(html.contains("data-role=\"workbench-xml-source-summary\""));
        assert!(html.contains("data-role=\"workbench-display-comparison-summary\""));
        assert!(html.contains("data-role=\"retained-catalog-xml-source\""));
        assert!(html.contains("data-role=\"retained-import-upstream-gap-summary\""));
        assert!(html.contains("data-role=\"verification-bundle-import-surface\""));
        assert!(html.contains("data-role=\"verification-bundle-import-json\""));
        assert!(html.contains("data-role=\"verification-bundle-import-submit\""));
        assert!(html.contains("data-role=\"retained-import-artifact-id\""));
        assert!(html.contains("data-role=\"retained-import-case-id\""));
        assert!(html.contains("data-role=\"retained-import-summary\""));
        assert!(html.contains("data-role=\"retained-import-submit\""));
        assert!(html.contains("data-import-status=\"blocked\""));
        assert!(html.contains("data-panel=\"workbench-lineage\""));
        assert!(html.contains("data-panel=\"workbench-compare\""));
        assert!(html.contains("data-panel=\"workbench-replay\""));
        assert!(html.contains("Prepare handoff"));
    }
}
