use std::fs;
use std::path::{Path, PathBuf};

use oxfml_core::consumer::runtime::SingleFormulaHost;
use oxfml_core::format::oxfml_en_us_locale_context;
use oxfml_core::interface::{
    LibraryContextProvider, TypedContextQueryBundle, TypedContextQueryFamily,
};
use oxfml_core::EvaluationBackend;
use oxfunc_core::value::EvalValue;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::adapters::oxvba::{OxvbaSourceModule, OxvbaSourceProject};
use crate::services::programmatic_testing::{
    ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
};
use crate::services::vba_host::{VbaHostRuntime, VbaProjectAssociation, VbaUdfAdmissionStatus};

const DEFAULT_CASE_ID: &str = "VBA-UDF-T001";
const DEFAULT_FORMULA: &str = "=AddThem(2,3)";
const DEFAULT_PROJECT_NAME: &str = "DnaVbaFixture";
const DEFAULT_MODULE_NAME: &str = "Module1";
const DEFAULT_APPLICATION_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_ASSOCIATION_ID: &str = "vba-assoc-1";
const DEFAULT_PROJECT_REF: &str = "memory:AddThem";
const DEFAULT_ORACLE_REF: &str =
    "OxXlPlay/states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaUdfVerificationRequest {
    pub case_id: String,
    pub formula: String,
    pub project_name: String,
    pub module_name: String,
    pub module_source: String,
    pub vba_project_path: Option<String>,
    pub application_version: String,
    pub association_id: String,
    pub project_ref: String,
    pub excel_oracle_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VbaUdfVerificationReport {
    pub schema_id: String,
    pub case_id: String,
    pub formula: String,
    pub output_root: String,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub dna_value: Value,
    pub excel_oracle_value: Option<Value>,
    pub value_match: Option<bool>,
    pub excel_oracle_ref: Option<String>,
    pub oracle_status: String,
    pub invocation_record_path: String,
    pub capability_snapshot_path: String,
    pub blocked_matrix_path: String,
    pub artifact_open_mode_hint: ProgrammaticOpenModeHint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VbaUdfOracleObservation {
    pub comparison_value: Value,
    pub effective_display_text: Option<String>,
    pub formula_text: Option<String>,
    pub source_metadata: Value,
}

pub fn default_vba_udf_verification_request() -> VbaUdfVerificationRequest {
    let mut request = default_vba_udf_evaluation_request();
    request.excel_oracle_ref = Some(DEFAULT_ORACLE_REF.to_string());
    request
}

pub fn default_vba_udf_evaluation_request() -> VbaUdfVerificationRequest {
    VbaUdfVerificationRequest {
        case_id: DEFAULT_CASE_ID.to_string(),
        formula: DEFAULT_FORMULA.to_string(),
        project_name: DEFAULT_PROJECT_NAME.to_string(),
        module_name: DEFAULT_MODULE_NAME.to_string(),
        module_source: default_addthem_module_source(),
        vba_project_path: None,
        application_version: DEFAULT_APPLICATION_VERSION.to_string(),
        association_id: DEFAULT_ASSOCIATION_ID.to_string(),
        project_ref: DEFAULT_PROJECT_REF.to_string(),
        excel_oracle_ref: None,
    }
}

pub fn default_vba_udf_output_root() -> Result<PathBuf, String> {
    Ok(repo_root()?
        .join("target")
        .join("onecalc-verification")
        .join("vba-udf")
        .join(DEFAULT_CASE_ID))
}

pub fn load_vba_udf_module_source(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read VBA module source `{}`: {error}",
            path.display()
        )
    })
}

