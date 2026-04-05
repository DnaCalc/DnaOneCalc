use crate::domain::ids::FormulaSpaceId;
use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, ProgrammaticArtifactCatalogEntry,
    ProgrammaticComparisonStatus,
};
use crate::state::{OneCalcHostState, RetainedArtifactRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedArtifactImportRequest {
    pub formula_space_id: FormulaSpaceId,
    pub catalog_entry: ProgrammaticArtifactCatalogEntry,
    pub discrepancy_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualRetainedArtifactImportRequest {
    pub artifact_id: String,
    pub case_id: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub discrepancy_summary: Option<String>,
}

pub fn import_programmatic_artifact(
    state: &mut OneCalcHostState,
    request: RetainedArtifactImportRequest,
) {
    let record = RetainedArtifactRecord {
        artifact_id: request.catalog_entry.artifact_id.clone(),
        case_id: request.catalog_entry.case_id,
        formula_space_id: request.formula_space_id,
        comparison_status: request.catalog_entry.comparison_status,
        open_mode_hint: request.catalog_entry.open_mode_hint,
        discrepancy_summary: request.discrepancy_summary,
    };

    state
        .retained_artifacts
        .catalog
        .insert(record.artifact_id.clone(), record);
}

pub fn import_programmatic_artifacts(
    state: &mut OneCalcHostState,
    requests: impl IntoIterator<Item = RetainedArtifactImportRequest>,
) {
    for request in requests {
        import_programmatic_artifact(state, request);
    }
}

pub fn open_retained_artifact_by_id(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> Result<(), String> {
    let Some(record) = state.retained_artifacts.catalog.get(artifact_id) else {
        return Err(format!("retained artifact not found: {artifact_id}"));
    };

    state.retained_artifacts.open_artifact_id = Some(record.artifact_id.clone());
    state.workspace_shell.active_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.active_mode = match record.open_mode_hint {
        crate::services::programmatic_testing::ProgrammaticOpenModeHint::Inspect => {
            crate::state::AppMode::Inspect
        }
        crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench => {
            crate::state::AppMode::Workbench
        }
    };
    Ok(())
}

pub fn open_retained_artifact_in_inspect_by_id(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> Result<(), String> {
    let Some(record) = state.retained_artifacts.catalog.get(artifact_id) else {
        return Err(format!("retained artifact not found: {artifact_id}"));
    };

    state.retained_artifacts.open_artifact_id = Some(record.artifact_id.clone());
    state.workspace_shell.active_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(record.formula_space_id.clone());
    state.active_formula_space_view.active_mode = crate::state::AppMode::Inspect;
    Ok(())
}

pub fn import_manual_artifact_for_active_formula_space(
    state: &mut OneCalcHostState,
    request: ManualRetainedArtifactImportRequest,
) -> Result<(), String> {
    let Some(formula_space_id) = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state.active_formula_space_view.selected_formula_space_id.clone())
    else {
        return Err("no active formula space for retained artifact import".to_string());
    };

    let artifact_id = request.artifact_id;
    import_programmatic_artifact(
        state,
        RetainedArtifactImportRequest {
            formula_space_id,
            catalog_entry: build_programmatic_artifact_catalog_entry(
                artifact_id.clone(),
                request.case_id,
                request.comparison_status,
            ),
            discrepancy_summary: request.discrepancy_summary,
        },
    );
    open_retained_artifact_by_id(state, &artifact_id)
}

pub fn active_retained_artifact<'a>(
    state: &'a OneCalcHostState,
) -> Option<&'a RetainedArtifactRecord> {
    let artifact_id = state.retained_artifacts.open_artifact_id.as_ref()?;
    state.retained_artifacts.catalog.get(artifact_id)
}

pub fn retained_artifacts_for_formula_space<'a>(
    state: &'a OneCalcHostState,
    formula_space_id: &FormulaSpaceId,
) -> Vec<&'a RetainedArtifactRecord> {
    let mut records = state
        .retained_artifacts
        .catalog
        .values()
        .filter(|record| &record.formula_space_id == formula_space_id)
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };

    #[test]
    fn importing_programmatic_artifact_populates_catalog() {
        let mut state = OneCalcHostState::default();

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );

        assert!(state.retained_artifacts.catalog.contains_key("artifact-1"));
    }

    #[test]
    fn importing_multiple_programmatic_artifacts_populates_sorted_formula_space_catalog() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifacts(
            &mut state,
            [
                RetainedArtifactImportRequest {
                    formula_space_id: FormulaSpaceId::new("space-1"),
                    catalog_entry: ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-2".to_string(),
                        case_id: "case-2".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Matched,
                        open_mode_hint: ProgrammaticOpenModeHint::Inspect,
                    },
                    discrepancy_summary: None,
                },
                RetainedArtifactImportRequest {
                    formula_space_id: FormulaSpaceId::new("space-1"),
                    catalog_entry: ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Mismatched,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                    discrepancy_summary: Some("dna=1 excel=2".to_string()),
                },
            ],
        );

        let records = retained_artifacts_for_formula_space(&state, &FormulaSpaceId::new("space-1"));
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].artifact_id, "artifact-1");
        assert_eq!(records[1].artifact_id, "artifact-2");
    }

    #[test]
    fn opening_retained_artifact_routes_shell_to_its_formula_space_and_mode() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Blocked,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        open_retained_artifact_by_id(&mut state, "artifact-1").expect("artifact should open");

        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref().map(|id| id.as_str()),
            Some("space-1")
        );
        assert_eq!(state.active_formula_space_view.active_mode, crate::state::AppMode::Workbench);
    }

    #[test]
    fn opening_retained_artifact_in_inspect_routes_shell_to_inspect_mode() {
        let mut state = OneCalcHostState::default();
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: FormulaSpaceId::new("space-1"),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );

        open_retained_artifact_in_inspect_by_id(&mut state, "artifact-1")
            .expect("artifact should open in inspect");

        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref().map(|id| id.as_str()),
            Some("space-1")
        );
        assert_eq!(state.active_formula_space_view.active_mode, crate::state::AppMode::Inspect);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-1")
        );
    }

    #[test]
    fn importing_manual_artifact_uses_active_formula_space_and_opens_it() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());

        import_manual_artifact_for_active_formula_space(
            &mut state,
            ManualRetainedArtifactImportRequest {
                artifact_id: "artifact-9".to_string(),
                case_id: "case-9".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Blocked,
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        )
        .expect("manual artifact should import");

        let record = state
            .retained_artifacts
            .catalog
            .get("artifact-9")
            .expect("artifact imported");
        assert_eq!(record.formula_space_id, formula_space_id);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-9")
        );
        assert_eq!(state.active_formula_space_view.active_mode, crate::state::AppMode::Workbench);
    }
}
