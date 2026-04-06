use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::services::programmatic_testing::{
    ProgrammaticArtifactCatalogEntry, ProgrammaticBatchPlan, ProgrammaticCapabilityProfile,
    ProgrammaticComparisonStatus, ProgrammaticFormulaCase, ProgrammaticHostProfile,
    build_programmatic_artifact_catalog_entry, build_programmatic_batch_plan,
    default_windows_excel_capability_profile, default_windows_excel_host_profile,
};
use crate::services::spreadsheet_xml::{
    SpreadsheetXmlCellExtraction, extract_cell_from_spreadsheet_xml,
};

#[cfg(feature = "oxfml-live")]
use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaEditRequest, LiveOxfmlBridge, OxfmlEditorBridge,
};
#[cfg(feature = "oxfml-live")]
use oxfml_core::FormulaChannelKind;
#[cfg(feature = "oxfml-live")]
use oxfml_core::consumer::replay::{ReplayProjectionRequest, ReplayProjectionService};
#[cfg(feature = "oxfml-live")]
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest};
#[cfg(feature = "oxfml-live")]
use oxfml_core::interface::TypedContextQueryBundle;
#[cfg(feature = "oxfml-live")]
use oxfml_core::source::FormulaSourceRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationBatchRequest {
    #[serde(default = "default_windows_excel_host_profile")]
    pub host_profile: ProgrammaticHostProfile,
    #[serde(default = "default_windows_excel_capability_profile")]
    pub capabilities: ProgrammaticCapabilityProfile,
    pub cases: Vec<ProgrammaticFormulaCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCommandCapture {
    pub command_label: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxfmlVerificationSummary {
    pub evaluation_summary: Option<String>,
    pub effective_display_summary: Option<String>,
    pub blocked_reason: Option<String>,
    pub parse_status: Option<String>,
    pub green_tree_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcelObservationSummary {
    pub observed_value_repr: Option<String>,
    pub observed_formula_repr: Option<String>,
    pub capture_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCaseReport {
    pub case_id: String,
    pub entered_cell_text: String,
    pub artifact_catalog_entry: ProgrammaticArtifactCatalogEntry,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub visible_output_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub replay_mismatch_kinds: Vec<String>,
    pub discrepancy_summary: Option<String>,
    pub oxfml_summary: OxfmlVerificationSummary,
    pub excel_summary: Option<ExcelObservationSummary>,
    pub spreadsheet_xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    pub upstream_gap_report: Option<VerificationObservationGapReport>,
    pub case_output_dir: String,
    pub scenario_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationObservationGapReport {
    pub oxfml_scope_required: Vec<String>,
    pub oxxlplay_supported_surfaces: Vec<String>,
    pub oxxlplay_missing_surfaces: Vec<String>,
    pub oxreplay_required_views: Vec<String>,
    pub oxreplay_current_bundle_views: Vec<String>,
    pub oxreplay_missing_views: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationBundleReport {
    pub bundle_id: String,
    pub output_root: String,
    pub host_profile: ProgrammaticHostProfile,
    pub capabilities: ProgrammaticCapabilityProfile,
    pub batch_plan: ProgrammaticBatchPlan,
    pub retained_artifact_catalog: Vec<ProgrammaticArtifactCatalogEntry>,
    pub case_reports: Vec<VerificationCaseReport>,
}

#[cfg(feature = "oxfml-live")]
pub trait VerificationCommandRunner {
    fn run_oxxlplay_capture(
        &self,
        scenario_path: &Path,
        output_dir: &Path,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_validate_bundle(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_diff(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String>;

    fn run_oxreplay_explain(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String>;
}

#[cfg(feature = "oxfml-live")]
#[derive(Debug, Default)]
pub struct ProcessVerificationCommandRunner;

#[cfg(feature = "oxfml-live")]
impl VerificationCommandRunner for ProcessVerificationCommandRunner {
    fn run_oxxlplay_capture(
        &self,
        scenario_path: &Path,
        output_dir: &Path,
    ) -> Result<VerificationCommandCapture, String> {
        let scenario_path = absolute_path(scenario_path)?;
        let output_dir = absolute_path(output_dir)?;
        run_command_capture(
            "oxxlplay-capture",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxXlPlay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxxlplay-cli"),
                OsString::from("--"),
                OsString::from("capture-run"),
                OsString::from("--scenario"),
                scenario_path.into_os_string(),
                OsString::from("--output-dir"),
                output_dir.into_os_string(),
            ],
        )
    }

    fn run_oxreplay_validate_bundle(
        &self,
        manifest_path: &Path,
    ) -> Result<VerificationCommandCapture, String> {
        let manifest_path = absolute_path(manifest_path)?;
        run_command_capture(
            "oxreplay-validate-bundle",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("validate-bundle"),
                OsString::from("--bundle"),
                manifest_path.into_os_string(),
                OsString::from("--format"),
                OsString::from("json"),
            ],
        )
    }

    fn run_oxreplay_diff(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String> {
        let left_path = absolute_path(left_path)?;
        let right_path = absolute_path(right_path)?;
        run_command_capture(
            "oxreplay-diff",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("diff"),
                OsString::from("--left"),
                left_path.into_os_string(),
                OsString::from("--left-kind"),
                OsString::from(left_kind),
                OsString::from("--right"),
                right_path.into_os_string(),
                OsString::from("--right-kind"),
                OsString::from(right_kind),
            ],
        )
    }

    fn run_oxreplay_explain(
        &self,
        left_path: &Path,
        left_kind: &str,
        right_path: &Path,
        right_kind: &str,
    ) -> Result<VerificationCommandCapture, String> {
        let left_path = absolute_path(left_path)?;
        let right_path = absolute_path(right_path)?;
        run_command_capture(
            "oxreplay-explain",
            "cargo",
            &[
                OsString::from("run"),
                OsString::from("--manifest-path"),
                PathBuf::from(r"C:\Work\DnaCalc\OxReplay\Cargo.toml").into_os_string(),
                OsString::from("-p"),
                OsString::from("oxreplay-dnarecalc-cli"),
                OsString::from("--"),
                OsString::from("explain"),
                OsString::from("--left"),
                left_path.into_os_string(),
                OsString::from("--left-kind"),
                OsString::from(left_kind),
                OsString::from("--right"),
                right_path.into_os_string(),
                OsString::from("--right-kind"),
                OsString::from(right_kind),
            ],
        )
    }
}

#[cfg(feature = "oxfml-live")]
pub fn load_verification_batch_request(
    input_path: impl AsRef<Path>,
) -> Result<VerificationBatchRequest, String> {
    let input_path = input_path.as_ref();
    let text = fs::read_to_string(input_path).map_err(|error| {
        format!(
            "failed to read verification batch request from `{}`: {error}",
            input_path.display()
        )
    })?;
    let request: VerificationBatchRequest = serde_json::from_str(&text).map_err(|error| {
        format!(
            "failed to parse verification batch request from `{}`: {error}",
            input_path.display()
        )
    })?;
    if request.cases.is_empty() {
        return Err("verification batch request must contain at least one case".to_string());
    }
    for case in &request.cases {
        if case.entered_cell_text.trim().is_empty() && case.spreadsheet_xml_source.is_none() {
            return Err(format!(
                "verification case `{}` must provide entered_cell_text or spreadsheet_xml_source",
                case.case_id
            ));
        }
    }
    Ok(request)
}

#[cfg(feature = "oxfml-live")]
pub fn single_case_request(
    case_id: impl Into<String>,
    formula: impl Into<String>,
) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: default_windows_excel_host_profile(),
        capabilities: default_windows_excel_capability_profile(),
        cases: vec![ProgrammaticFormulaCase {
            case_id: case_id.into(),
            entered_cell_text: formula.into(),
            spreadsheet_xml_source: None,
        }],
    }
}

#[cfg(feature = "oxfml-live")]
pub fn single_xml_case_request(
    case_id: impl Into<String>,
    workbook_path: impl Into<String>,
    locator: impl Into<String>,
) -> Result<VerificationBatchRequest, String> {
    let case_id = case_id.into();
    let workbook_path = workbook_path.into();
    let locator = locator.into();
    let extraction = extract_cell_from_spreadsheet_xml(&workbook_path, &locator)?;

    Ok(VerificationBatchRequest {
        host_profile: default_windows_excel_host_profile(),
        capabilities: default_windows_excel_capability_profile(),
        cases: vec![ProgrammaticFormulaCase {
            case_id,
            entered_cell_text: extraction.entered_cell_text,
            spreadsheet_xml_source: Some(
                crate::services::programmatic_testing::ProgrammaticSpreadsheetXmlSource {
                    workbook_path,
                    locator,
                },
            ),
        }],
    })
}

#[cfg(feature = "oxfml-live")]
pub fn default_output_root() -> Result<PathBuf, String> {
    let repo_root = repo_root()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("failed to compute timestamp for verification bundle: {error}"))?
        .as_secs();
    Ok(repo_root
        .join("target")
        .join("onecalc-verification")
        .join(format!("bundle-{timestamp}")))
}

#[cfg(feature = "oxfml-live")]
pub fn run_verification_batch(
    request: &VerificationBatchRequest,
    output_root: impl AsRef<Path>,
) -> Result<VerificationBundleReport, String> {
    let runner = ProcessVerificationCommandRunner;
    run_verification_batch_with_runner(request, output_root, &runner)
}

#[cfg(not(feature = "oxfml-live"))]
pub fn load_verification_batch_request(
    _input_path: impl AsRef<Path>,
) -> Result<VerificationBatchRequest, String> {
    Err("verification CLI requires the `oxfml-live` feature".to_string())
}

#[cfg(not(feature = "oxfml-live"))]
pub fn single_case_request(
    case_id: impl Into<String>,
    formula: impl Into<String>,
) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: default_windows_excel_host_profile(),
        capabilities: default_windows_excel_capability_profile(),
        cases: vec![ProgrammaticFormulaCase {
            case_id: case_id.into(),
            entered_cell_text: formula.into(),
            spreadsheet_xml_source: None,
        }],
    }
}

#[cfg(not(feature = "oxfml-live"))]
pub fn single_xml_case_request(
    _case_id: impl Into<String>,
    _workbook_path: impl Into<String>,
    _locator: impl Into<String>,
) -> Result<VerificationBatchRequest, String> {
    Err("verification CLI requires the `oxfml-live` feature".to_string())
}

#[cfg(not(feature = "oxfml-live"))]
pub fn default_output_root() -> Result<PathBuf, String> {
    Err("verification CLI requires the `oxfml-live` feature".to_string())
}

#[cfg(not(feature = "oxfml-live"))]
pub fn run_verification_batch(
    _request: &VerificationBatchRequest,
    _output_root: impl AsRef<Path>,
) -> Result<VerificationBundleReport, String> {
    Err("verification CLI requires the `oxfml-live` feature".to_string())
}

#[cfg(feature = "oxfml-live")]
pub fn run_verification_batch_with_runner<R: VerificationCommandRunner>(
    request: &VerificationBatchRequest,
    output_root: impl AsRef<Path>,
    runner: &R,
) -> Result<VerificationBundleReport, String> {
    if request.cases.is_empty() {
        return Err("verification batch request must contain at least one case".to_string());
    }
    for case in &request.cases {
        if case.entered_cell_text.trim().is_empty() && case.spreadsheet_xml_source.is_none() {
            return Err(format!(
                "verification case `{}` must provide entered_cell_text or spreadsheet_xml_source",
                case.case_id
            ));
        }
    }

    let repo_root = repo_root()?;
    let output_root = output_root.as_ref();
    fs::create_dir_all(output_root).map_err(|error| {
        format!(
            "failed to create verification bundle output root `{}`: {error}",
            output_root.display()
        )
    })?;

    let bundle_id = output_root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("verification-bundle")
        .to_string();
    let batch_plan = build_programmatic_batch_plan(
        &request.cases,
        &request.host_profile,
        &request.capabilities,
    );

    write_json_file(output_root.join("input-request.json"), request)?;
    write_json_file(output_root.join("batch-plan.json"), &batch_plan)?;

    let mut retained_artifact_catalog = Vec::with_capacity(request.cases.len());
    let mut case_reports = Vec::with_capacity(request.cases.len());

    for case in &request.cases {
        let case_report = run_case_verification(
            &repo_root,
            output_root,
            case,
            &request.host_profile,
            &request.capabilities,
            runner,
        )?;
        retained_artifact_catalog.push(case_report.artifact_catalog_entry.clone());
        case_reports.push(case_report);
    }

    let report = VerificationBundleReport {
        bundle_id,
        output_root: display_repo_relative(output_root, &repo_root),
        host_profile: request.host_profile.clone(),
        capabilities: request.capabilities.clone(),
        batch_plan,
        retained_artifact_catalog,
        case_reports,
    };
    write_json_file(output_root.join("verification-bundle-report.json"), &report)?;
    write_json_file(
        output_root.join("retained-artifact-catalog.json"),
        &report.retained_artifact_catalog,
    )?;
    Ok(report)
}

#[cfg(feature = "oxfml-live")]
fn run_case_verification<R: VerificationCommandRunner>(
    repo_root: &Path,
    output_root: &Path,
    case: &ProgrammaticFormulaCase,
    host_profile: &ProgrammaticHostProfile,
    capabilities: &ProgrammaticCapabilityProfile,
    runner: &R,
) -> Result<VerificationCaseReport, String> {
    let case_dir = output_root.join("cases").join(sanitize_case_id(&case.case_id));
    let command_dir = case_dir.join("commands");
    let oxxlplay_dir = case_dir.join("oxxlplay");
    let oxreplay_dir = case_dir.join("oxreplay");
    fs::create_dir_all(&command_dir).map_err(|error| {
        format!(
            "failed to create case command directory `{}`: {error}",
            command_dir.display()
        )
    })?;
    fs::create_dir_all(&oxxlplay_dir).map_err(|error| {
        format!(
            "failed to create OxXlPlay output directory `{}`: {error}",
            oxxlplay_dir.display()
        )
    })?;
    fs::create_dir_all(&oxreplay_dir).map_err(|error| {
        format!(
            "failed to create OxReplay output directory `{}`: {error}",
            oxreplay_dir.display()
        )
    })?;

    let spreadsheet_xml_extraction = if let Some(source) = &case.spreadsheet_xml_source {
        let extraction = extract_cell_from_spreadsheet_xml(&source.workbook_path, &source.locator)?;
        write_json_file(case_dir.join("xml-cell-extract.json"), &extraction)?;
        write_json_file(
            case_dir.join("required-observation-scope.json"),
            &extraction.observation_scope,
        )?;
        Some(extraction)
    } else {
        None
    };
    let effective_case = ProgrammaticFormulaCase {
        case_id: case.case_id.clone(),
        entered_cell_text: spreadsheet_xml_extraction
            .as_ref()
            .map(|extraction| extraction.entered_cell_text.clone())
            .unwrap_or_else(|| case.entered_cell_text.clone()),
        spreadsheet_xml_source: case.spreadsheet_xml_source.clone(),
    };
    let upstream_gap_report = spreadsheet_xml_extraction
        .as_ref()
        .map(build_observation_gap_report);
    if let Some(gap_report) = &upstream_gap_report {
        write_json_file(case_dir.join("upstream-gap-report.json"), gap_report)?;
    }

    write_json_file(
        case_dir.join("case-input.json"),
        &json!({
            "requested_case": case,
            "effective_case": &effective_case,
            "host_profile": host_profile,
            "capabilities": capabilities,
            "spreadsheet_xml_extraction": spreadsheet_xml_extraction,
        }),
    )?;

    let oxfml_result = run_oxfml_case(&effective_case)?;
    let projection_path = case_dir.join("oxfml-v1-replay-projection.json");
    write_json_file(case_dir.join("oxfml-runtime-summary.json"), &oxfml_result.summary)?;
    write_json_file(&projection_path, &oxfml_result.replay_projection_json)?;

    let workbook_path = case_dir.join("workbook.xml");
    let workbook_write =
        materialize_case_workbook(&workbook_path, &effective_case, spreadsheet_xml_extraction.as_ref())?;
    write_json_file(command_dir.join("write-workbook.json"), &workbook_write)?;

    let scenario_path = case_dir.join("scenario.json");
    let scenario_json = build_oxxlplay_scenario_json(
        repo_root,
        &case_dir,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
    );
    write_json_file(&scenario_path, &scenario_json)?;

    let capture = runner.run_oxxlplay_capture(&scenario_path, &oxxlplay_dir)?;
    write_json_file(command_dir.join("oxxlplay-capture.json"), &capture)?;
    if capture.exit_code != 0 {
        return finish_blocked_case(
            repo_root,
            &effective_case,
            &case_dir,
            format!("OxXlPlay capture failed with exit code {}", capture.exit_code),
            oxfml_result.summary,
            None,
            spreadsheet_xml_extraction,
            upstream_gap_report,
        );
    }

    let excel_summary = summarize_excel_capture(oxxlplay_dir.join("capture.json"))?;
    write_json_file(case_dir.join("excel-observation-summary.json"), &excel_summary)?;

    let validate_capture =
        runner.run_oxreplay_validate_bundle(&oxxlplay_dir.join("oxreplay-manifest.json"))?;
    write_json_file(
        command_dir.join("oxreplay-validate-bundle.json"),
        &validate_capture,
    )?;
    if !validate_capture.stdout.trim().is_empty() {
        write_json_text_file(
            oxreplay_dir.join("validate-bundle.report.json"),
            &validate_capture.stdout,
        )?;
    }
    if validate_capture.exit_code != 0 {
        return finish_blocked_case(
            repo_root,
            &effective_case,
            &case_dir,
            format!(
                "OxReplay validate-bundle reported exit code {}",
                validate_capture.exit_code
            ),
            oxfml_result.summary,
            Some(excel_summary),
            spreadsheet_xml_extraction,
            upstream_gap_report,
        );
    }

    let diff_capture = runner.run_oxreplay_diff(
        &projection_path,
        "oxfml-v1-replay-projection",
        &oxxlplay_dir.join("views").join("normalized-replay.json"),
        "normalized-replay",
    )?;
    write_json_file(command_dir.join("oxreplay-diff.json"), &diff_capture)?;
    if !diff_capture.stdout.trim().is_empty() {
        write_json_text_file(oxreplay_dir.join("diff.report.json"), &diff_capture.stdout)?;
    }

    let explain_capture = runner.run_oxreplay_explain(
        &projection_path,
        "oxfml-v1-replay-projection",
        &oxxlplay_dir.join("views").join("normalized-replay.json"),
        "normalized-replay",
    )?;
    write_json_file(command_dir.join("oxreplay-explain.json"), &explain_capture)?;
    if !explain_capture.stdout.trim().is_empty() {
        write_json_text_file(
            oxreplay_dir.join("explain.report.json"),
            &explain_capture.stdout,
        )?;
    }

    let diff_report = parse_json_text(&diff_capture.stdout, "OxReplay diff stdout")?;
    let is_equivalent = diff_report
        .get("equivalent")
        .and_then(Value::as_bool)
        .ok_or_else(|| "OxReplay diff output did not contain a boolean `equivalent`".to_string())?;

    let replay_mismatch_kinds = diff_report
        .get("mismatches")
        .and_then(Value::as_array)
        .map(|mismatches| {
            mismatches
                .iter()
                .filter_map(|mismatch| {
                    mismatch
                        .get("mismatch_kind")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let visible_output_match = match (
        oxfml_result.summary.effective_display_summary.as_deref(),
        excel_summary.observed_value_repr.as_deref(),
    ) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    };
    let comparison_status = if diff_capture.exit_code == 0 && is_equivalent {
        ProgrammaticComparisonStatus::Matched
    } else if diff_capture.exit_code == 1 && !is_equivalent {
        ProgrammaticComparisonStatus::Mismatched
    } else {
        ProgrammaticComparisonStatus::Blocked
    };

    let discrepancy_summary = build_discrepancy_summary(
        comparison_status,
        visible_output_match,
        &replay_mismatch_kinds,
        &oxfml_result.summary,
        &excel_summary,
    );
    let artifact_catalog_entry = build_programmatic_artifact_catalog_entry(
        format!("artifact-{}", sanitize_case_id(&case.case_id)),
        case.case_id.clone(),
        comparison_status,
    );

    let report = VerificationCaseReport {
        case_id: case.case_id.clone(),
        entered_cell_text: effective_case.entered_cell_text.clone(),
        artifact_catalog_entry: artifact_catalog_entry.clone(),
        comparison_status,
        visible_output_match,
        replay_equivalent: Some(is_equivalent),
        replay_mismatch_kinds,
        discrepancy_summary,
        oxfml_summary: oxfml_result.summary,
        excel_summary: Some(excel_summary),
        spreadsheet_xml_extraction,
        upstream_gap_report,
        case_output_dir: display_repo_relative(&case_dir, repo_root),
        scenario_path: display_repo_relative(&scenario_path, repo_root),
    };
    write_json_file(
        case_dir.join("programmatic-artifact-catalog-entry.json"),
        &artifact_catalog_entry,
    )?;
    write_json_file(case_dir.join("comparison-summary.json"), &report)?;
    Ok(report)
}

#[cfg(feature = "oxfml-live")]
fn finish_blocked_case(
    repo_root: &Path,
    case: &ProgrammaticFormulaCase,
    case_dir: &Path,
    blocked_reason: String,
    oxfml_summary: OxfmlVerificationSummary,
    excel_summary: Option<ExcelObservationSummary>,
    spreadsheet_xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    upstream_gap_report: Option<VerificationObservationGapReport>,
) -> Result<VerificationCaseReport, String> {
    let artifact_catalog_entry = build_programmatic_artifact_catalog_entry(
        format!("artifact-{}", sanitize_case_id(&case.case_id)),
        case.case_id.clone(),
        ProgrammaticComparisonStatus::Blocked,
    );
    let report = VerificationCaseReport {
        case_id: case.case_id.clone(),
        entered_cell_text: case.entered_cell_text.clone(),
        artifact_catalog_entry: artifact_catalog_entry.clone(),
        comparison_status: ProgrammaticComparisonStatus::Blocked,
        visible_output_match: None,
        replay_equivalent: None,
        replay_mismatch_kinds: Vec::new(),
        discrepancy_summary: Some(blocked_reason),
        oxfml_summary,
        excel_summary,
        spreadsheet_xml_extraction,
        upstream_gap_report,
        case_output_dir: display_repo_relative(case_dir, repo_root),
        scenario_path: display_repo_relative(case_dir.join("scenario.json"), repo_root),
    };
    write_json_file(
        case_dir.join("programmatic-artifact-catalog-entry.json"),
        &artifact_catalog_entry,
    )?;
    write_json_file(case_dir.join("comparison-summary.json"), &report)?;
    Ok(report)
}

#[cfg(feature = "oxfml-live")]
struct OxfmlCaseArtifacts {
    summary: OxfmlVerificationSummary,
    replay_projection_json: Value,
}

#[cfg(feature = "oxfml-live")]
fn run_oxfml_case(case: &ProgrammaticFormulaCase) -> Result<OxfmlCaseArtifacts, String> {
    let bridge = LiveOxfmlBridge::default();
    let formula_edit_result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: case.case_id.clone(),
            entered_text: case.entered_cell_text.clone(),
            cursor_offset: case.entered_cell_text.len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::FullSemanticPlan,
        })
        .map_err(|error| format!("live OxFml bridge failed for case `{}`: {error:?}", case.case_id))?;

    let source = FormulaSourceRecord::new(case.case_id.clone(), 1, case.entered_cell_text.clone())
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let runtime_result = RuntimeEnvironment::new()
        .execute(RuntimeFormulaRequest::new(
            source,
            TypedContextQueryBundle::default(),
        ))
        .map_err(|error| format!("OxFml runtime execution failed for case `{}`: {error}", case.case_id))?;
    let projection = ReplayProjectionService::project(
        ReplayProjectionRequest::runtime_result(&runtime_result)
            .with_source_case_id(case.case_id.clone())
            .with_shared_scenario_alias(format!("onecalc_verify_{}", sanitize_case_id(&case.case_id))),
    );

    let summary = OxfmlVerificationSummary {
        evaluation_summary: formula_edit_result
            .document
            .value_presentation
            .as_ref()
            .map(|value| value.evaluation_summary.clone()),
        effective_display_summary: formula_edit_result
            .document
            .value_presentation
            .as_ref()
            .and_then(|value| value.effective_display_summary.clone()),
        blocked_reason: formula_edit_result
            .document
            .value_presentation
            .as_ref()
            .and_then(|value| value.blocked_reason.clone())
            .or_else(|| {
                formula_edit_result
                    .document
                    .provenance_summary
                    .as_ref()
                    .and_then(|summary| summary.blocked_reason.clone())
            }),
        parse_status: formula_edit_result
            .document
            .parse_summary
            .as_ref()
            .map(|summary| summary.status.clone()),
        green_tree_key: Some(
            formula_edit_result
                .document
                .editor_syntax_snapshot
                .green_tree_key
                .clone(),
        ),
    };

    Ok(OxfmlCaseArtifacts {
        summary,
        replay_projection_json: serialize_replay_projection(&projection),
    })
}

#[cfg(feature = "oxfml-live")]
fn build_oxxlplay_scenario_json(
    repo_root: &Path,
    case_dir: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Value {
    let scenario_id = format!("onecalc_verify_{}", sanitize_case_id(&case.case_id));
    let locator = spreadsheet_xml_extraction
        .map(|extraction| extraction.locator.clone())
        .unwrap_or_else(|| "Sheet1!A1".to_string());
    json!({
        "scenario_id": scenario_id,
        "replay_class": "capture_surface_basic",
        "retained_root": display_repo_relative(case_dir, repo_root),
        "workbook_ref": "./workbook.xml",
        "workbook_kind": if spreadsheet_xml_extraction.is_some() { "spreadsheetml-2003-import" } else { "spreadsheetml-2003-generated" },
        "trigger": "open_then_recalc",
        "observable_surfaces": [
            {
                "surface_id": "sheet1_a1_value",
                "surface_kind": "cell_value",
                "locator": &locator,
                "required": true
            },
            {
                "surface_id": "sheet1_a1_formula",
                "surface_kind": "formula_text",
                "locator": &locator,
                "required": false
            }
        ],
        "requested_observation_scope": spreadsheet_xml_extraction.map(|extraction| &extraction.observation_scope),
        "source_cell_locator": spreadsheet_xml_extraction.map(|extraction| extraction.locator.clone()),
        "source_workbook_path": spreadsheet_xml_extraction.map(|extraction| extraction.workbook_path.clone())
    })
}

#[cfg(feature = "oxfml-live")]
fn build_observation_gap_report(
    observation_scope: &SpreadsheetXmlCellExtraction,
) -> VerificationObservationGapReport {
    let oxxlplay_supported_surfaces = vec!["cell_value".to_string(), "formula_text".to_string()];
    let oxxlplay_missing_surfaces = observation_scope
        .observation_scope
        .oxxlplay_required_surfaces
        .iter()
        .filter(|surface| {
            !oxxlplay_supported_surfaces
                .iter()
                .any(|supported| supported == *surface)
        })
        .cloned()
        .collect::<Vec<_>>();
    let oxreplay_current_bundle_views =
        vec!["visible_value".to_string(), "replay_normalized_events".to_string()];
    let oxreplay_missing_views = observation_scope
        .observation_scope
        .oxreplay_required_views
        .iter()
        .filter(|view| {
            !oxreplay_current_bundle_views
                .iter()
                .any(|supported| supported == *view)
        })
        .cloned()
        .collect::<Vec<_>>();

    VerificationObservationGapReport {
        oxfml_scope_required: observation_scope.observation_scope.oxfml_required_scope.clone(),
        oxxlplay_supported_surfaces,
        oxxlplay_missing_surfaces,
        oxreplay_required_views: observation_scope.observation_scope.oxreplay_required_views.clone(),
        oxreplay_current_bundle_views,
        oxreplay_missing_views,
    }
}

#[cfg(feature = "oxfml-live")]
fn summarize_excel_capture(capture_path: PathBuf) -> Result<ExcelObservationSummary, String> {
    let capture_json = read_json_file(&capture_path)?;
    let surfaces = capture_json
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("capture file `{}` did not contain a `surfaces` array", capture_path.display()))?;

    let mut observed_value_repr = None;
    let mut observed_formula_repr = None;
    let mut capture_status = "captured".to_string();

    for surface in surfaces {
        let surface_kind = surface
            .get("surface")
            .and_then(|value| value.get("surface_kind"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = surface
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let value_repr = surface
            .get("value_repr")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        match surface_kind {
            "cell_value" => {
                observed_value_repr = value_repr;
                if status != "direct" && capture_status == "captured" {
                    capture_status = status.to_string();
                }
            }
            "formula_text" => {
                observed_formula_repr = value_repr;
            }
            _ => {}
        }
    }

    Ok(ExcelObservationSummary {
        observed_value_repr,
        observed_formula_repr,
        capture_status,
    })
}

#[cfg(feature = "oxfml-live")]
fn serialize_replay_projection(
    projection: &oxfml_core::consumer::replay::ReplayProjectionResult,
) -> Value {
    json!({
        "source_artifact_family": projection.source_artifact_family,
        "source_case_id": projection.source_case_id,
        "source_case_ids": projection.source_case_ids,
        "shared_scenario_alias": projection.shared_scenario_alias,
        "formula_stable_id": projection.formula_stable_id,
        "session_id": projection.session_id,
        "witness_id": projection.witness_id,
        "phase": projection.phase,
        "commit_decision_kind": projection.commit_decision_kind,
        "trace_event_kinds": projection.trace_event_kinds,
    })
}

#[cfg(feature = "oxfml-live")]
fn build_discrepancy_summary(
    comparison_status: ProgrammaticComparisonStatus,
    visible_output_match: Option<bool>,
    replay_mismatch_kinds: &[String],
    oxfml_summary: &OxfmlVerificationSummary,
    excel_summary: &ExcelObservationSummary,
) -> Option<String> {
    match comparison_status {
        ProgrammaticComparisonStatus::Matched => None,
        ProgrammaticComparisonStatus::Blocked => Some(
            oxfml_summary
                .blocked_reason
                .clone()
                .unwrap_or_else(|| "comparison blocked before both lanes completed".to_string()),
        ),
        ProgrammaticComparisonStatus::Mismatched => {
            let oxfml_value = oxfml_summary
                .effective_display_summary
                .clone()
                .unwrap_or_else(|| "<unavailable>".to_string());
            let excel_value = excel_summary
                .observed_value_repr
                .clone()
                .unwrap_or_else(|| "<unavailable>".to_string());

            if visible_output_match == Some(true) {
                let mismatch_kinds = if replay_mismatch_kinds.is_empty() {
                    "unknown_replay_mismatch".to_string()
                } else {
                    replay_mismatch_kinds.join(", ")
                };
                Some(format!(
                    "Visible outputs matched at `{oxfml_value}`, but replay comparison still diverged ({mismatch_kinds}). This points to an OxFml/OxReplay normalization seam, not a visible value disagreement."
                ))
            } else {
                Some(format!("OxFml={oxfml_value} / Excel={excel_value}"))
            }
        }
    }
}

#[cfg(feature = "oxfml-live")]
fn run_command_capture(
    command_label: &str,
    program: &str,
    args: &[OsString],
) -> Result<VerificationCommandCapture, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("failed to start `{command_label}`: {error}"))?;

    Ok(VerificationCommandCapture {
        command_label: command_label.to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn materialize_case_workbook(
    workbook_path: &Path,
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Result<VerificationCommandCapture, String> {
    if let Some(extraction) = spreadsheet_xml_extraction {
        fs::copy(&extraction.workbook_path, workbook_path).map_err(|error| {
            format!(
                "failed to copy SpreadsheetML workbook `{}` to `{}`: {error}",
                extraction.workbook_path,
                workbook_path.display()
            )
        })?;
        return Ok(VerificationCommandCapture {
            command_label: "copy-spreadsheetml-workbook".to_string(),
            exit_code: 0,
            stdout: format!(
                "copied workbook from {} to {}",
                extraction.workbook_path,
                workbook_path.display()
            ),
            stderr: String::new(),
        });
    }

    write_excel_2003_xml_workbook(workbook_path, &case.entered_cell_text)
}

fn write_excel_2003_xml_workbook(
    workbook_path: &Path,
    entered_cell_text: &str,
) -> Result<VerificationCommandCapture, String> {
    let cell_xml = spreadsheet_cell_xml(entered_cell_text);
    let workbook_xml = format!(
        r#"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:o="urn:schemas-microsoft-com:office:office"
 xmlns:x="urn:schemas-microsoft-com:office:excel"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
  <Worksheet ss:Name="Sheet1">
    <Table>
      <Row>
        {}
      </Row>
    </Table>
  </Worksheet>
</Workbook>
"#,
        cell_xml
    );
    fs::write(workbook_path, workbook_xml).map_err(|error| {
        format!(
            "failed to write Excel 2003 XML workbook `{}`: {error}",
            workbook_path.display()
        )
    })?;

    Ok(VerificationCommandCapture {
        command_label: "write-workbook".to_string(),
        exit_code: 0,
        stdout: format!("wrote workbook to {}", workbook_path.display()),
        stderr: String::new(),
    })
}

fn spreadsheet_cell_xml(entered_cell_text: &str) -> String {
    if entered_cell_text.starts_with('=') {
        return format!(
            r#"<Cell ss:Formula="{}"><Data ss:Type="Number">0</Data></Cell>"#,
            escape_spreadsheet_xml(entered_cell_text)
        );
    }

    if let Some(text) = entered_cell_text.strip_prefix('\'') {
        return format!(
            r#"<Cell><Data ss:Type="String">{}</Data></Cell>"#,
            escape_spreadsheet_xml(text)
        );
    }

    if let Ok(number) = entered_cell_text.parse::<f64>() {
        return format!(
            r#"<Cell><Data ss:Type="Number">{}</Data></Cell>"#,
            number
        );
    }

    if entered_cell_text.eq_ignore_ascii_case("true")
        || entered_cell_text.eq_ignore_ascii_case("false")
    {
        let boolean_value = if entered_cell_text.eq_ignore_ascii_case("true") {
            "1"
        } else {
            "0"
        };
        return format!(
            r#"<Cell><Data ss:Type="Boolean">{boolean_value}</Data></Cell>"#
        );
    }

    format!(
        r#"<Cell><Data ss:Type="String">{}</Data></Cell>"#,
        escape_spreadsheet_xml(entered_cell_text)
    )
}

fn escape_spreadsheet_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_json_file(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory for `{}`: {error}",
                path.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize JSON for `{}`: {error}", path.display()))?;
    fs::write(path, text)
        .map_err(|error| format!("failed to write JSON file `{}`: {error}", path.display()))
}

fn write_json_text_file(path: impl AsRef<Path>, text: &str) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory for `{}`: {error}",
                path.display()
            )
        })?;
    }
    let normalized = serde_json::from_str::<Value>(text).map_err(|error| {
        format!(
            "failed to parse JSON text before writing `{}`: {error}",
            path.display()
        )
    })?;
    write_json_file(path, &normalized)
}

fn read_json_file(path: impl AsRef<Path>) -> Result<Value, String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSON file `{}`: {error}", path.display()))?;
    parse_json_text(&text, &path.display().to_string())
}

fn parse_json_text(text: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(text)
        .map_err(|error| format!("failed to parse JSON from `{label}`: {error}"))
}

fn sanitize_case_id(case_id: &str) -> String {
    case_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn repo_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve DnaOneCalc repo root".to_string())
}

fn display_repo_relative(path: impl AsRef<Path>, repo_root: &Path) -> String {
    let path = path.as_ref();
    path.strip_prefix(repo_root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|error| format!("failed to resolve absolute path for `{}`: {error}", path.display()))
}

#[cfg(all(test, feature = "oxfml-live"))]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeVerificationRunner {
        capture_exit_code: i32,
        validate_exit_code: i32,
        diff_exit_code: i32,
        explain_exit_code: i32,
        diff_equivalent: bool,
        calls: Mutex<Vec<String>>,
    }

    impl VerificationCommandRunner for FakeVerificationRunner {
        fn run_oxxlplay_capture(
            &self,
            _scenario_path: &Path,
            output_dir: &Path,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls.lock().expect("calls").push("oxxlplay_capture".to_string());
            fs::create_dir_all(output_dir.join("views")).expect("views dir");
            write_json_file(
                output_dir.join("capture.json"),
                &json!({
                    "surfaces": [
                        {
                            "surface": {
                                "surface_id": "sheet1_a1_value",
                                "surface_kind": "cell_value",
                                "locator": "Sheet1!A1",
                                "required": true
                            },
                            "status": "direct",
                            "value_repr": if self.diff_equivalent { "6" } else { "7" },
                            "capture_loss": "none",
                            "uncertainty": "none"
                        },
                        {
                            "surface": {
                                "surface_id": "sheet1_a1_formula",
                                "surface_kind": "formula_text",
                                "locator": "Sheet1!A1",
                                "required": false
                            },
                            "status": "direct",
                            "value_repr": "=SUM(1,2,3)",
                            "capture_loss": "none",
                            "uncertainty": "none"
                        }
                    ]
                }),
            )
            .expect("capture should write");
            write_json_file(
                output_dir.join("oxreplay-manifest.json"),
                &json!({
                    "bundle_id": "fake-bundle",
                    "scenario_id": "onecalc_verify_case_1",
                    "bundle_schema": "replay.bundle.v1"
                }),
            )
            .expect("manifest should write");
            write_json_file(
                output_dir.join("views").join("normalized-replay.json"),
                &json!({
                    "scenario_id": "onecalc_verify_case_1",
                    "lane_id": "oxxlplay",
                    "events": [
                        {
                            "event_id": "sheet1_a1_value",
                            "source_label": "cell_value:Sheet1!A1:direct",
                            "normalized_family": "excel.surface.cell_value.direct:Sheet1!A1=6"
                        }
                    ],
                    "registry_refs": []
                }),
            )
            .expect("normalized replay should write");
            Ok(VerificationCommandCapture {
                command_label: "oxxlplay-capture".to_string(),
                exit_code: self.capture_exit_code,
                stdout: String::new(),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_validate_bundle(
            &self,
            _manifest_path: &Path,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls.lock().expect("calls").push("validate_bundle".to_string());
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-validate-bundle".to_string(),
                exit_code: self.validate_exit_code,
                stdout: "{\"status\":\"Valid\"}".to_string(),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_diff(
            &self,
            _left_path: &Path,
            _left_kind: &str,
            _right_path: &Path,
            _right_kind: &str,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls.lock().expect("calls").push("diff".to_string());
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-diff".to_string(),
                exit_code: self.diff_exit_code,
                stdout: format!(
                    "{{\"equivalent\":{}}}",
                    if self.diff_equivalent { "true" } else { "false" }
                ),
                stderr: String::new(),
            })
        }

        fn run_oxreplay_explain(
            &self,
            _left_path: &Path,
            _left_kind: &str,
            _right_path: &Path,
            _right_kind: &str,
        ) -> Result<VerificationCommandCapture, String> {
            self.calls.lock().expect("calls").push("explain".to_string());
            Ok(VerificationCommandCapture {
                command_label: "oxreplay-explain".to_string(),
                exit_code: self.explain_exit_code,
                stdout: "{\"summary\":\"diff\"}".to_string(),
                stderr: String::new(),
            })
        }
    }

    #[test]
    fn verification_batch_writes_mismatched_case_as_workbench_artifact() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: false,
            diff_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(report.case_reports.len(), 1);
        assert_eq!(
            report.case_reports[0].comparison_status,
            ProgrammaticComparisonStatus::Mismatched
        );
        assert_eq!(
            report.case_reports[0].artifact_catalog_entry.open_mode_hint,
            crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench
        );
        assert!(output_root.join("verification-bundle-report.json").is_file());
        assert!(
            output_root
                .join("cases")
                .join("case-1")
                .join("comparison-summary.json")
                .is_file()
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_marks_capture_failure_as_blocked() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let output_root = temp_root.join("bundle");
        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-1".to_string(),
                entered_cell_text: "=SUM(1,2,3)".to_string(),
                spreadsheet_xml_source: None,
            }],
        };
        let runner = FakeVerificationRunner {
            capture_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(
            report.case_reports[0].comparison_status,
            ProgrammaticComparisonStatus::Blocked
        );
        assert_eq!(
            report.case_reports[0].artifact_catalog_entry.open_mode_hint,
            crate::services::programmatic_testing::ProgrammaticOpenModeHint::Workbench
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_records_spreadsheetml_scope_for_xml_backed_cases() {
        let temp_root = std::env::temp_dir().join(format!(
            "onecalc-verification-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let xml_path = temp_root.join("source.xml");
        let output_root = temp_root.join("bundle");
        fs::create_dir_all(&temp_root).expect("temp root");
        fs::write(
            &xml_path,
            r##"<?xml version="1.0"?>
<?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
 xmlns:x="urn:schemas-microsoft-com:office:excel">
  <Styles>
    <Style ss:ID="calc">
      <NumberFormat ss:Format="$#,##0.00"/>
      <Font ss:Color="#112233"/>
      <Interior ss:Color="#445566"/>
    </Style>
  </Styles>
  <Worksheet ss:Name="Input">
    <Table>
      <Row>
        <Cell ss:StyleID="calc" ss:Formula="=SUM(1,2,3)"><Data ss:Type="Number">0</Data></Cell>
      </Row>
    </Table>
    <ConditionalFormatting ss:Range="A1">
      <Condition ss:Type="Expression" ss:Formula="=A1>0"/>
      <Font ss:Color="#FF0000"/>
      <Interior ss:Color="#00FF00"/>
    </ConditionalFormatting>
  </Worksheet>
</Workbook>"##,
        )
        .expect("xml write");

        let request = VerificationBatchRequest {
            host_profile: default_windows_excel_host_profile(),
            capabilities: default_windows_excel_capability_profile(),
            cases: vec![ProgrammaticFormulaCase {
                case_id: "case-xml".to_string(),
                entered_cell_text: String::new(),
                spreadsheet_xml_source: Some(
                    crate::services::programmatic_testing::ProgrammaticSpreadsheetXmlSource {
                        workbook_path: xml_path.to_string_lossy().into_owned(),
                        locator: "Input!A1".to_string(),
                    },
                ),
            }],
        };
        let runner = FakeVerificationRunner {
            diff_equivalent: false,
            diff_exit_code: 1,
            ..Default::default()
        };

        let report =
            run_verification_batch_with_runner(&request, &output_root, &runner).expect("report");

        assert_eq!(report.case_reports.len(), 1);
        assert_eq!(report.case_reports[0].entered_cell_text, "=SUM(1,2,3)");
        assert!(report.case_reports[0].spreadsheet_xml_extraction.is_some());
        assert!(report.case_reports[0].upstream_gap_report.is_some());
        assert!(
            output_root
                .join("cases")
                .join("case-xml")
                .join("xml-cell-extract.json")
                .is_file()
        );
        assert!(
            output_root
                .join("cases")
                .join("case-xml")
                .join("required-observation-scope.json")
                .is_file()
        );
        assert!(
            output_root
                .join("cases")
                .join("case-xml")
                .join("upstream-gap-report.json")
                .is_file()
        );

        let _ = fs::remove_dir_all(temp_root);
    }
}