pub fn run_vba_udf_verification(
    request: &VbaUdfVerificationRequest,
    output_root: impl AsRef<Path>,
) -> Result<VbaUdfVerificationReport, String> {
    validate_request(request)?;
    let repo_root = repo_root()?;
    let output_root = output_root.as_ref();
    fs::create_dir_all(output_root).map_err(|error| {
        format!(
            "failed to create VBA UDF verification output root `{}`: {error}",
            output_root.display()
        )
    })?;

    let oracle = if let Some(oracle_ref) = request.excel_oracle_ref.as_deref() {
        let oracle_path = resolve_repo_relative_path(&repo_root, oracle_ref);
        let oracle = load_oracle_observation(&oracle_path)?;
        verify_oracle_matches_formula(request, &oracle)?;
        Some(oracle)
    } else {
        None
    };

    let association =
        VbaProjectAssociation::workspace_source(&request.association_id, &request.project_ref);
    let runtime = if let Some(project_path) = request.vba_project_path.as_deref() {
        VbaHostRuntime::load_project_path_for_evaluation(
            association,
            resolve_repo_relative_path(&repo_root, project_path),
            &request.application_version,
        )?
    } else if request.excel_oracle_ref.is_some() {
        VbaHostRuntime::load_source_project(
            association,
            OxvbaSourceProject {
                project_name: request.project_name.clone(),
                modules: vec![OxvbaSourceModule {
                    module_name: request.module_name.clone(),
                    source: request.module_source.clone(),
                }],
            },
            &request.application_version,
            request.excel_oracle_ref.clone(),
        )?
    } else {
        VbaHostRuntime::load_source_project_for_evaluation(
            association,
            OxvbaSourceProject {
                project_name: request.project_name.clone(),
                modules: vec![OxvbaSourceModule {
                    module_name: request.module_name.clone(),
                    source: request.module_source.clone(),
                }],
            },
            &request.application_version,
        )?
    };

    let locale = oxfml_en_us_locale_context();
    let query_bundle =
        TypedContextQueryBundle::new(None, None, Some(&locale), Some(46000.0), Some(0.5))
            .with_host_function_provider(Some(&runtime));
    let mut host = SingleFormulaHost::new(&request.case_id, &request.formula);
    let result = host
        .recalc_with_interfaces(
            EvaluationBackend::OxFuncBacked,
            query_bundle,
            Some(&runtime),
        )
        .map_err(|error| format!("VBA UDF formula evaluation failed: {error}"))?;

    if !result
        .typed_query_bundle_spec
        .families
        .contains(&TypedContextQueryFamily::HostFunction)
    {
        return Err(
            "VBA UDF formula evaluation did not exercise the OxFml host-function family"
                .to_string(),
        );
    }

    let dna_value = eval_value_to_oracle_shape(&result.published_worksheet_value);
    let value_match = oracle
        .as_ref()
        .map(|oracle| values_match(&dna_value, &oracle.comparison_value));
    let comparison_status = match value_match {
        Some(true) | None => ProgrammaticComparisonStatus::Matched,
        Some(false) => ProgrammaticComparisonStatus::Mismatched,
    };
    let oracle_status = match value_match {
        Some(true) => "matched",
        Some(false) => "mismatched",
        None => "not_requested",
    };

    let invocation_record_path = output_root.join("vba-udf-invocation-record.json");
    let capability_snapshot_path = output_root.join("vba-udf-capability-snapshot.json");
    let blocked_matrix_path = output_root.join("vba-udf-blocked-matrix.json");

    let invocation_record = json!({
        "schema_id": "dnaonecalc.vba_udf_invocation.v1",
        "case_id": request.case_id,
        "association": runtime.association(),
        "formula_call": request.formula,
        "excel_oracle_ref": request.excel_oracle_ref,
        "oracle_status": oracle_status,
        "signature": runtime.registrations().first().map(registration_to_json),
        "type_map": {
            "status": runtime.registrations().first().map(|registration| registration.type_map_status.as_str()).unwrap_or("none"),
            "argument_rules": ["number-to-double"],
            "return_rule": "double-to-number"
        },
        "dna_result": {
            "comparison_value": dna_value,
            "raw_value_debug": format!("{:?}", result.published_worksheet_value)
        },
        "excel_oracle": oracle,
        "verdict": oracle_status
    });
    write_json_file(&invocation_record_path, &invocation_record)?;

    let capability_snapshot = json!({
        "schema_id": "dnaonecalc.vba_udf_capability_snapshot.v1",
        "case_id": request.case_id,
        "oxvba_project_name": runtime.association().project_identity.as_deref().unwrap_or(&request.project_name),
        "application_root": {
            "name": "Application",
            "members": ["Version"],
            "broader_excel_application_surface": "not-admitted"
        },
        "admitted_udf_count": runtime.registrations().len(),
        "candidate_count": runtime.candidates().len(),
        "candidates": runtime.candidates().iter().map(candidate_to_json).collect::<Vec<_>>(),
        "library_context_snapshot": library_snapshot_to_json(&runtime.current_snapshot()),
        "typed_matrix": {
            "admitted": ["VBA-UDF-T001"],
            "blocked": ["VBA-UDF-T002", "VBA-UDF-T003", "VBA-UDF-T004", "VBA-UDF-T005", "VBA-UDF-T006", "VBA-UDF-T007", "VBA-UDF-T008"]
        }
    });
    write_json_file(&capability_snapshot_path, &capability_snapshot)?;
    write_json_file(&blocked_matrix_path, &blocked_matrix_rows())?;

    let report = VbaUdfVerificationReport {
        schema_id: "dnaonecalc.vba_udf_verification_report.v1".to_string(),
        case_id: request.case_id.clone(),
        formula: request.formula.clone(),
        output_root: display_repo_relative(output_root, &repo_root),
        comparison_status,
        dna_value: invocation_record["dna_result"]["comparison_value"].clone(),
        excel_oracle_value: oracle
            .as_ref()
            .map(|oracle| oracle.comparison_value.clone()),
        value_match,
        excel_oracle_ref: request.excel_oracle_ref.clone(),
        oracle_status: oracle_status.to_string(),
        invocation_record_path: display_repo_relative(&invocation_record_path, &repo_root),
        capability_snapshot_path: display_repo_relative(&capability_snapshot_path, &repo_root),
        blocked_matrix_path: display_repo_relative(&blocked_matrix_path, &repo_root),
        artifact_open_mode_hint: if value_match.unwrap_or(true) {
            ProgrammaticOpenModeHint::Inspect
        } else {
            ProgrammaticOpenModeHint::Workbench
        },
    };
    write_json_file(
        output_root.join("vba-udf-verification-report.json"),
        &report,
    )?;
    Ok(report)
}

