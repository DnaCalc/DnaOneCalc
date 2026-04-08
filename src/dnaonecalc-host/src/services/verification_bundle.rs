use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::programmatic_testing::{
    build_programmatic_artifact_catalog_entry, build_programmatic_batch_plan,
    default_verification_config, default_windows_excel_capability_profile,
    default_windows_excel_host_profile, ProgrammaticArtifactCatalogEntry, ProgrammaticBatchPlan,
    ProgrammaticCapabilityProfile, ProgrammaticComparisonStatus, ProgrammaticFormulaCase,
    ProgrammaticHostProfile,
};
use crate::services::spreadsheet_xml::{
    extract_cell_from_spreadsheet_xml, SpreadsheetXmlCellExtraction,
};

#[cfg(feature = "oxfml-live")]
use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaEditRequest, LiveOxfmlBridge, OxfmlEditorBridge,
};
#[cfg(feature = "oxfml-live")]
use oxfml_core::consumer::replay::{ReplayProjectionRequest, ReplayProjectionService};
#[cfg(feature = "oxfml-live")]
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest};
#[cfg(feature = "oxfml-live")]
use oxfml_core::interface::TypedContextQueryBundle;
#[cfg(feature = "oxfml-live")]
use oxfml_core::publication::{
    LocaleFormatContextSurface, VerificationConditionalFormattingRule,
    VerificationPublicationContext, VerificationPublicationSurface,
};
#[cfg(feature = "oxfml-live")]
use oxfml_core::source::FormulaSourceRecord;
#[cfg(feature = "oxfml-live")]
use oxfml_core::FormulaChannelKind;
#[cfg(feature = "oxfml-live")]
use oxfunc_core::locale_format::{
    format_profile, LocaleFormatContext, LocaleProfileId, WorkbookDateSystem,
    TEST_FORMAT_CODE_ENGINE, TEST_LOCALE_VALUE_PARSER,
};

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
    pub effective_display_text: Option<String>,
    pub observed_formula_repr: Option<String>,
    pub capture_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxReplayMismatchRecord {
    pub mismatch_kind: String,
    pub severity: Option<String>,
    pub view_family: Option<String>,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OxReplayExplainRecord {
    pub query_id: Option<String>,
    pub summary: Option<String>,
    pub mismatch_kind: String,
    pub severity: Option<String>,
    pub view_family: Option<String>,
    pub left_value_repr: Option<String>,
    pub right_value_repr: Option<String>,
    pub detail: Option<String>,
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
    pub replay_mismatch_records: Vec<OxReplayMismatchRecord>,
    pub replay_explain_records: Vec<OxReplayExplainRecord>,
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
    let config = default_verification_config();
    single_case_request_with_config(case_id, formula, &config)
}

#[cfg(feature = "oxfml-live")]
pub fn single_case_request_with_config(
    case_id: impl Into<String>,
    formula: impl Into<String>,
    config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: config.host_profile.clone(),
        capabilities: config.capabilities.clone(),
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
    let config = default_verification_config();
    single_xml_case_request_with_config(case_id, workbook_path, locator, &config)
}

#[cfg(feature = "oxfml-live")]
pub fn single_xml_case_request_with_config(
    case_id: impl Into<String>,
    workbook_path: impl Into<String>,
    locator: impl Into<String>,
    config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
) -> Result<VerificationBatchRequest, String> {
    let case_id = case_id.into();
    let workbook_path = workbook_path.into();
    let locator = locator.into();
    let extraction = extract_cell_from_spreadsheet_xml(&workbook_path, &locator)?;

    Ok(VerificationBatchRequest {
        host_profile: config.host_profile.clone(),
        capabilities: config.capabilities.clone(),
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
    let config = default_verification_config();
    single_case_request_with_config(case_id, formula, &config)
}

#[cfg(not(feature = "oxfml-live"))]
pub fn single_case_request_with_config(
    case_id: impl Into<String>,
    formula: impl Into<String>,
    config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
) -> VerificationBatchRequest {
    VerificationBatchRequest {
        host_profile: config.host_profile.clone(),
        capabilities: config.capabilities.clone(),
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
pub fn single_xml_case_request_with_config(
    _case_id: impl Into<String>,
    _workbook_path: impl Into<String>,
    _locator: impl Into<String>,
    _config: &crate::services::programmatic_testing::ProgrammaticVerificationConfig,
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
    let batch_plan =
        build_programmatic_batch_plan(&request.cases, &request.host_profile, &request.capabilities);

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
    let case_dir = output_root
        .join("cases")
        .join(sanitize_case_id(&case.case_id));
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

    let oxfml_result = run_oxfml_case(&effective_case, spreadsheet_xml_extraction.as_ref())?;
    let projection_path = case_dir.join("oxfml-v1-replay-projection.json");
    write_json_file(
        case_dir.join("oxfml-runtime-summary.json"),
        &oxfml_result.summary,
    )?;
    write_json_file(&projection_path, &oxfml_result.replay_projection_json)?;

    let workbook_path = case_dir.join("workbook.xml");
    let workbook_write = materialize_case_workbook(
        &workbook_path,
        &effective_case,
        spreadsheet_xml_extraction.as_ref(),
    )?;
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
            format!(
                "OxXlPlay capture failed with exit code {}",
                capture.exit_code
            ),
            oxfml_result.summary,
            None,
            spreadsheet_xml_extraction,
            upstream_gap_report,
        );
    }

    let excel_summary = summarize_excel_capture(oxxlplay_dir.join("capture.json"))?;
    write_json_file(
        case_dir.join("excel-observation-summary.json"),
        &excel_summary,
    )?;

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

    let replay_mismatch_records = parse_oxreplay_mismatch_records(&diff_report);
    let replay_explain_records = parse_oxreplay_explain_records(&explain_capture.stdout)?;
    let replay_mismatch_kinds = replay_mismatch_records
        .iter()
        .map(|record| record.mismatch_kind.clone())
        .collect::<Vec<_>>();
    let visible_output_match = match (
        oxfml_result.summary.effective_display_summary.as_deref(),
        preferred_excel_display_repr(&excel_summary),
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
        &replay_mismatch_records,
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
        replay_mismatch_records,
        replay_explain_records,
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
        replay_mismatch_records: Vec::new(),
        replay_explain_records: Vec::new(),
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
fn run_oxfml_case(
    case: &ProgrammaticFormulaCase,
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> Result<OxfmlCaseArtifacts, String> {
    let bridge = LiveOxfmlBridge::default();
    let formula_edit_result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: case.case_id.clone(),
            entered_text: case.entered_cell_text.clone(),
            cursor_offset: case.entered_cell_text.len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::FullSemanticPlan,
        })
        .map_err(|error| {
            format!(
                "live OxFml bridge failed for case `{}`: {error:?}",
                case.case_id
            )
        })?;

    let source = FormulaSourceRecord::new(case.case_id.clone(), 1, case.entered_cell_text.clone())
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let locale_ctx = verification_locale_context(spreadsheet_xml_extraction);
    let typed_query_bundle =
        TypedContextQueryBundle::new(None, None, Some(&locale_ctx), None, None);
    let runtime_request = RuntimeFormulaRequest::new(source, typed_query_bundle);
    let runtime_request = if let Some(extraction) = spreadsheet_xml_extraction {
        runtime_request.with_verification_publication_context(
            build_verification_publication_context(extraction),
        )
    } else {
        runtime_request
    };
    let runtime_result = RuntimeEnvironment::new()
        .execute(runtime_request)
        .map_err(|error| {
            format!(
                "OxFml runtime execution failed for case `{}`: {error}",
                case.case_id
            )
        })?;
    let projection = ReplayProjectionService::project(
        ReplayProjectionRequest::runtime_result(&runtime_result)
            .with_source_case_id(case.case_id.clone())
            .with_shared_scenario_alias(format!(
                "onecalc_verify_{}",
                sanitize_case_id(&case.case_id)
            )),
    );

    let summary = OxfmlVerificationSummary {
        evaluation_summary: formula_edit_result
            .document
            .value_presentation
            .as_ref()
            .map(|value| value.evaluation_summary.clone()),
        effective_display_summary: Some(
            runtime_result
                .verification_publication_surface
                .effective_display_text
                .clone(),
        ),
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
fn verification_locale_context(
    spreadsheet_xml_extraction: Option<&SpreadsheetXmlCellExtraction>,
) -> LocaleFormatContext<'static> {
    let date_system = if spreadsheet_xml_extraction.and_then(|value| value.date1904) == Some(true) {
        WorkbookDateSystem::System1904
    } else {
        WorkbookDateSystem::System1900
    };

    LocaleFormatContext {
        profile: format_profile(LocaleProfileId::EnUs),
        date_system,
        parser: &TEST_LOCALE_VALUE_PARSER,
        formatter: &TEST_FORMAT_CODE_ENGINE,
    }
}

#[cfg(feature = "oxfml-live")]
fn build_verification_publication_context(
    extraction: &SpreadsheetXmlCellExtraction,
) -> VerificationPublicationContext {
    VerificationPublicationContext {
        format_profile: Some(extraction.workbook_format_profile_hint.clone()),
        number_format_code: extraction.number_format_code.clone(),
        style_id: extraction.style_id.clone(),
        style_hierarchy: extraction.style_hierarchy.clone(),
        font_color: extraction.font_color.clone(),
        fill_color: extraction.fill_color.clone(),
        conditional_formatting_rules: extraction
            .conditional_formats
            .iter()
            .map(build_verification_conditional_formatting_rule)
            .collect(),
    }
}

#[cfg(feature = "oxfml-live")]
fn build_verification_conditional_formatting_rule(
    rule: &crate::services::spreadsheet_xml::ConditionalFormatRule,
) -> VerificationConditionalFormattingRule {
    let mut thresholds = Vec::new();
    if let Some(formula) = &rule.formula {
        thresholds.push(formula.clone());
    }
    if let Some(value1) = &rule.value1 {
        thresholds.push(value1.clone());
    }
    if let Some(value2) = &rule.value2 {
        thresholds.push(value2.clone());
    }

    VerificationConditionalFormattingRule {
        target_ranges: vec![rule.range.clone()],
        rule_kind: rule
            .rule_kind
            .clone()
            .unwrap_or_else(|| "expression".to_string())
            .to_ascii_lowercase(),
        operator: rule
            .operator
            .as_ref()
            .map(|value| value.to_ascii_lowercase()),
        thresholds,
        font_color: rule.font_color.clone(),
        fill_color: rule.interior_color.clone(),
        effective_display_text: None,
        applies: None,
        effective_font_color: None,
        effective_fill_color: None,
    }
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
    let workbook_kind = if spreadsheet_xml_extraction.is_some() {
        "spreadsheetml-2003-import"
    } else {
        "programmatic-formula"
    };
    let mut scenario = json!({
        "scenario_id": scenario_id,
        "replay_class": "capture_surface_basic",
        "retained_root": display_repo_relative(case_dir, repo_root),
        "workbook_ref": "./workbook.xml",
        "workbook_kind": workbook_kind,
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
    });
    if spreadsheet_xml_extraction.is_none() {
        scenario["entered_cell_text"] = Value::String(case.entered_cell_text.clone());
    }
    scenario
}

#[cfg(feature = "oxfml-live")]
fn build_observation_gap_report(
    observation_scope: &SpreadsheetXmlCellExtraction,
) -> VerificationObservationGapReport {
    let oxxlplay_supported_surfaces = vec![
        "cell_value".to_string(),
        "formula_text".to_string(),
        "effective_display_text".to_string(),
        "number_format_code".to_string(),
        "style_id".to_string(),
        "font_color".to_string(),
        "fill_color".to_string(),
        "conditional_formatting_rules".to_string(),
        "conditional_formatting_effective_style".to_string(),
    ];
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
    let oxreplay_current_bundle_views = vec![
        "visible_value".to_string(),
        "effective_display_text".to_string(),
        "formatting_view".to_string(),
        "conditional_formatting_view".to_string(),
        "replay_normalized_events".to_string(),
    ];
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
        oxfml_scope_required: observation_scope
            .observation_scope
            .oxfml_required_scope
            .clone(),
        oxxlplay_supported_surfaces,
        oxxlplay_missing_surfaces,
        oxreplay_required_views: observation_scope
            .observation_scope
            .oxreplay_required_views
            .clone(),
        oxreplay_current_bundle_views,
        oxreplay_missing_views,
    }
}

fn parse_oxreplay_mismatch_records(diff_report: &Value) -> Vec<OxReplayMismatchRecord> {
    diff_report
        .get("mismatches")
        .and_then(Value::as_array)
        .map(|mismatches| {
            mismatches
                .iter()
                .filter_map(parse_oxreplay_mismatch_record)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_oxreplay_mismatch_record(value: &Value) -> Option<OxReplayMismatchRecord> {
    let mismatch_kind = value
        .get("mismatch_kind")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)?;

    Some(OxReplayMismatchRecord {
        mismatch_kind,
        severity: value
            .get("severity")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        view_family: value
            .get("view_family")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        left_value_repr: json_value_to_repr(value.get("left_value")),
        right_value_repr: json_value_to_repr(value.get("right_value")),
        detail: value
            .get("detail")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    })
}

fn parse_oxreplay_explain_records(
    explain_stdout: &str,
) -> Result<Vec<OxReplayExplainRecord>, String> {
    if explain_stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    let explain_report = parse_json_text(explain_stdout, "OxReplay explain stdout")?;
    Ok(explain_report
        .get("records")
        .and_then(Value::as_array)
        .map(|records| {
            records
                .iter()
                .filter_map(|record| {
                    let mismatch_kind = record
                        .get("mismatch_kind")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)?;
                    Some(OxReplayExplainRecord {
                        query_id: record
                            .get("query_id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        summary: record
                            .get("summary")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        mismatch_kind,
                        severity: record
                            .get("severity")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        view_family: record
                            .get("view_family")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        left_value_repr: json_value_to_repr(record.get("left_value")),
                        right_value_repr: json_value_to_repr(record.get("right_value")),
                        detail: record
                            .get("detail")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn json_value_to_repr(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

pub fn replay_display_comparison_summary(
    replay_mismatch_records: &[OxReplayMismatchRecord],
    fallback_left: Option<&str>,
    fallback_right: Option<&str>,
) -> Option<String> {
    if let Some(display_record) = replay_mismatch_records
        .iter()
        .find(|record| record.view_family.as_deref() == Some("effective_display_text"))
    {
        let left = display_record
            .left_value_repr
            .clone()
            .or_else(|| fallback_left.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        let right = display_record
            .right_value_repr
            .clone()
            .or_else(|| fallback_right.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        return Some(format!(
            "Display divergence (effective_display_text): OxFml {left} vs Excel {right}"
        ));
    }

    if let Some(value_record) = replay_mismatch_records.iter().find(|record| {
        record.view_family.as_deref() == Some("visible_value")
            || record.mismatch_kind == "visible_value"
            || record.mismatch_kind == "view_value"
    }) {
        let left = value_record
            .left_value_repr
            .clone()
            .or_else(|| fallback_left.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        let right = value_record
            .right_value_repr
            .clone()
            .or_else(|| fallback_right.map(ToOwned::to_owned))
            .unwrap_or_else(|| "<unavailable>".to_string());
        return Some(format!(
            "Visible value divergence: OxFml {left} vs Excel {right}"
        ));
    }

    match (fallback_left, fallback_right) {
        (Some(left), Some(right)) if left != right => {
            Some(format!("Display divergence: OxFml {left} vs Excel {right}"))
        }
        _ => None,
    }
}

pub fn replay_projection_coverage_gap_summaries(
    replay_mismatch_records: &[OxReplayMismatchRecord],
) -> Vec<String> {
    replay_mismatch_records
        .iter()
        .filter(|record| record.mismatch_kind == "projection_coverage_gap")
        .map(|record| match (record.view_family.as_deref(), record.detail.as_deref()) {
            (Some(view_family), Some(detail)) => {
                format!("Projection coverage gap ({view_family}): {detail}")
            }
            (Some(view_family), None) => {
                format!(
                    "Projection coverage gap ({view_family}): comparison family is missing on one side."
                )
            }
            (None, Some(detail)) => format!("Projection coverage gap: {detail}"),
            (None, None) => "Projection coverage gap: comparison family is missing on one side."
                .to_string(),
        })
        .collect()
}

#[cfg(feature = "oxfml-live")]
fn summarize_excel_capture(capture_path: PathBuf) -> Result<ExcelObservationSummary, String> {
    let capture_json = read_json_file(&capture_path)?;
    let surfaces = capture_json
        .get("surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "capture file `{}` did not contain a `surfaces` array",
                capture_path.display()
            )
        })?;

    let mut observed_value_repr = None;
    let mut effective_display_text = None;
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
            "effective_display_text" => {
                effective_display_text = value_repr;
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
        effective_display_text,
        observed_formula_repr,
        capture_status,
    })
}

fn preferred_excel_display_repr(summary: &ExcelObservationSummary) -> Option<&str> {
    summary
        .effective_display_text
        .as_deref()
        .or(summary.observed_value_repr.as_deref())
}

#[cfg(feature = "oxfml-live")]
fn serialize_replay_projection(
    projection: &oxfml_core::consumer::replay::ReplayProjectionResult,
) -> Value {
    json!({
        "source_artifact_family": projection.source_artifact_family,
        "source_schema_id": projection.source_schema_id,
        "source_fixture_family": projection.source_fixture_family,
        "source_case_id": projection.source_case_id,
        "source_case_ids": projection.source_case_ids,
        "shared_scenario_alias": projection.shared_scenario_alias,
        "formula_stable_id": projection.formula_stable_id,
        "session_id": projection.session_id,
        "library_context_snapshot_ref": projection.library_context_snapshot_ref.as_ref().map(|value| {
            json!({
                "snapshot_id": value.snapshot_id,
                "snapshot_version": value.snapshot_version
            })
        }),
        "typed_query_bundle_spec": projection.typed_query_bundle_spec.as_ref().map(|value| format!("{value:?}")),
        "registry_pin": projection.registry_pin,
        "witness_id": projection.witness_id,
        "witness_lifecycle_state": projection.witness_lifecycle_state,
        "retention_policy_id": projection.retention_policy_id,
        "source_bundle_ref": projection.source_bundle_ref,
        "reduction_manifest_ref": projection.reduction_manifest_ref,
        "phase": projection.phase,
        "candidate_result_id": projection.candidate_result_id,
        "commit_decision_kind": projection.commit_decision_kind,
        "trace_event_kinds": projection.trace_event_kinds,
        "comparison_views": projection.comparison_views.as_ref().map(|views| serialize_comparison_views(views)).unwrap_or_default(),
        "verification_publication_surface": projection.verification_publication_surface.as_ref().map(serialize_verification_publication_surface),
    })
}

#[cfg(feature = "oxfml-live")]
fn serialize_comparison_views(
    comparison_views: &[oxfml_core::consumer::replay::ReplayComparisonView],
) -> Value {
    Value::Array(
        comparison_views
            .iter()
            .map(|view| {
                json!({
                    "view_family": view.view_family,
                    "value": view.value
                })
            })
            .collect(),
    )
}

#[cfg(feature = "oxfml-live")]
fn serialize_verification_publication_surface(surface: &VerificationPublicationSurface) -> Value {
    json!({
        "entered_cell_text": surface.entered_cell_text,
        "typed_value": {
            "value_kind": surface.typed_value.value_kind,
            "worksheet_value_class": format!("{:?}", surface.typed_value.worksheet_value_class),
            "payload": format!("{:?}", surface.typed_value.payload),
            "error_kind": surface.typed_value.error_kind,
        },
        "visible_value_text": surface.visible_value_text,
        "effective_display_text": surface.effective_display_text,
        "format_profile": surface.format_profile,
        "locale_format_context": surface.locale_format_context.as_ref().map(serialize_locale_format_context_surface),
        "date1904": surface.date1904,
        "number_format_code": surface.number_format_code,
        "style_id": surface.style_id,
        "style_hierarchy": surface.style_hierarchy,
        "format_dependency_facts": surface.format_dependency_facts.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>(),
        "format_delta": surface.format_delta.as_ref().map(|value| format!("{value:?}")),
        "display_delta": surface.display_delta.as_ref().map(|value| format!("{value:?}")),
        "returned_value_surface": format!("{:?}", surface.returned_value_surface),
        "presentation_hint": surface.presentation_hint.as_ref().map(|value| format!("{value:?}")),
        "font_color": surface.font_color,
        "fill_color": surface.fill_color,
        "effective_font_color": surface.effective_font_color,
        "effective_fill_color": surface.effective_fill_color,
        "conditional_formatting_rules": surface.conditional_formatting_rules.iter().map(serialize_verification_conditional_formatting_rule).collect::<Vec<_>>(),
        "conditional_formatting_target_ranges": surface.conditional_formatting_target_ranges,
        "conditional_formatting_rule_kind": surface.conditional_formatting_rule_kind,
        "conditional_formatting_operator": surface.conditional_formatting_operator,
        "conditional_formatting_thresholds": surface.conditional_formatting_thresholds,
        "conditional_formatting_applies": surface.conditional_formatting_applies,
        "conditional_formatting_effective_font_color": surface.conditional_formatting_effective_font_color,
        "conditional_formatting_effective_fill_color": surface.conditional_formatting_effective_fill_color,
        "conditional_formatting_effective_display": surface.conditional_formatting_effective_display,
    })
}

#[cfg(feature = "oxfml-live")]
fn serialize_locale_format_context_surface(surface: &LocaleFormatContextSurface) -> Value {
    json!({
        "locale_profile_id": surface.locale_profile_id,
        "date_system": surface.date_system,
        "decimal_separator": surface.decimal_separator,
        "thousands_separator": surface.thousands_separator,
        "currency_symbol": surface.currency_symbol,
        "date_separator": surface.date_separator,
        "time_separator": surface.time_separator,
    })
}

#[cfg(feature = "oxfml-live")]
fn serialize_verification_conditional_formatting_rule(
    rule: &VerificationConditionalFormattingRule,
) -> Value {
    json!({
        "target_ranges": rule.target_ranges,
        "rule_kind": rule.rule_kind,
        "operator": rule.operator,
        "thresholds": rule.thresholds,
        "font_color": rule.font_color,
        "fill_color": rule.fill_color,
        "effective_display_text": rule.effective_display_text,
        "applies": rule.applies,
        "effective_font_color": rule.effective_font_color,
        "effective_fill_color": rule.effective_fill_color,
    })
}

#[cfg(feature = "oxfml-live")]
fn build_discrepancy_summary(
    comparison_status: ProgrammaticComparisonStatus,
    visible_output_match: Option<bool>,
    replay_mismatch_records: &[OxReplayMismatchRecord],
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
            let display_summary = replay_display_comparison_summary(
                replay_mismatch_records,
                Some(&oxfml_value),
                Some(&excel_value),
            );
            let projection_gap_summary =
                replay_projection_coverage_gap_summaries(replay_mismatch_records);

            if display_summary.is_some() || !projection_gap_summary.is_empty() {
                let mut parts = Vec::new();
                if let Some(display_summary) = display_summary {
                    parts.push(display_summary);
                }
                if !projection_gap_summary.is_empty() {
                    parts.push(projection_gap_summary.join(" | "));
                }
                return Some(parts.join(" | "));
            }

            if visible_output_match == Some(true) {
                let mismatch_kinds = if replay_mismatch_records.is_empty() {
                    "unknown_replay_mismatch".to_string()
                } else {
                    replay_mismatch_records
                        .iter()
                        .map(|record| record.mismatch_kind.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
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
        return format!(r#"<Cell><Data ss:Type="Number">{}</Data></Cell>"#, number);
    }

    if entered_cell_text.eq_ignore_ascii_case("true")
        || entered_cell_text.eq_ignore_ascii_case("false")
    {
        let boolean_value = if entered_cell_text.eq_ignore_ascii_case("true") {
            "1"
        } else {
            "0"
        };
        return format!(r#"<Cell><Data ss:Type="Boolean">{boolean_value}</Data></Cell>"#);
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
        .map_err(|error| {
            format!(
                "failed to resolve absolute path for `{}`: {error}",
                path.display()
            )
        })
}

#[cfg(test)]
mod consumer_shape_tests {
    use super::*;

    #[test]
    fn replay_display_comparison_summary_prefers_effective_display_family() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "effective_display_text".to_string(),
                severity: Some("informational".to_string()),
                view_family: Some("effective_display_text".to_string()),
                left_value_repr: Some("6".to_string()),
                right_value_repr: Some("$6.00".to_string()),
                detail: Some("comparison view values diverged".to_string()),
            }],
            Some("6"),
            Some("$6.00"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00")
        );
    }

    #[test]
    fn replay_projection_coverage_gap_summaries_keep_family_specific_labels() {
        let summaries = replay_projection_coverage_gap_summaries(&[
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                detail: Some(
                    "comparison view family `formatting_view` is missing on one side".to_string(),
                ),
            },
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("conditional_formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("[{\"range\":\"A1\"}]".to_string()),
                detail: Some(
                    "comparison view family `conditional_formatting_view` is missing on one side"
                        .to_string(),
                ),
            },
        ]);

        assert_eq!(summaries.len(), 2);
        assert_eq!(
            summaries[0],
            "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
        );
        assert_eq!(
            summaries[1],
            "Projection coverage gap (conditional_formatting_view): comparison view family `conditional_formatting_view` is missing on one side"
        );
    }

    #[test]
    fn replay_display_comparison_summary_keeps_legacy_view_value_fallback() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "view_value".to_string(),
                severity: Some("semantic".to_string()),
                view_family: None,
                left_value_repr: None,
                right_value_repr: None,
                detail: None,
            }],
            Some("6"),
            Some("7"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Visible value divergence: OxFml 6 vs Excel 7")
        );
    }

    #[test]
    fn parse_oxreplay_records_keep_machine_readable_view_family_shape() {
        let diff_report = json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "formatting_view",
                    "right_value": { "number_format_code": "$#,##0.00" },
                    "detail": "comparison view family `formatting_view` is missing on one side"
                }
            ]
        });
        let explain_stdout = serde_json::to_string(&json!({
            "records": [
                {
                    "query_id": "explain-01",
                    "summary": "comparison diverged on `effective_display_text`",
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "query_id": "explain-02",
                    "summary": "comparison view family `conditional_formatting_view` is missing on one side",
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "conditional_formatting_view",
                    "right_value": [{ "range": "A1" }],
                    "detail": "comparison view family `conditional_formatting_view` is missing on one side"
                }
            ]
        }))
        .expect("json text");

        let mismatch_records = parse_oxreplay_mismatch_records(&diff_report);
        let explain_records =
            parse_oxreplay_explain_records(&explain_stdout).expect("explain records");

        assert_eq!(mismatch_records.len(), 2);
        assert_eq!(
            mismatch_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(mismatch_records[0].left_value_repr.as_deref(), Some("6"));
        assert_eq!(
            mismatch_records[1].view_family.as_deref(),
            Some("formatting_view")
        );
        assert_eq!(
            mismatch_records[1].right_value_repr.as_deref(),
            Some("{\"number_format_code\":\"$#,##0.00\"}")
        );
        assert_eq!(explain_records.len(), 2);
        assert_eq!(
            explain_records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(
            explain_records[1].view_family.as_deref(),
            Some("conditional_formatting_view")
        );
        assert_eq!(
            explain_records[1].right_value_repr.as_deref(),
            Some("[{\"range\":\"A1\"}]")
        );
    }
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
            self.calls
                .lock()
                .expect("calls")
                .push("oxxlplay_capture".to_string());
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
                        },
                        {
                            "surface": {
                                "surface_id": "sheet1_a1_display",
                                "surface_kind": "effective_display_text",
                                "locator": "Sheet1!A1",
                                "required": true
                            },
                            "status": "direct",
                            "value_repr": if self.diff_equivalent { "6" } else { "$7.00" },
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
            self.calls
                .lock()
                .expect("calls")
                .push("validate_bundle".to_string());
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
                    if self.diff_equivalent {
                        "true"
                    } else {
                        "false"
                    }
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
            self.calls
                .lock()
                .expect("calls")
                .push("explain".to_string());
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
        assert!(output_root
            .join("verification-bundle-report.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-1")
            .join("comparison-summary.json")
            .is_file());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn verification_batch_emits_programmatic_formula_scenario_for_formula_cases() {
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
                case_id: "case-formula".to_string(),
                entered_cell_text: "=LET(a,{1,2,3},b,{4,5,6},SUM(a*b))".to_string(),
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
        let case_dir = output_root.join("cases").join("case-formula");
        let scenario: Value = serde_json::from_str(
            &fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"),
        )
        .expect("scenario parse");

        assert_eq!(scenario["workbook_kind"], "programmatic-formula");
        assert_eq!(
            scenario["entered_cell_text"],
            "=LET(a,{1,2,3},b,{4,5,6},SUM(a*b))"
        );
        assert_eq!(scenario["workbook_ref"], "./workbook.xml");
        assert!(case_dir.join("workbook.xml").is_file());

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
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("xml-cell-extract.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("required-observation-scope.json")
            .is_file());
        assert!(output_root
            .join("cases")
            .join("case-xml")
            .join("upstream-gap-report.json")
            .is_file());
        let case_dir = output_root.join("cases").join("case-xml");
        let scenario: Value =
            serde_json::from_str(&fs::read_to_string(case_dir.join("scenario.json")).expect("scenario json"))
                .expect("scenario parse");
        assert_eq!(scenario["workbook_kind"], "spreadsheetml-2003-import");
        assert!(scenario.get("entered_cell_text").is_none());
        assert!(case_dir.join("workbook.xml").is_file());

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn replay_display_comparison_summary_prefers_effective_display_family() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "effective_display_text".to_string(),
                severity: Some("informational".to_string()),
                view_family: Some("effective_display_text".to_string()),
                left_value_repr: Some("6".to_string()),
                right_value_repr: Some("$6.00".to_string()),
                detail: Some("comparison view values diverged".to_string()),
            }],
            Some("6"),
            Some("$6.00"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00")
        );
    }

    #[test]
    fn replay_projection_coverage_gap_summaries_keep_family_specific_labels() {
        let summaries = replay_projection_coverage_gap_summaries(&[
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("{\"number_format_code\":\"$#,##0.00\"}".to_string()),
                detail: Some(
                    "comparison view family `formatting_view` is missing on one side".to_string(),
                ),
            },
            OxReplayMismatchRecord {
                mismatch_kind: "projection_coverage_gap".to_string(),
                severity: Some("coverage".to_string()),
                view_family: Some("conditional_formatting_view".to_string()),
                left_value_repr: None,
                right_value_repr: Some("[{\"range\":\"A1\"}]".to_string()),
                detail: Some(
                    "comparison view family `conditional_formatting_view` is missing on one side"
                        .to_string(),
                ),
            },
        ]);

        assert_eq!(summaries.len(), 2);
        assert_eq!(
            summaries[0],
            "Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side"
        );
        assert_eq!(
            summaries[1],
            "Projection coverage gap (conditional_formatting_view): comparison view family `conditional_formatting_view` is missing on one side"
        );
    }

    #[test]
    fn replay_display_comparison_summary_keeps_legacy_view_value_fallback() {
        let summary = replay_display_comparison_summary(
            &[OxReplayMismatchRecord {
                mismatch_kind: "view_value".to_string(),
                severity: Some("semantic".to_string()),
                view_family: None,
                left_value_repr: None,
                right_value_repr: None,
                detail: None,
            }],
            Some("6"),
            Some("7"),
        );

        assert_eq!(
            summary.as_deref(),
            Some("Visible value divergence: OxFml 6 vs Excel 7")
        );
    }

    #[test]
    fn discrepancy_summary_combines_display_divergence_and_projection_gaps() {
        let summary = build_discrepancy_summary(
            ProgrammaticComparisonStatus::Mismatched,
            Some(false),
            &[
                OxReplayMismatchRecord {
                    mismatch_kind: "effective_display_text".to_string(),
                    severity: Some("informational".to_string()),
                    view_family: Some("effective_display_text".to_string()),
                    left_value_repr: Some("6".to_string()),
                    right_value_repr: Some("$6.00".to_string()),
                    detail: Some("comparison view values diverged".to_string()),
                },
                OxReplayMismatchRecord {
                    mismatch_kind: "projection_coverage_gap".to_string(),
                    severity: Some("coverage".to_string()),
                    view_family: Some("formatting_view".to_string()),
                    left_value_repr: None,
                    right_value_repr: None,
                    detail: Some(
                        "comparison view family `formatting_view` is missing on one side"
                            .to_string(),
                    ),
                },
            ],
            &OxfmlVerificationSummary {
                evaluation_summary: Some("Number · 6".to_string()),
                effective_display_summary: Some("6".to_string()),
                blocked_reason: None,
                parse_status: Some("Valid".to_string()),
                green_tree_key: Some("green-1".to_string()),
            },
            &ExcelObservationSummary {
                observed_value_repr: Some("$6.00".to_string()),
                effective_display_text: Some("$6.00".to_string()),
                observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
                capture_status: "captured".to_string(),
            },
        );

        assert_eq!(
            summary.as_deref(),
            Some("Display divergence (effective_display_text): OxFml 6 vs Excel $6.00 | Projection coverage gap (formatting_view): comparison view family `formatting_view` is missing on one side")
        );
    }

    #[test]
    fn parse_oxreplay_mismatch_records_keeps_view_family_and_values() {
        let diff_report = json!({
            "equivalent": false,
            "mismatches": [
                {
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "formatting_view",
                    "right_value": { "number_format_code": "$#,##0.00" },
                    "detail": "comparison view family `formatting_view` is missing on one side"
                }
            ]
        });

        let records = parse_oxreplay_mismatch_records(&diff_report);

        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(records[0].left_value_repr.as_deref(), Some("6"));
        assert_eq!(records[0].right_value_repr.as_deref(), Some("$6.00"));
        assert_eq!(records[1].view_family.as_deref(), Some("formatting_view"));
        assert_eq!(
            records[1].right_value_repr.as_deref(),
            Some("{\"number_format_code\":\"$#,##0.00\"}")
        );
    }

    #[test]
    fn parse_oxreplay_explain_records_keeps_machine_readable_family_shape() {
        let explain_stdout = serde_json::to_string(&json!({
            "records": [
                {
                    "query_id": "explain-01",
                    "summary": "comparison diverged on `effective_display_text`",
                    "mismatch_kind": "effective_display_text",
                    "severity": "informational",
                    "view_family": "effective_display_text",
                    "left_value": "6",
                    "right_value": "$6.00",
                    "detail": "comparison view values diverged"
                },
                {
                    "query_id": "explain-02",
                    "summary": "comparison view family `conditional_formatting_view` is missing on one side",
                    "mismatch_kind": "projection_coverage_gap",
                    "severity": "coverage",
                    "view_family": "conditional_formatting_view",
                    "right_value": [{ "range": "A1" }],
                    "detail": "comparison view family `conditional_formatting_view` is missing on one side"
                }
            ]
        }))
        .expect("json text");

        let records = parse_oxreplay_explain_records(&explain_stdout).expect("explain records");

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].query_id.as_deref(), Some("explain-01"));
        assert_eq!(
            records[0].view_family.as_deref(),
            Some("effective_display_text")
        );
        assert_eq!(
            records[1].view_family.as_deref(),
            Some("conditional_formatting_view")
        );
        assert_eq!(
            records[1].right_value_repr.as_deref(),
            Some("[{\"range\":\"A1\"}]")
        );
    }

    #[test]
    fn summarize_excel_capture_reads_effective_display_text_when_present() {
        let path = std::env::temp_dir().join(format!(
            "dnaonecalc-capture-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix epoch")
                .as_nanos()
        ));

        write_json_file(
            &path,
            &json!({
                "surfaces": [
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_value",
                            "surface_kind": "cell_value",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "value_repr": "6",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    },
                    {
                        "surface": {
                            "surface_id": "sheet1_a1_display",
                            "surface_kind": "effective_display_text",
                            "locator": "Input!A1",
                            "required": true
                        },
                        "status": "direct",
                        "value_repr": "$6.00",
                        "capture_loss": "none",
                        "uncertainty": "none"
                    }
                ]
            }),
        )
        .expect("capture json");

        let summary = summarize_excel_capture(path.clone()).expect("capture summary");

        assert_eq!(summary.observed_value_repr.as_deref(), Some("6"));
        assert_eq!(summary.effective_display_text.as_deref(), Some("$6.00"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn preferred_excel_display_repr_uses_effective_display_text_before_observed_value() {
        let summary = ExcelObservationSummary {
            observed_value_repr: Some("6".to_string()),
            effective_display_text: Some("$6.00".to_string()),
            observed_formula_repr: Some("=SUM(1,2,3)".to_string()),
            capture_status: "captured".to_string(),
        };

        assert_eq!(preferred_excel_display_repr(&summary), Some("$6.00"));
    }
}
