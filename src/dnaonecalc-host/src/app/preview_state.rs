use crate::domain::ids::FormulaSpaceId;
use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, ProgrammaticComparisonStatus,
};
use crate::services::retained_artifacts::{
    import_programmatic_artifact, RetainedArtifactImportRequest,
};
use crate::state::{FormulaSpaceState, OneCalcHostState};
use crate::test_support::sample_editor_document;

pub fn preview_host_state() -> OneCalcHostState {
    let formula_space_id = FormulaSpaceId::new("preview-space");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state
        .workspace_shell
        .pinned_formula_space_ids
        .insert(formula_space_id.clone());

    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
    formula_space.effective_display_summary = Some("3".to_string());
    formula_space.latest_evaluation_summary = Some("Number".to_string());
    state.formula_spaces.insert(formula_space);

    import_programmatic_artifact(
        &mut state,
        RetainedArtifactImportRequest {
            formula_space_id: formula_space_id.clone(),
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
            formula_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                "preview-artifact-blocked",
                "preview-case-2",
                ProgrammaticComparisonStatus::Blocked,
            ),
            discrepancy_summary: Some("excel lane unavailable".to_string()),
        },
    );

    state
}