fn default_addthem_module_source() -> String {
    concat!(
        "Public Function AddThem(val1 As Double, val2 As Double) As Double\n",
        "  AddThem = val1 + val2\n",
        "End Function\n"
    )
    .to_string()
}

fn validate_request(request: &VbaUdfVerificationRequest) -> Result<(), String> {
    if request.case_id.trim().is_empty() {
        return Err("verify-vba-udf requires a non-empty case id".to_string());
    }
    if request.formula.trim().is_empty() {
        return Err("verify-vba-udf requires a non-empty formula".to_string());
    }
    if request.module_source.trim().is_empty() {
        return Err("verify-vba-udf requires a non-empty VBA module source".to_string());
    }
    Ok(())
}

fn load_oracle_observation(path: &Path) -> Result<VbaUdfOracleObservation, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read VBA UDF oracle `{}`: {error}",
            path.display()
        )
    })?;
    let json: Value = serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse VBA UDF oracle `{}`: {error}",
            path.display()
        )
    })?;
    let comparison_value = json
        .get("comparison_views")
        .and_then(Value::as_array)
        .and_then(|views| {
            views.iter().find_map(|view| {
                (view.get("view_family").and_then(Value::as_str) == Some("comparison_value"))
                    .then(|| view.get("value"))
                    .flatten()
            })
        })
        .and_then(normalize_oracle_comparison_value)
        .ok_or_else(|| {
            format!(
                "VBA UDF oracle `{}` does not contain a numeric comparison_value view",
                path.display()
            )
        })?;
    let effective_display_text = json
        .get("comparison_views")
        .and_then(Value::as_array)
        .and_then(|views| {
            views.iter().find_map(|view| {
                (view.get("view_family").and_then(Value::as_str) == Some("effective_display_text"))
                    .then(|| view.get("value").and_then(Value::as_str))
                    .flatten()
                    .map(ToOwned::to_owned)
            })
        });
    let source_metadata = json.get("source_metadata").cloned().unwrap_or(Value::Null);
    let formula_text = source_metadata
        .get("formula_text")
        .or_else(|| {
            source_metadata
                .get("execution_outcome")
                .and_then(|outcome| outcome.get("entered_cell_text"))
        })
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Ok(VbaUdfOracleObservation {
        comparison_value,
        effective_display_text,
        formula_text,
        source_metadata,
    })
}

fn normalize_oracle_comparison_value(value: &Value) -> Option<Value> {
    if value.get("kind").and_then(Value::as_str) == Some("number") {
        let number = value.get("value").or_else(|| value.get("number"))?;
        return Some(json!({ "kind": "number", "number": number }));
    }
    let nested = value.get("value")?;
    if nested.get("kind").and_then(Value::as_str) == Some("number") {
        let number = nested.get("number").or_else(|| nested.get("value"))?;
        return Some(json!({ "kind": "number", "number": number }));
    }
    None
}

fn verify_oracle_matches_formula(
    request: &VbaUdfVerificationRequest,
    oracle: &VbaUdfOracleObservation,
) -> Result<(), String> {
    if let Some(formula_text) = &oracle.formula_text {
        if !formula_text.eq_ignore_ascii_case(&request.formula) {
            return Err(format!(
                "VBA UDF oracle formula `{formula_text}` does not match requested formula `{}`",
                request.formula
            ));
        }
    }
    Ok(())
}

