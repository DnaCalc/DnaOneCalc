use crate::domain::ids::FormulaSpaceId;
use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, ProgrammaticComparisonStatus,
};
use crate::services::retained_artifacts::{
    import_programmatic_artifact, RetainedArtifactImportRequest,
};
use crate::state::{
    FormulaArrayPreviewState, FormulaSpaceContextState, FormulaSpaceState, OneCalcHostState,
    ProjectionTruthSource,
};
use crate::test_support::{
    array_editor_document, blocked_editor_document, diagnostic_editor_document,
    sample_editor_document,
};

pub fn preview_host_state() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();

    let success_space_id = FormulaSpaceId::new("preview-success");
    let diagnostic_space_id = FormulaSpaceId::new("preview-diagnostic");
    let array_space_id = FormulaSpaceId::new("preview-array");

    state.workspace_shell.active_formula_space_id = Some(success_space_id.clone());
    state.workspace_shell.open_formula_space_order.extend([
        success_space_id.clone(),
        diagnostic_space_id.clone(),
        array_space_id.clone(),
    ]);
    state
        .workspace_shell
        .pinned_formula_space_ids
        .insert(success_space_id.clone());

    state
        .formula_spaces
        .insert(success_formula_space(success_space_id.clone()));
    state
        .formula_spaces
        .insert(diagnostic_formula_space(diagnostic_space_id.clone()));
    state
        .formula_spaces
        .insert(array_formula_space(array_space_id.clone()));

    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id: success_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "preview-artifact-mismatch",
                "preview-case-1",
                ProgrammaticComparisonStatus::Mismatched,
            ),
            discrepancy_summary: Some("dna=3 excel=4".to_string()),
        },
    );
    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id: diagnostic_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "preview-artifact-blocked",
                "preview-case-2",
                ProgrammaticComparisonStatus::Blocked,
            ),
            discrepancy_summary: Some("excel lane unavailable".to_string()),
        },
    );
    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id: array_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "preview-artifact-match",
                "preview-case-3",
                ProgrammaticComparisonStatus::Matched,
            ),
            discrepancy_summary: Some("retained replay matches preview baseline".to_string()),
        },
    );

    state
}

fn success_formula_space(formula_space_id: FormulaSpaceId) -> FormulaSpaceState {
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.effective_display_summary = Some("3".to_string());
    formula_space.latest_evaluation_summary = Some("Number · 3".to_string());
    formula_space.context = FormulaSpaceContextState {
        scenario_label: "Success · SUM result".to_string(),
        host_profile: "Preview host shell".to_string(),
        packet_kind: "seeded demo state".to_string(),
        capability_floor: "Explore + Inspect".to_string(),
        mode_availability: "Explore / Inspect / Workbench".to_string(),
        truth_source: ProjectionTruthSource::LocalFallback,
        trace_summary: Some("Seeded scenario until a live OxFml edit refresh arrives".to_string()),
        blocked_reason: None,
    };
    formula_space
}

fn diagnostic_formula_space(formula_space_id: FormulaSpaceId) -> FormulaSpaceState {
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,)");
    formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
    formula_space.effective_display_summary = Some("Input incomplete".to_string());
    formula_space.latest_evaluation_summary =
        Some("Diagnostic · Missing trailing argument".to_string());
    formula_space.context = FormulaSpaceContextState {
        scenario_label: "Diagnostic · Missing argument".to_string(),
        host_profile: "Preview host shell".to_string(),
        packet_kind: "seeded demo state".to_string(),
        capability_floor: "Explore + Inspect".to_string(),
        mode_availability: "Explore / Inspect / Workbench".to_string(),
        truth_source: ProjectionTruthSource::LocalFallback,
        trace_summary: Some("Seeded scenario until a live OxFml edit refresh arrives".to_string()),
        blocked_reason: None,
    };
    formula_space
}

