use leptos::prelude::*;

use crate::services::programmatic_testing::ProgrammaticComparisonStatus;
use crate::services::retained_artifacts::{
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use crate::services::workbench_mode::{WorkbenchComparisonRecordView, WorkbenchExplainRecordView};
use crate::ui::panels::workbench::{
    WorkbenchActionsClusterViewModel, WorkbenchCatalogClusterViewModel,
    WorkbenchEvidenceClusterViewModel, WorkbenchLineageClusterViewModel,
    WorkbenchOutcomeClusterViewModel,
};

#[component]
fn WorkbenchComparisonRecordCard(record: WorkbenchComparisonRecordView) -> impl IntoView {
    view! {
        <article
            class="onecalc-workbench-shell__comparison-record"
            data-role="workbench-comparison-record"
            data-family=record.view_family.clone().unwrap_or_else(|| record.mismatch_kind.clone())
            data-projection-gap=if record.is_projection_gap { "true" } else { "false" }
        >
            <header class="onecalc-workbench-shell__comparison-record-header">
                <div>
                    <div class="onecalc-workbench-shell__eyebrow">"Compare"</div>
                    <h4 data-role="workbench-comparison-family">{record.family_label.clone()}</h4>
                </div>
                <div class="onecalc-workbench-shell__comparison-record-badges">
                    <span data-role="workbench-comparison-kind">{record.status_label.clone()}</span>
                    <span data-role="workbench-comparison-severity">{record.severity.clone()}</span>
                </div>
            </header>
            <p data-role="workbench-comparison-summary">{record.summary.clone()}</p>
            <div class="onecalc-workbench-shell__comparison-lane">
                <div class="onecalc-workbench-shell__comparison-lane-card" data-role="workbench-comparison-left">
                    <span class="onecalc-workbench-shell__comparison-label">"OxFml"</span>
                    <strong>{record.left_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
                <div class="onecalc-workbench-shell__comparison-lane-card" data-role="workbench-comparison-right">
                    <span class="onecalc-workbench-shell__comparison-label">"Excel / replay"</span>
                    <strong>{record.right_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
            </div>
        </article>
    }
}

#[component]
fn WorkbenchExplainRecordCard(record: WorkbenchExplainRecordView) -> impl IntoView {
    view! {
        <article class="onecalc-workbench-shell__explain-record" data-role="workbench-explain-record">
            <header class="onecalc-workbench-shell__comparison-record-header">
                <div>
                    <div class="onecalc-workbench-shell__eyebrow">"Explain"</div>
                    <h4>{record.family_label.clone()}</h4>
                </div>
                {record.query_id.as_ref().map(|query_id| view! {
                    <span data-role="workbench-explain-query-id">{query_id.clone()}</span>
                })}
            </header>
            <p data-role="workbench-explain-summary">{record.summary.clone()}</p>
            <div class="onecalc-workbench-shell__comparison-lane">
                <div class="onecalc-workbench-shell__comparison-lane-card">
                    <span class="onecalc-workbench-shell__comparison-label">"OxFml"</span>
                    <strong>{record.left_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
                <div class="onecalc-workbench-shell__comparison-lane-card">
                    <span class="onecalc-workbench-shell__comparison-label">"Excel / replay"</span>
                    <strong>{record.right_value_repr.unwrap_or_else(|| "Unavailable".to_string())}</strong>
                </div>
            </div>
        </article>
    }
}

fn comparison_badge_text(value: Option<bool>, yes: &str, no: &str, unknown: &str) -> String {
    match value {
        Some(true) => yes.to_string(),
        Some(false) => no.to_string(),
        None => unknown.to_string(),
    }
}

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
    let (artifact_id, set_artifact_id) = signal(String::new());
    let (case_id, set_case_id) = signal(String::new());
    let (discrepancy_summary, set_discrepancy_summary) = signal(String::new());
    let (verification_bundle_json, set_verification_bundle_json) = signal(String::new());

    view! {
        <section class="onecalc-workbench-shell" data-screen="workbench">
            <header class="onecalc-workbench-shell__header">
                <div>
                    <div class="onecalc-workbench-shell__eyebrow">"Workbench"</div>
                    <h1>"Twin Oracle Workbench"</h1>
                </div>
                <p class="onecalc-workbench-shell__lead">
                    "Browse retained discrepancies, spend replay family evidence directly, and move from imported Excel comparison bundles into semantic dissection without changing tools."
                </p>
                <div class="onecalc-workbench-shell__meta">
                    <span data-role="workbench-scenario-label">{outcome.scenario_label.clone()}</span>
                    <span data-role="workbench-truth-source">{outcome.truth_source_label.clone()}</span>
                    <span data-role="workbench-host-profile">{outcome.host_profile_summary.clone()}</span>
                    <span data-role="workbench-comparison-status">
                        {outcome.comparison_status_summary.clone().unwrap_or_else(|| "pending".to_string())}
                    </span>
                </div>
            </header>

            <div class="onecalc-workbench-shell__body">
                <section class="onecalc-workbench-shell__outcome-card" data-panel="workbench-outcome">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Outcome"</div>
                            <h2>"Retained discrepancy state"</h2>
                        </div>
                        <span class="onecalc-workbench-shell__outcome-chip">{outcome_summary.clone()}</span>
                    </div>
                    <div class="onecalc-workbench-shell__hero-outcome" data-role="workbench-outcome-hero">
                        <div class="onecalc-workbench-shell__comparison-label">"Recommended action"</div>
                        <strong>{outcome.recommended_action.clone()}</strong>
                    </div>
                    <div class="onecalc-workbench-shell__score-grid">
                        <div class="onecalc-workbench-shell__score-card" data-role="workbench-visible-output-score">
                            <span class="onecalc-workbench-shell__comparison-label">"Visible output"</span>
                            <strong>{comparison_badge_text(outcome.visible_output_match, "matched", "diverged", "unknown")}</strong>
                        </div>
                        <div class="onecalc-workbench-shell__score-card" data-role="workbench-replay-equivalent-score">
                            <span class="onecalc-workbench-shell__comparison-label">"Replay equivalence"</span>
                            <strong>{comparison_badge_text(outcome.replay_equivalent, "equivalent", "diverged", "unknown")}</strong>
                        </div>
                        <div class="onecalc-workbench-shell__score-card" data-role="workbench-capability-floor">
                            <span class="onecalc-workbench-shell__comparison-label">"Capability floor"</span>
                            <strong>{outcome.capability_floor_summary.clone()}</strong>
                        </div>
                    </div>
                    {outcome.retained_artifact_id.as_ref().map(|artifact_id| view! {
                        <div data-role="retained-artifact-id">"Artifact: " {artifact_id.clone()}</div>
                    })}
                    {outcome.retained_case_id.as_ref().map(|case_id| view! {
                        <div data-role="workbench-retained-case-id">"Case: " {case_id.clone()}</div>
                    })}
                </section>

                <section class="onecalc-workbench-shell__evidence-card" data-panel="workbench-evidence">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Evidence"</div>
                            <h2>"Imported and live context"</h2>
                        </div>
                    </div>
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

                <section class="onecalc-workbench-shell__compare-card" data-panel="workbench-compare">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Replay diff"</div>
                            <h2>"Comparison families"</h2>
                        </div>
                    </div>
                    <div class="onecalc-workbench-shell__comparison-grid" data-role="workbench-comparison-grid">
                        {if evidence.comparison_records.is_empty() {
                            view! { <div>"No comparison family evidence yet"</div> }.into_any()
                        } else {
                            view! {
                                {evidence
                                    .comparison_records
                                    .into_iter()
                                    .map(|record| view! { <WorkbenchComparisonRecordCard record=record /> })
                                    .collect_view()}
                            }
                            .into_any()
                        }}
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
                        }
                        .into_any()
                    }}
                </section>

                <section class="onecalc-workbench-shell__replay-card" data-panel="workbench-replay">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Replay explain"</div>
                            <h2>"Explain evidence"</h2>
                        </div>
                    </div>
                    <div class="onecalc-workbench-shell__comparison-grid" data-role="workbench-explain-grid">
                        {if evidence.explain_records.is_empty() {
                            view! { <div>"No explain records yet"</div> }.into_any()
                        } else {
                            view! {
                                {evidence
                                    .explain_records
                                    .into_iter()
                                    .map(|record| view! { <WorkbenchExplainRecordCard record=record /> })
                                    .collect_view()}
                            }
                            .into_any()
                        }}
                    </div>
                </section>

                <section class="onecalc-workbench-shell__lineage-card" data-panel="workbench-lineage">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Lineage"</div>
                            <h2>"Acquisition path"</h2>
                        </div>
                    </div>
                    <ul>
                        {lineage
                            .lineage_items
                            .into_iter()
                            .map(|item| view! { <li>{item}</li> })
                            .collect_view()}
                    </ul>
                </section>

                <section class="onecalc-workbench-shell__actions-card" data-panel="workbench-actions">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Actions"</div>
                            <h2>"Next moves"</h2>
                        </div>
                    </div>
                    <div class="onecalc-workbench-shell__hero-outcome">
                        <div class="onecalc-workbench-shell__comparison-label">"Primary move"</div>
                        <strong>{actions.recommended_action.clone()}</strong>
                    </div>
                    <ul>
                        {actions
                            .action_items
                            .into_iter()
                            .map(|item| view! { <li>{item}</li> })
                            .collect_view()}
                    </ul>
                </section>

                <section class="onecalc-workbench-shell__catalog-card" data-panel="workbench-catalog">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Catalog"</div>
                            <h2>"Retained cases"</h2>
                        </div>
                    </div>
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
                                        class="onecalc-workbench-shell__catalog-item"
                                        data-role="retained-catalog-item"
                                        data-artifact-id=item.artifact_id.clone()
                                        data-comparison-status=item.comparison_status.clone()
                                        data-open=if item.is_open { "true" } else { "false" }
                                    >
                                        <div class="onecalc-workbench-shell__catalog-item-header">
                                            <div>
                                                <strong data-role="retained-catalog-label">{item.artifact_id.clone()}</strong>
                                                <div data-role="retained-catalog-case-id">{item.case_id.clone()}</div>
                                            </div>
                                            <span data-role="retained-catalog-status">{item.comparison_status.clone()}</span>
                                        </div>
                                        {item.xml_source_summary.as_ref().map(|summary| view! {
                                            <div data-role="retained-catalog-xml-source">{summary.clone()}</div>
                                        })}
                                        {item.discrepancy_summary.as_ref().map(|summary| view! {
                                            <div data-role="retained-catalog-discrepancy">{summary.clone()}</div>
                                        })}
                                        <div class="onecalc-workbench-shell__catalog-item-actions">
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
                                        </div>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                </section>

                <section class="onecalc-workbench-shell__import-surface" data-role="retained-import-surface">
                    <div class="onecalc-workbench-shell__panel-header">
                        <div>
                            <div class="onecalc-workbench-shell__eyebrow">"Import"</div>
                            <h2>"Retained discrepancy form"</h2>
                        </div>
                    </div>
                    <p data-role="retained-import-description">
                        "Create a retained discrepancy record using the same artifact and case vocabulary as the verification bundle path. This is for narrow manual intake, not a separate evidence format."
                    </p>
                    <div class="onecalc-workbench-shell__import-field-group">
                        <label class="onecalc-workbench-shell__import-label" for="retained-import-artifact-id">
                            "Artifact id"
                        </label>
                        <input
                            id="retained-import-artifact-id"
                            data-role="retained-import-artifact-id"
                            prop:value=artifact_id
                            on:input=move |ev| {
                                set_artifact_id.set(event_target_value(&ev));
                            }
                        />
                        <span class="onecalc-workbench-shell__import-help">
                            "Stable retained artifact identity used in Workbench and Inspect."
                        </span>
                    </div>
                    <div class="onecalc-workbench-shell__import-field-group">
                        <label class="onecalc-workbench-shell__import-label" for="retained-import-case-id">
                            "Case id"
                        </label>
                        <input
                            id="retained-import-case-id"
                            data-role="retained-import-case-id"
                            prop:value=case_id
                            on:input=move |ev| {
                                set_case_id.set(event_target_value(&ev));
                            }
                        />
                        <span class="onecalc-workbench-shell__import-help">
                            "The verification case identity that produced the evidence."
                        </span>
                    </div>
                    <div class="onecalc-workbench-shell__import-field-group">
                        <label class="onecalc-workbench-shell__import-label" for="retained-import-summary">
                            "Discrepancy summary"
                        </label>
                        <input
                            id="retained-import-summary"
                            data-role="retained-import-summary"
                            prop:value=discrepancy_summary
                            on:input=move |ev| {
                                set_discrepancy_summary.set(event_target_value(&ev));
                            }
                        />
                    </div>
                    <div class="onecalc-workbench-shell__import-outcome-guide" data-role="retained-import-outcome-guide">
                        <div>"Choose the outcome lane that best matches the retained evidence shape."</div>
                        <div class="onecalc-workbench-shell__import-buttons">
                            <button
                                type="button"
                                data-role="retained-import-submit"
                                data-import-status="matched"
                                on:click=move |_| {
                                    if let Some(callback) = on_import_retained_artifact.as_ref() {
                                        let artifact_id = artifact_id.get_untracked();
                                        let case_id = case_id.get_untracked();
                                        if artifact_id.trim().is_empty() || case_id.trim().is_empty() {
                                            return;
                                        }
                                        callback.run(ManualRetainedArtifactImportRequest {
                                            artifact_id,
                                            case_id,
                                            comparison_status: ProgrammaticComparisonStatus::Matched,
                                            discrepancy_summary: {
                                                let summary = discrepancy_summary.get_untracked();
                                                if summary.trim().is_empty() { None } else { Some(summary) }
                                            },
                                        });
                                    }
                                }
                            >
                                "Import as matched"
                            </button>
                            <button
                                type="button"
                                data-role="retained-import-submit"
                                data-import-status="mismatched"
                                on:click=move |_| {
                                    if let Some(callback) = on_import_retained_artifact.as_ref() {
                                        let artifact_id = artifact_id.get_untracked();
                                        let case_id = case_id.get_untracked();
                                        if artifact_id.trim().is_empty() || case_id.trim().is_empty() {
                                            return;
                                        }
                                        callback.run(ManualRetainedArtifactImportRequest {
                                            artifact_id,
                                            case_id,
                                            comparison_status: ProgrammaticComparisonStatus::Mismatched,
                                            discrepancy_summary: {
                                                let summary = discrepancy_summary.get_untracked();
                                                if summary.trim().is_empty() { None } else { Some(summary) }
                                            },
                                        });
                                    }
                                }
                            >
                                "Import as mismatched"
                            </button>
                            <button
                                type="button"
                                data-role="retained-import-submit"
                                data-import-status="blocked"
                                on:click=move |_| {
                                    if let Some(callback) = on_import_retained_artifact.as_ref() {
                                        let artifact_id = artifact_id.get_untracked();
                                        let case_id = case_id.get_untracked();
                                        if artifact_id.trim().is_empty() || case_id.trim().is_empty() {
                                            return;
                                        }
                                        callback.run(ManualRetainedArtifactImportRequest {
                                            artifact_id,
                                            case_id,
                                            comparison_status: ProgrammaticComparisonStatus::Blocked,
                                            discrepancy_summary: {
                                                let summary = discrepancy_summary.get_untracked();
                                                if summary.trim().is_empty() { None } else { Some(summary) }
                                            },
                                        });
                                    }
                                }
                            >
                                "Import as blocked"
                            </button>
                        </div>
                    </div>
                    <div class="onecalc-workbench-shell__bundle-import-surface" data-role="verification-bundle-import-surface">
                        <h3>"Import Verification Bundle"</h3>
                        <p data-role="verification-bundle-import-description">
                            "Paste the exact verification-bundle-report.json content emitted by the CLI path. OneCalc will import the cases and preserve XML source, comparison-family evidence, explain rows, and upstream gap context."
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
                </section>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbench_shell_renders_outcome_replay_and_import_surfaces() {
        let html = view! {
            <WorkbenchShell
                outcome=WorkbenchOutcomeClusterViewModel {
                    scenario_label: "Mismatch · Retained discrepancy".to_string(),
                    truth_source_label: "live-backed".to_string(),
                    host_profile_summary: "Windows desktop preview".to_string(),
                    capability_floor_summary: "Workbench with retained artifacts".to_string(),
                    outcome_summary: Some("Mismatched".to_string()),
                    recommended_action: "Review projection coverage gaps before claiming semantic mismatch".to_string(),
                    retained_artifact_id: Some("artifact-1".to_string()),
                    retained_case_id: Some("case-1".to_string()),
                    comparison_status_summary: Some("mismatched".to_string()),
                    visible_output_match: Some(false),
                    replay_equivalent: Some(false),
                }
                evidence=WorkbenchEvidenceClusterViewModel {
                    raw_entered_cell_text: "=SUM(1,2)".to_string(),
                    evidence_summary: Some("green=green-1, diagnostics=1".to_string()),
                    retained_discrepancy_summary: Some("dna=1 excel=2".to_string()),
                    trace_summary: Some("Preview trace captured for retained mismatch".to_string()),
                    imported_bundle_summary: Some("Imported bundle: target/example".to_string()),
                    xml_source_summary: Some("Input @ Input!A1 | format $#,##0.00".to_string()),
                    display_comparison_summary: Some(
                        "Display divergence (effective_display_text): OxFml 6 vs Excel $6.00"
                            .to_string(),
                    ),
                    upstream_gap_summary: vec![
                        "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
                            .to_string(),
                    ],
                    comparison_records: vec![
                        WorkbenchComparisonRecordView {
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
                    ],
                    explain_records: vec![WorkbenchExplainRecordView {
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
                }
                lineage=WorkbenchLineageClusterViewModel {
                    lineage_items: vec!["Scenario opened".to_string(), "Evaluation captured".to_string()],
                }
                actions=WorkbenchActionsClusterViewModel {
                    action_items: vec!["Retain snapshot".to_string(), "Prepare handoff".to_string()],
                    recommended_action: "Review projection coverage gaps before claiming semantic mismatch".to_string(),
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
        assert!(html.contains("data-role=\"workbench-outcome-hero\""));
        assert!(html.contains("data-role=\"workbench-visible-output-score\""));
        assert!(html.contains("data-role=\"workbench-replay-equivalent-score\""));
        assert!(html.contains("data-role=\"workbench-comparison-grid\""));
        assert!(html.contains("data-role=\"workbench-comparison-record\""));
        assert!(html.contains("Display divergence"));
        assert!(html.contains("data-role=\"workbench-explain-grid\""));
        assert!(html.contains("data-role=\"workbench-explain-record\""));
        assert!(html.contains("data-role=\"workbench-imported-bundle-summary\""));
        assert!(html.contains("data-role=\"workbench-xml-source-summary\""));
        assert!(html.contains("data-role=\"workbench-display-comparison-summary\""));
        assert!(html.contains("data-role=\"retained-catalog-item\""));
        assert!(html.contains("data-role=\"retained-catalog-open\""));
        assert!(html.contains("data-role=\"retained-catalog-open-inspect\""));
        assert!(html.contains("data-open=\"true\""));
        assert!(html.contains("data-role=\"retained-import-surface\""));
        assert!(html.contains("data-role=\"verification-bundle-import-surface\""));
        assert!(html.contains("data-role=\"verification-bundle-import-json\""));
        assert!(html.contains("data-role=\"verification-bundle-import-submit\""));
        assert!(html.contains("data-role=\"retained-import-artifact-id\""));
        assert!(html.contains("data-role=\"retained-import-case-id\""));
        assert!(html.contains("data-role=\"retained-import-summary\""));
        assert!(html.contains("data-role=\"retained-import-submit\""));
        assert!(html.contains("data-import-status=\"blocked\""));
        assert!(html.contains("Prepare handoff"));
    }
}