fn eval_value_to_oracle_shape(value: &EvalValue) -> Value {
    match value {
        EvalValue::Number(number) => json!({ "kind": "number", "number": number }),
        other => json!({ "kind": "unsupported", "debug": format!("{other:?}") }),
    }
}

fn values_match(left: &Value, right: &Value) -> bool {
    match (
        left.get("kind").and_then(Value::as_str),
        right.get("kind").and_then(Value::as_str),
    ) {
        (Some("number"), Some("number")) => {
            let left = left.get("number").and_then(Value::as_f64);
            let right = right.get("number").and_then(Value::as_f64);
            match (left, right) {
                (Some(left), Some(right)) => (left - right).abs() <= f64::EPSILON,
                _ => false,
            }
        }
        _ => left == right,
    }
}

fn registration_to_json(registration: &crate::services::vba_host::VbaUdfRegistration) -> Value {
    json!({
        "project": &registration.signature.project_name,
        "module": &registration.signature.module_name,
        "function": &registration.signature.procedure_name,
        "formula_name": &registration.formula_name,
        "stable_host_call_id": &registration.stable_host_call_id,
        "parameters": registration.signature.parameters.iter().map(|parameter| {
            json!({
                "name": &parameter.name,
                "vba_type": format!("{:?}", parameter.vba_type),
                "evidence": format!("{:?}", parameter.evidence)
            })
        }).collect::<Vec<_>>(),
        "return_type": format!("{:?}", registration.signature.return_type),
        "return_evidence": format!("{:?}", registration.signature.evidence),
        "type_map_status": &registration.type_map_status,
        "excel_oracle_ref": &registration.excel_oracle_ref
    })
}

fn candidate_to_json(candidate: &crate::services::vba_host::VbaUdfCandidate) -> Value {
    let (status, reason) = match &candidate.admission_status {
        VbaUdfAdmissionStatus::Admitted => ("admitted", None),
        VbaUdfAdmissionStatus::Rejected { reason } => ("rejected", Some(reason.as_str())),
    };
    json!({
        "association_id": &candidate.association_id,
        "project_name": &candidate.project_name,
        "module_name": &candidate.module_name,
        "procedure_name": &candidate.procedure_name,
        "stable_host_call_id": &candidate.stable_host_call_id,
        "admission_status": status,
        "reason": reason
    })
}

fn library_snapshot_to_json(snapshot: &oxfml_core::semantics::LibraryContextSnapshot) -> Value {
    json!({
        "snapshot_id": snapshot.snapshot_id,
        "snapshot_version": snapshot.snapshot_version,
        "entries": snapshot.entries.iter().map(|entry| {
            json!({
                "surface_name": &entry.surface_name,
                "canonical_id": &entry.canonical_id,
                "surface_stable_id": &entry.surface_stable_id,
                "name_resolution_table_ref": &entry.name_resolution_table_ref,
                "semantic_trait_profile_ref": &entry.semantic_trait_profile_ref,
                "metadata_status": &entry.metadata_status,
                "admission_interface_kind": &entry.admission_interface_kind,
                "preparation_owner": &entry.preparation_owner,
                "runtime_boundary_kind": &entry.runtime_boundary_kind,
                "interface_contract_ref": &entry.interface_contract_ref,
                "registration_source_kind": format!("{:?}", entry.registration_source_kind),
                "parse_bind_state": format!("{:?}", entry.parse_bind_state),
                "semantic_plan_state": format!("{:?}", entry.semantic_plan_state),
                "runtime_capability_state": entry.runtime_capability_state.map(|state| format!("{:?}", state)),
            })
        }).collect::<Vec<_>>()
    })
}

fn blocked_matrix_rows() -> Value {
    json!([
        { "row_id": "VBA-UDF-T002", "formula_call": "=AddThem(TRUE,3)", "status": "blocked", "reason": "Excel oracle row required before admission" },
        { "row_id": "VBA-UDF-T003", "formula_call": "=AddThem(\"2\",3)", "status": "blocked", "reason": "Excel oracle row required before admission" },
        { "row_id": "VBA-UDF-T004", "formula_call": "=AddThem(\"\",3)", "status": "blocked", "reason": "Excel oracle row required before admission" },
        { "row_id": "VBA-UDF-T005", "formula_call": "=AddThem(A1,3) with blank A1", "status": "blocked", "reason": "Excel oracle row required before admission" },
        { "row_id": "VBA-UDF-T006", "formula_call": "=AddThem(A1,3) with #DIV/0! A1", "status": "blocked", "reason": "Excel oracle row required before admission" },
        { "row_id": "VBA-UDF-T007", "formula_call": "=EchoText(\"abc\")", "status": "blocked", "reason": "second scalar family requires retained Excel oracle evidence" },
        { "row_id": "VBA-UDF-T008", "formula_call": "=NotIt(TRUE)", "status": "blocked", "reason": "second scalar family requires retained Excel oracle evidence" }
    ])
}