fn array_formula_space(formula_space_id: FormulaSpaceId) -> FormulaSpaceState {
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SEQUENCE(2,2)");
    formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,2)"));
    formula_space.effective_display_summary = Some("{1,2;3,4}".to_string());
    formula_space.latest_evaluation_summary = Some("Array · 2x2 dynamic result".to_string());
    formula_space.array_preview = Some(FormulaArrayPreviewState {
        label: "2x2 spill preview".to_string(),
        rows: vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ],
        truncated: false,
    });
    formula_space.context = FormulaSpaceContextState {
        scenario_label: "Array · Dynamic spill".to_string(),
        host_profile: "Preview host shell".to_string(),
        packet_kind: "seeded demo state".to_string(),
        capability_floor: "Explore + Inspect + retained replay".to_string(),
        mode_availability: "Explore / Inspect / Workbench".to_string(),
        truth_source: ProjectionTruthSource::LocalFallback,
        trace_summary: Some("Seeded scenario until a live OxFml edit refresh arrives".to_string()),
        blocked_reason: None,
    };
    formula_space
}

#[allow(dead_code)]
fn blocked_formula_space(formula_space_id: FormulaSpaceId) -> FormulaSpaceState {
    let mut formula_space = FormulaSpaceState::new(formula_space_id, "=XLOOKUP(A1,B1:B9,C1:C9)");
    formula_space.editor_document = Some(blocked_editor_document("=XLOOKUP(A1,B1:B9,C1:C9)"));
    formula_space.effective_display_summary = Some("Blocked on host lane".to_string());
    formula_space.latest_evaluation_summary =
        Some("Blocked · comparison lane unavailable".to_string());
    formula_space.context = FormulaSpaceContextState {
        scenario_label: "Blocked · Host limitation".to_string(),
        host_profile: "Preview host shell".to_string(),
        packet_kind: "seeded demo state".to_string(),
        capability_floor: "Inspect with blocked reason".to_string(),
        mode_availability: "Explore / Inspect / Workbench".to_string(),
        truth_source: ProjectionTruthSource::LocalFallback,
        trace_summary: Some(
            "Seeded blocked scenario until a live OxFml edit refresh arrives".to_string(),
        ),
        blocked_reason: Some("Excel comparison lane unavailable on this host".to_string()),
    };
    formula_space
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the fixture shape of `preview_host_state` exactly. If a future
    /// change reorders or relabels the seeded spaces and catalog entries,
    /// this test catches it — the preview fixture is documented here.
    #[test]
    fn preview_host_state_seeds_multiple_demo_scenarios() {
        let state = preview_host_state();

        // Three seeded formula spaces in a specific order.
        let open_ids: Vec<&str> = state
            .workspace_shell
            .open_formula_space_order
            .iter()
            .map(|id| id.as_str())
            .collect();
        assert_eq!(
            open_ids,
            vec!["preview-success", "preview-diagnostic", "preview-array"]
        );
        // Active space is the first one, and it's the only pinned space.
        assert_eq!(
            state
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("preview-success"),
        );
        assert!(state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("preview-success")));

        // Success space fixture content.
        let success = state
            .formula_spaces
            .get(&FormulaSpaceId::new("preview-success"))
            .expect("success space");
        assert_eq!(success.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(success.effective_display_summary.as_deref(), Some("3"));
        assert_eq!(success.context.scenario_label, "Success · SUM result");
        assert_eq!(
            success.context.truth_source,
            ProjectionTruthSource::LocalFallback
        );

        // Diagnostic space fixture content.
        let diagnostic = state
            .formula_spaces
            .get(&FormulaSpaceId::new("preview-diagnostic"))
            .expect("diagnostic space");
        assert_eq!(diagnostic.raw_entered_cell_text, "=SUM(1,)");
        assert_eq!(
            diagnostic.effective_display_summary.as_deref(),
            Some("Input incomplete")
        );

        // Array space fixture content.
        let array = state
            .formula_spaces
            .get(&FormulaSpaceId::new("preview-array"))
            .expect("array space");
        assert_eq!(array.raw_entered_cell_text, "=SEQUENCE(2,2)");
        let preview = array
            .array_preview
            .as_ref()
            .expect("array preview populated");
        assert_eq!(preview.rows.len(), 2);
        assert_eq!(preview.rows[0], vec!["1".to_string(), "2".to_string()]);

        // Retained artifact catalog carries exactly three entries keyed by
        // the expected artifact ids.
        assert_eq!(state.retained_artifacts.catalog.len(), 3);
        for expected_id in [
            "preview-artifact-mismatch",
            "preview-artifact-blocked",
            "preview-artifact-match",
        ] {
            assert!(
                state.retained_artifacts.catalog.contains_key(expected_id),
                "retained catalog missing {expected_id}",
            );
        }
    }
}