fn write_json_file(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory `{}`: {error}",
                parent.display()
            )
        })?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        format!(
            "failed to serialize JSON artifact `{}`: {error}",
            path.display()
        )
    })?;
    fs::write(path, bytes).map_err(|error| {
        format!(
            "failed to write JSON artifact `{}`: {error}",
            path.display()
        )
    })
}

fn resolve_repo_relative_path(repo_root: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else if let Ok(current_dir) = std::env::current_dir() {
        let current_candidate = current_dir.join(&candidate);
        if current_candidate.exists() {
            return current_candidate;
        }
        let onecalc_candidate = dnaonecalc_repo_root()
            .map(|root| root.join(&candidate))
            .unwrap_or_else(|_| current_candidate.clone());
        if onecalc_candidate.exists() {
            return onecalc_candidate;
        }
        repo_root.join(candidate)
    } else {
        repo_root.join(candidate)
    }
}

fn repo_root() -> Result<PathBuf, String> {
    let onecalc_root = dnaonecalc_repo_root()?;
    onecalc_root.parent().map(Path::to_path_buf).ok_or_else(|| {
        format!(
            "failed to resolve DnaCalc root from `{}`",
            onecalc_root.display()
        )
    })
}

fn dnaonecalc_repo_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "failed to resolve DnaOneCalc repository root from `{}`",
                manifest_dir.display()
            )
        })
}

fn display_repo_relative(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vba_udf_verification_matches_retained_excel_oracle() {
        let output_root = repo_root()
            .expect("repo root")
            .join("target")
            .join("onecalc-verification-tests")
            .join("vba-udf-t001");
        let _ = fs::remove_dir_all(&output_root);
        let request = default_vba_udf_verification_request();

        let report =
            run_vba_udf_verification(&request, &output_root).expect("VBA UDF verification");

        assert_eq!(
            report.comparison_status,
            ProgrammaticComparisonStatus::Matched
        );
        assert_eq!(report.value_match, Some(true));
        assert!(output_root.join("vba-udf-invocation-record.json").exists());
        assert!(output_root
            .join("vba-udf-capability-snapshot.json")
            .exists());
        assert!(output_root.join("vba-udf-blocked-matrix.json").exists());
    }

    #[test]
    fn vba_udf_evaluation_runs_custom_formula_without_oracle() {
        let output_root = repo_root()
            .expect("repo root")
            .join("DnaOneCalc")
            .join("target")
            .join("onecalc-verification-tests")
            .join("vba-udf-multiply-three-no-oracle");
        let _ = fs::remove_dir_all(&output_root);
        let mut request = default_vba_udf_evaluation_request();
        request.case_id = "VBA-UDF-MULTIPLY-THREE".to_string();
        request.formula = "=MultiplyThree(2, 3, 4)".to_string();
        request.module_source = concat!(
            "Public Function MultiplyThree(val1 As Double, val2 As Double, val3 As Double) As Double\n",
            "  MultiplyThree = val1 * val2 * val3\n",
            "End Function\n"
        )
        .to_string();

        let report = run_vba_udf_verification(&request, &output_root).expect("VBA UDF evaluation");

        assert_eq!(report.oracle_status, "not_requested");
        assert_eq!(report.value_match, None);
        assert_eq!(
            report.dna_value,
            json!({ "kind": "number", "number": 24.0 })
        );
    }

    #[test]
    fn vba_udf_oracle_loader_accepts_oxxlplay_normalized_shape() {
        let oracle = load_oracle_observation(&resolve_repo_relative_path(
            &repo_root().expect("repo root"),
            DEFAULT_ORACLE_REF,
        ))
        .expect("oracle");

        assert_eq!(
            oracle.comparison_value,
            json!({ "kind": "number", "number": 5.0 })
        );
        assert_eq!(oracle.effective_display_text.as_deref(), Some("5"));
        assert_eq!(oracle.formula_text.as_deref(), Some(DEFAULT_FORMULA));
    }
}
