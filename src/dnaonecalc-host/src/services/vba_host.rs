use std::cell::RefCell;
use std::collections::BTreeMap;

use oxfml_core::interface::{
    HostFunctionInvocation, HostFunctionProvider, HostFunctionProviderError,
    LibraryContextProvider, LibraryContextSnapshotRef,
};
use oxfml_core::semantics::{
    LibraryAvailabilityState, LibraryContextSnapshot, LibraryContextSnapshotEntry,
    RegistrationSourceKind,
};
use oxfunc_core::value::{CalcValue, CoreValue, ExcelText, WorksheetErrorCode};
use oxvba_compiler::{ModuleKind, VbaType};
use oxvba_host::{
    HostCallContext, HostCaller, HostConfig, PreparedVbaProject, ProjectModuleText, ProjectSource,
    UdfAdmissionPolicy, UdfAdmissionReport, VbaHost, VbaHostOptions, W093RegistrationRequest,
};
use oxvba_runtime::{VarType, Variant};

// ---------------------------------------------------------------------------
// Source-level input types (used by vba_udf_verification and load paths)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaSourceModule {
    pub module_name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaSourceProject {
    pub project_name: String,
    pub modules: Vec<VbaSourceModule>,
}

// ---------------------------------------------------------------------------
// Association model (unchanged — DnaOneCalc-owned)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VbaProjectAssociationScope {
    Workspace,
    FormulaSpace { formula_space_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VbaProjectLoadStatus {
    Unloaded,
    Loaded,
    Failed { diagnostic: String },
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaProjectAssociation {
    pub association_id: String,
    pub scope: VbaProjectAssociationScope,
    pub project_ref: String,
    pub project_identity: Option<String>,
    pub root_object_name: String,
    pub enabled: bool,
    pub source_fingerprint: Option<String>,
    pub last_load_status: VbaProjectLoadStatus,
}

impl VbaProjectAssociation {
    pub fn workspace_source(
        association_id: impl Into<String>,
        project_ref: impl Into<String>,
    ) -> Self {
        Self {
            association_id: association_id.into(),
            scope: VbaProjectAssociationScope::Workspace,
            project_ref: project_ref.into(),
            project_identity: None,
            root_object_name: "Application".to_string(),
            enabled: true,
            source_fingerprint: None,
            last_load_status: VbaProjectLoadStatus::Unloaded,
        }
    }
}

// ---------------------------------------------------------------------------
// UDF admission / registration types (DnaOneCalc-owned, now backed by W093)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VbaUdfAdmissionStatus {
    Admitted,
    Rejected { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VbaUdfRegistration {
    pub association_id: String,
    pub formula_name: String,
    pub callable_id: String,
    pub registration: W093RegistrationRequest,
    pub admission_status: VbaUdfAdmissionStatus,
    pub type_map_status: String,
    pub excel_oracle_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaUdfCandidate {
    pub association_id: String,
    pub project_name: String,
    pub module_name: String,
    pub procedure_name: String,
    pub callable_id: String,
    pub admission_status: VbaUdfAdmissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaUdfExcelOracleExpectation {
    pub case_id: String,
    pub formula_call: String,
    pub oracle_ref: String,
    pub expected_number: String,
}

// ---------------------------------------------------------------------------
// VbaHostRuntime — the core runtime, now consuming OxVba directly
// ---------------------------------------------------------------------------

pub struct VbaHostRuntime {
    association: VbaProjectAssociation,
    prepared: RefCell<PreparedVbaProject>,
    candidates: Vec<VbaUdfCandidate>,
    registrations: Vec<VbaUdfRegistration>,
    snapshot: LibraryContextSnapshot,
}

impl VbaHostRuntime {
    pub fn load_source_project(
        association: VbaProjectAssociation,
        project: VbaSourceProject,
        application_version: &str,
        oracle_ref: Option<String>,
    ) -> Result<Self, String> {
        let module_texts = build_module_texts(&project, application_version);
        let (loaded, project_name) = load_via_vba_host(module_texts)?;
        let admission_report = dnaonecalc_udf_admission_policy().admit(loaded.reflection());
        let prepared = loaded.prepare().map_err(format_host_diagnostic)?;
        Self::from_prepared(
            association,
            prepared,
            admission_report,
            Some(project_name),
            oracle_ref,
        )
    }

    pub fn load_source_project_for_evaluation(
        association: VbaProjectAssociation,
        project: VbaSourceProject,
        application_version: &str,
    ) -> Result<Self, String> {
        Self::load_source_project(association, project, application_version, None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_project_path_for_evaluation(
        mut association: VbaProjectAssociation,
        project_path: impl AsRef<std::path::Path>,
        application_version: &str,
    ) -> Result<Self, String> {
        let project_path = project_path.as_ref();
        let project_ref = project_path.display().to_string();
        association.project_ref = project_ref;
        if project_path.extension().and_then(|ext| ext.to_str()) == Some("bas") {
            let source = std::fs::read_to_string(project_path).map_err(|error| {
                format!(
                    "failed to read VBA module source `{}`: {error}",
                    project_path.display()
                )
            })?;
            let module_name = project_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .filter(|stem| !stem.trim().is_empty())
                .unwrap_or("Module1")
                .to_string();
            let project_name = project_path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("DnaVbaFixture")
                .to_string();
            return Self::load_source_project_for_evaluation(
                association,
                VbaSourceProject {
                    project_name,
                    modules: vec![VbaSourceModule {
                        module_name,
                        source,
                    }],
                },
                application_version,
            );
        }

        // .basproj path — load via oxvba_project, then feed as FileSet
        let loaded_workspace =
            oxvba_project::load_workspace_target(project_path).map_err(|error| {
                format!(
                    "failed to load OxVba workspace target `{}`: {error}",
                    project_path.display()
                )
            })?;
        let project_name = loaded_workspace.manifest.project_name.clone();

        // Convert manifest modules to ProjectModuleTexts, prepending Application class
        let mut module_texts = vec![application_class_module_text(application_version)];
        for module in &loaded_workspace.manifest.modules {
            module_texts.push(ProjectModuleText {
                name_hint: Some(module.module_name.clone()),
                kind_hint: Some(module.module_kind),
                text: module.source.clone(),
            });
        }

        let host = VbaHost::new(vba_host_options());
        let loaded = host
            .load_project(ProjectSource::ModuleTexts(module_texts))
            .map_err(format_host_diagnostic)?;
        let admission_report = dnaonecalc_udf_admission_policy().admit(loaded.reflection());
        let prepared = loaded.prepare().map_err(format_host_diagnostic)?;
        association.project_identity = Some(project_name.clone());
        Self::from_prepared(
            association,
            prepared,
            admission_report,
            Some(project_name),
            None,
        )
    }

    fn from_prepared(
        mut association: VbaProjectAssociation,
        prepared: PreparedVbaProject,
        admission_report: UdfAdmissionReport,
        project_identity: Option<String>,
        oracle_ref: Option<String>,
    ) -> Result<Self, String> {
        let mut candidates = Vec::new();
        let mut registrations = Vec::new();

        for admitted in &admission_report.admitted {
            let reg = &admitted.registration;
            let meta = &reg.callable_metadata;
            // Skip the synthetic Application class module
            if meta.module_name.eq_ignore_ascii_case("Application") {
                continue;
            }
            candidates.push(VbaUdfCandidate {
                association_id: association.association_id.clone(),
                project_name: reg.source_identity.project_id.clone(),
                module_name: meta.module_name.clone(),
                procedure_name: meta.public_name.clone(),
                callable_id: reg.invocation_target.callable_id.clone(),
                admission_status: VbaUdfAdmissionStatus::Admitted,
            });
            registrations.push(VbaUdfRegistration {
                association_id: association.association_id.clone(),
                formula_name: meta.public_name.clone(),
                callable_id: reg.invocation_target.callable_id.clone(),
                registration: reg.clone(),
                admission_status: VbaUdfAdmissionStatus::Admitted,
                type_map_status: reg.invocation_target.conversion_lane.clone(),
                excel_oracle_ref: oracle_ref.clone(),
            });
        }

        for rejected in &admission_report.rejected {
            candidates.push(VbaUdfCandidate {
                association_id: association.association_id.clone(),
                project_name: String::new(),
                module_name: String::new(),
                procedure_name: rejected.procedure_name.clone(),
                callable_id: rejected.callable_id.clone(),
                admission_status: VbaUdfAdmissionStatus::Rejected {
                    reason: rejected.message.clone(),
                },
            });
        }

        association.project_identity = project_identity;
        association.last_load_status = VbaProjectLoadStatus::Loaded;
        let snapshot = library_snapshot_for_registrations(&association, &registrations);
        Ok(Self {
            association,
            prepared: RefCell::new(prepared),
            candidates,
            registrations,
            snapshot,
        })
    }

    pub fn association(&self) -> &VbaProjectAssociation {
        &self.association
    }

    pub fn registrations(&self) -> &[VbaUdfRegistration] {
        &self.registrations
    }

    pub fn candidates(&self) -> &[VbaUdfCandidate] {
        &self.candidates
    }

    pub fn invoke_registered_udf(
        &self,
        function_name: &str,
        args: &[CalcValue],
    ) -> Result<CalcValue, String> {
        let registration = self
            .registrations
            .iter()
            .find(|registration| {
                registration
                    .formula_name
                    .eq_ignore_ascii_case(function_name)
            })
            .ok_or_else(|| format!("VBA UDF `{function_name}` is not admitted"))?;

        let variant_args = args
            .iter()
            .map(eval_value_to_variant)
            .collect::<Result<Vec<_>, _>>()?;
        let context = HostCallContext {
            caller: Some(HostCaller {
                source_system: "DnaOneCalc".to_string(),
                display_text: Some("A1".to_string()),
                stable_id: None,
                metadata: BTreeMap::new(),
            }),
            locale_id: None,
            metadata: BTreeMap::new(),
        };
        let mut prepared = self
            .prepared
            .try_borrow_mut()
            .map_err(|_| "VBA runtime session is already borrowed".to_string())?;
        let result = prepared
            .invoke_callable_variant(&registration.callable_id, context, &variant_args)
            .map_err(format_host_diagnostic)?;

        variant_to_eval_value(result.value)
    }
}

impl HostFunctionProvider for VbaHostRuntime {
    fn invoke_host_function(
        &self,
        invocation: &HostFunctionInvocation,
    ) -> Result<CalcValue, HostFunctionProviderError> {
        self.invoke_registered_udf(&invocation.function_name, &invocation.args)
            .map_err(HostFunctionProviderError::new)
    }
}

impl LibraryContextProvider for VbaHostRuntime {
    fn current_snapshot(&self) -> LibraryContextSnapshot {
        self.snapshot.clone()
    }

    fn snapshot_by_identity(
        &self,
        snapshot_ref: &LibraryContextSnapshotRef,
    ) -> Option<LibraryContextSnapshot> {
        (snapshot_ref == &LibraryContextSnapshotRef::from(&self.snapshot))
            .then(|| self.snapshot.clone())
    }
}

// ---------------------------------------------------------------------------
// VbaHost construction and loading helpers
// ---------------------------------------------------------------------------

fn format_host_diagnostic(err: oxvba_host::HostDiagnostic) -> String {
    format!("{}: {}", err.code, err.message)
}

fn vba_host_options() -> VbaHostOptions {
    VbaHostOptions {
        host_config: HostConfig { enable_jit: false },
    }
}

fn dnaonecalc_udf_admission_policy() -> UdfAdmissionPolicy {
    UdfAdmissionPolicy {
        allowed_scalar_types: vec![
            VbaType::Variant,
            VbaType::Boolean,
            VbaType::Byte,
            VbaType::Integer,
            VbaType::Long,
            VbaType::LongLong,
            VbaType::Single,
            VbaType::Double,
            VbaType::Currency,
            VbaType::Date,
            VbaType::String,
        ],
        ..UdfAdmissionPolicy::default()
    }
}

fn load_via_vba_host(
    module_texts: Vec<ProjectModuleText>,
) -> Result<(oxvba_host::LoadedVbaProject, String), String> {
    let host = VbaHost::new(vba_host_options());
    let loaded = host
        .load_project(ProjectSource::ModuleTexts(module_texts))
        .map_err(format_host_diagnostic)?;
    let project_name = loaded.reflection().identity.project_name.clone();
    Ok((loaded, project_name))
}

fn build_module_texts(
    project: &VbaSourceProject,
    application_version: &str,
) -> Vec<ProjectModuleText> {
    let mut texts = Vec::with_capacity(project.modules.len() + 1);
    texts.push(application_class_module_text(application_version));
    for module in &project.modules {
        texts.push(ProjectModuleText {
            name_hint: Some(module.module_name.clone()),
            kind_hint: None, // Procedural default
            text: module.source.clone(),
        });
    }
    texts
}

fn application_class_module_text(application_version: &str) -> ProjectModuleText {
    let escaped = application_version.replace('"', "\"\"");
    ProjectModuleText {
        name_hint: Some("Application".to_string()),
        kind_hint: Some(ModuleKind::Class),
        text: format!(
            concat!(
                "Attribute VB_Name = \"Application\"\n",
                "Attribute VB_PredeclaredId = True\n",
                "Public Property Get Version() As String\n",
                "Version = \"{}\"\n",
                "End Property\n",
            ),
            escaped
        ),
    }
}

// ---------------------------------------------------------------------------
// Type conversion helpers
// ---------------------------------------------------------------------------

fn eval_value_to_variant(value: &CalcValue) -> Result<Variant, String> {
    if value.rich().is_some() {
        return Err("VBA UDF arguments do not admit rich CalcValue payloads".to_string());
    }
    match value.core() {
        CoreValue::Number(number) => Ok(Variant::from_f64(*number)),
        CoreValue::Text(text) => Ok(Variant::from_string(text.to_string_lossy())),
        CoreValue::Logical(value) => Ok(Variant::from_bool(*value)),
        CoreValue::Empty | CoreValue::Missing => Ok(Variant::empty()),
        CoreValue::Error(code) => Ok(Variant::from_error_code(
            worksheet_error_to_variant_error_code(*code),
        )),
        CoreValue::Array(_) => {
            Err("VBA UDF arguments do not yet admit CalcValue array payloads".to_string())
        }
        CoreValue::Reference(_) => {
            Err("VBA UDF arguments do not yet admit CalcValue reference payloads".to_string())
        }
    }
}

fn variant_to_eval_value(value: Variant) -> Result<CalcValue, String> {
    match value.vtype() {
        VarType::Empty | VarType::Null => Ok(CalcValue::empty()),
        VarType::Integer => value
            .as_i16()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant Integer return could not be read".to_string()),
        VarType::SignedByte => value
            .as_i8()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant SignedByte return could not be read".to_string()),
        VarType::Byte => value
            .as_u8()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant Byte return could not be read".to_string()),
        VarType::UnsignedInteger => value
            .as_u16()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant UnsignedInteger return could not be read".to_string()),
        VarType::Long => value
            .as_i32()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant Long return could not be read".to_string()),
        VarType::UnsignedLong | VarType::UnsignedInt => value
            .as_u32()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant unsigned Long return could not be read".to_string()),
        VarType::LongLong => value
            .as_i64()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant LongLong return could not be read".to_string()),
        VarType::UnsignedLongLong => value
            .as_u64()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant UnsignedLongLong return could not be read".to_string()),
        VarType::Single => value
            .as_f32()
            .map(|v| CalcValue::number(v as f64))
            .ok_or_else(|| "VBA Variant Single return could not be read".to_string()),
        VarType::Double => value
            .as_f64()
            .map(CalcValue::number)
            .ok_or_else(|| "VBA Variant Double return could not be read".to_string()),
        VarType::Currency => value
            .as_currency_scaled_i64()
            .map(|v| CalcValue::number(v as f64 / 10_000.0))
            .ok_or_else(|| "VBA Variant Currency return could not be read".to_string()),
        VarType::Date => value
            .as_date_f64()
            .map(CalcValue::number)
            .ok_or_else(|| "VBA Variant Date return could not be read".to_string()),
        VarType::Boolean => value
            .as_bool()
            .map(CalcValue::logical)
            .ok_or_else(|| "VBA Variant Boolean return could not be read".to_string()),
        VarType::String => value
            .as_bstr()
            .map(|v| {
                let text = v.as_str();
                CalcValue::text(ExcelText::from_interop_assignment(&text))
            })
            .ok_or_else(|| "VBA Variant String return could not be read".to_string()),
        VarType::Error => value
            .as_error_code()
            .map(|code| CalcValue::error(variant_error_code_to_worksheet_error(code)))
            .ok_or_else(|| "VBA Variant Error return could not be read".to_string()),
        other => Err(format!(
            "VBA UDF returned a Variant type not yet mapped to CalcValue: {other:?}"
        )),
    }
}

fn worksheet_error_to_variant_error_code(code: WorksheetErrorCode) -> i32 {
    match code {
        WorksheetErrorCode::Null => 2000,
        WorksheetErrorCode::Div0 => 2007,
        WorksheetErrorCode::Value => 2015,
        WorksheetErrorCode::Ref => 2023,
        WorksheetErrorCode::Name => 2029,
        WorksheetErrorCode::Num => 2036,
        WorksheetErrorCode::NA => 2042,
        WorksheetErrorCode::GettingData => 2043,
        WorksheetErrorCode::Spill => 2045,
        WorksheetErrorCode::Calc => 2050,
        WorksheetErrorCode::Field => 2049,
        WorksheetErrorCode::Busy => 2051,
        WorksheetErrorCode::Blocked => 2047,
        WorksheetErrorCode::Connect => 2046,
    }
}

fn variant_error_code_to_worksheet_error(code: i32) -> WorksheetErrorCode {
    match code {
        2000 => WorksheetErrorCode::Null,
        2007 => WorksheetErrorCode::Div0,
        2015 => WorksheetErrorCode::Value,
        2023 => WorksheetErrorCode::Ref,
        2029 => WorksheetErrorCode::Name,
        2036 => WorksheetErrorCode::Num,
        2042 => WorksheetErrorCode::NA,
        2043 => WorksheetErrorCode::GettingData,
        2045 => WorksheetErrorCode::Spill,
        2050 => WorksheetErrorCode::Calc,
        2049 => WorksheetErrorCode::Field,
        2051 => WorksheetErrorCode::Busy,
        2047 => WorksheetErrorCode::Blocked,
        2046 => WorksheetErrorCode::Connect,
        _ => WorksheetErrorCode::Value,
    }
}

// ---------------------------------------------------------------------------
// OxFml library context snapshot construction
// ---------------------------------------------------------------------------

fn library_snapshot_for_registrations(
    association: &VbaProjectAssociation,
    registrations: &[VbaUdfRegistration],
) -> LibraryContextSnapshot {
    LibraryContextSnapshot {
        snapshot_id: format!("dnaonecalc-vba-{}", association.association_id),
        snapshot_version: "vba-first-slice".to_string(),
        entries: registrations
            .iter()
            .map(|registration| LibraryContextSnapshotEntry {
                surface_name: registration.formula_name.clone(),
                canonical_id: Some(format!(
                    "FUNC.VBA.{}",
                    registration.formula_name.to_ascii_uppercase()
                )),
                surface_stable_id: Some(registration.callable_id.clone()),
                name_resolution_table_ref: Some(format!("vba:{}", association.association_id)),
                semantic_trait_profile_ref: Some("vba-udf-typed-scalar.v1".to_string()),
                gating_profile_ref: None,
                metadata_status: Some("host_registered".to_string()),
                special_interface_kind: None,
                admission_interface_kind: Some(
                    registration.registration.capability.policy_name.clone(),
                ),
                preparation_owner: Some("DnaOneCalc".to_string()),
                runtime_boundary_kind: Some("vba_host_callback".to_string()),
                interface_contract_ref: Some("dnaonecalc-vba-udf-typed-scalar".to_string()),
                registration_source_kind: RegistrationSourceKind::Vba,
                parse_bind_state: LibraryAvailabilityState::CatalogKnown,
                semantic_plan_state: LibraryAvailabilityState::CatalogKnown,
                runtime_capability_state: Some(LibraryAvailabilityState::CatalogKnown),
                post_dispatch_state: None,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxfml_core::format::oxfml_en_us_locale_context;
    use oxfml_core::interface::TypedContextQueryBundle;
    use oxfml_core::EvaluationBackend;
    use oxfunc_core::functions::rand_fn::RandomProvider;

    struct FixedRandomProvider {
        value: f64,
    }

    impl RandomProvider for FixedRandomProvider {
        fn random_unit(&self) -> f64 {
            self.value
        }
    }

    static FIXED_RANDOM_PROVIDER_05: FixedRandomProvider = FixedRandomProvider { value: 0.5 };

    fn add_them_project(extra_source: &str) -> VbaSourceProject {
        VbaSourceProject {
            project_name: "DnaVbaFixture".to_string(),
            modules: vec![VbaSourceModule {
                module_name: "Module1".to_string(),
                source: format!(
                    "{}{}",
                    concat!(
                        "Public Function AddThem(val1 As Double, val2 As Double) As Double\n",
                        "AddThem = val1 + val2\n",
                        "End Function\n"
                    ),
                    extra_source
                ),
            }],
        }
    }

    #[test]
    fn vba_project_association_round_trips_minimal_state() {
        let association =
            VbaProjectAssociation::workspace_source("vba-assoc-1", "fixtures/vba/AddThem.basproj");
        let json = serde_json::to_string(&association).expect("serialize association");
        let roundtrip: VbaProjectAssociation =
            serde_json::from_str(&json).expect("deserialize association");

        assert_eq!(roundtrip.association_id, "vba-assoc-1");
        assert_eq!(roundtrip.root_object_name, "Application");
        assert!(roundtrip.enabled);
        assert_eq!(roundtrip.last_load_status, VbaProjectLoadStatus::Unloaded);
    }

    #[test]
    fn hosted_project_injects_application_version_module() {
        let module_texts = build_module_texts(
            &add_them_project(concat!(
                "Public Function HostVersion() As String\n",
                "HostVersion = Application.Version\n",
                "End Function\n"
            )),
            "0.1.0-test",
        );
        let (loaded, _project_name) =
            load_via_vba_host(module_texts).expect("hosted project should load");
        let mut prepared = loaded.prepare().expect("hosted project should prepare");

        let result = prepared
            .invoke_by_name_variant("Module1", "HostVersion", &[])
            .expect("HostVersion should run");

        assert_eq!(
            result.as_bstr(),
            Some(oxvba_runtime::bstr::BStr::from("0.1.0-test"))
        );
    }

    #[test]
    fn vba_host_admits_addthem_as_double_udf() {
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-1", "memory:AddThem"),
            add_them_project(""),
            "0.1.0-test",
            Some(
                "states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json".to_string(),
            ),
        )
        .expect("runtime should load");

        assert_eq!(
            runtime.association().last_load_status,
            VbaProjectLoadStatus::Loaded
        );
        assert_eq!(runtime.registrations().len(), 1);
        let registration = &runtime.registrations()[0];
        // W093 callable metadata preserves original procedure casing
        assert!(registration.formula_name.eq_ignore_ascii_case("AddThem"));
        assert_eq!(
            registration.registration.callable_metadata.parameter_count,
            2
        );
        assert_eq!(
            registration.excel_oracle_ref.as_deref(),
            Some("states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json")
        );
        assert!(runtime.current_snapshot().entries[0]
            .surface_name
            .eq_ignore_ascii_case("AddThem"));
        assert_eq!(
            runtime.current_snapshot().entries[0]
                .runtime_boundary_kind
                .as_deref(),
            Some("vba_host_callback")
        );
    }

    #[test]
    fn vba_host_provider_invokes_addthem_from_oxfml_formula() {
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-1", "memory:AddThem"),
            add_them_project(""),
            "0.1.0-test",
            Some(
                "states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json".to_string(),
            ),
        )
        .expect("runtime should load");
        let locale = oxfml_en_us_locale_context();
        let query_bundle = TypedContextQueryBundle::new(
            None,
            None,
            Some(&locale),
            Some(46000.0),
            Some(&FIXED_RANDOM_PROVIDER_05),
        )
        .with_host_function_provider(Some(&runtime));
        let mut host =
            oxfml_core::consumer::runtime::SingleFormulaHost::new("vba-udf-t001", "=AddThem(2,3)");

        let result = host
            .recalc_with_interfaces(
                EvaluationBackend::OxFuncBacked,
                query_bundle,
                Some(&runtime),
            )
            .expect("formula should evaluate through VBA host provider");

        assert_eq!(result.published_worksheet_value, CalcValue::number(5.0));
        assert!(result
            .typed_query_bundle_spec
            .families
            .contains(&oxfml_core::interface::TypedContextQueryFamily::HostFunction));
    }

    #[test]
    fn vba_host_end_to_end_vba_udf_in_compound_formula() {
        // End-to-end: VBA source → OxVba compile → UDF admission → OxFml registration
        // → formula evaluation with the VBA UDF embedded in a larger arithmetic expression.
        // Formula: =3*AddThem(4,5) → 3 * (4+5) = 27
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("e2e-compound-1", "memory:AddThem"),
            add_them_project(""),
            "0.1.0-test",
            None,
        )
        .expect("runtime should load");

        let locale = oxfml_en_us_locale_context();
        let query_bundle = TypedContextQueryBundle::new(
            None,
            None,
            Some(&locale),
            Some(46000.0),
            Some(&FIXED_RANDOM_PROVIDER_05),
        )
        .with_host_function_provider(Some(&runtime));

        let mut host = oxfml_core::consumer::runtime::SingleFormulaHost::new(
            "e2e-compound-vba-udf",
            "=3*AddThem(4,5)",
        );

        let result = host
            .recalc_with_interfaces(
                EvaluationBackend::OxFuncBacked,
                query_bundle,
                Some(&runtime),
            )
            .expect("=3*AddThem(4,5) should evaluate through VBA host provider");

        assert_eq!(
            result.published_worksheet_value,
            CalcValue::number(27.0),
            "3 * AddThem(4,5) = 3 * (4+5) = 27"
        );
    }

    #[test]
    fn vba_host_invokes_text_boolean_integer_and_variant_scalar_udfs() {
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-typed", "memory:TypedScalars"),
            VbaSourceProject {
                project_name: "DnaVbaFixture".to_string(),
                modules: vec![VbaSourceModule {
                    module_name: "Module1".to_string(),
                    source: concat!(
                        "Public Function EchoText(ByVal value As String) As String\n",
                        "EchoText = value & \"!\"\n",
                        "End Function\n",
                        "Public Function NotIt(ByVal value As Boolean) As Boolean\n",
                        "NotIt = Not value\n",
                        "End Function\n",
                        "Public Function AddLongs(ByVal a As Long, ByVal b As Long) As Long\n",
                        "AddLongs = a + b\n",
                        "End Function\n",
                        "Public Function PickVariant(ByVal value As Variant) As Variant\n",
                        "PickVariant = value\n",
                        "End Function\n",
                    )
                    .to_string(),
                }],
            },
            "0.1.0-test",
            None,
        )
        .expect("runtime should load");

        let names: Vec<_> = runtime
            .registrations()
            .iter()
            .map(|registration| registration.formula_name.to_ascii_lowercase())
            .collect();
        assert!(names.contains(&"echotext".to_string()));
        assert!(names.contains(&"notit".to_string()));
        assert!(names.contains(&"addlongs".to_string()));
        assert!(names.contains(&"pickvariant".to_string()));

        assert_eq!(
            runtime
                .invoke_registered_udf(
                    "EchoText",
                    &[CalcValue::text(ExcelText::from_interop_assignment("abc"))],
                )
                .expect("text UDF should run"),
            CalcValue::text(ExcelText::from_interop_assignment("abc!"))
        );
        assert_eq!(
            runtime
                .invoke_registered_udf("NotIt", &[CalcValue::logical(true)])
                .expect("boolean UDF should run"),
            CalcValue::logical(false)
        );
        assert_eq!(
            runtime
                .invoke_registered_udf(
                    "AddLongs",
                    &[CalcValue::number(2.0), CalcValue::number(3.0)],
                )
                .expect("Long UDF should run"),
            CalcValue::number(5.0)
        );
        assert_eq!(
            runtime
                .invoke_registered_udf(
                    "PickVariant",
                    &[CalcValue::text(ExcelText::from_interop_assignment(
                        "variant"
                    ))],
                )
                .expect("Variant-returning UDF should run"),
            CalcValue::text(ExcelText::from_interop_assignment("variant"))
        );

        let locale = oxfml_en_us_locale_context();
        let query_bundle = TypedContextQueryBundle::new(
            None,
            None,
            Some(&locale),
            Some(46000.0),
            Some(&FIXED_RANDOM_PROVIDER_05),
        )
        .with_host_function_provider(Some(&runtime));

        let mut text_host =
            oxfml_core::consumer::runtime::SingleFormulaHost::new("vba-text", "=EchoText(\"abc\")");
        let text_result = text_host
            .recalc_with_interfaces(
                EvaluationBackend::OxFuncBacked,
                query_bundle.clone(),
                Some(&runtime),
            )
            .expect("text UDF formula should evaluate");
        assert_eq!(
            text_result.published_worksheet_value,
            CalcValue::text(ExcelText::from_interop_assignment("abc!"))
        );

        let mut bool_host =
            oxfml_core::consumer::runtime::SingleFormulaHost::new("vba-bool", "=NotIt(TRUE)");
        let bool_result = bool_host
            .recalc_with_interfaces(
                EvaluationBackend::OxFuncBacked,
                query_bundle.clone(),
                Some(&runtime),
            )
            .expect("boolean UDF formula should evaluate");
        assert_eq!(
            bool_result.published_worksheet_value,
            CalcValue::logical(false)
        );

        let mut long_host =
            oxfml_core::consumer::runtime::SingleFormulaHost::new("vba-long", "=AddLongs(2,3)");
        let long_result = long_host
            .recalc_with_interfaces(
                EvaluationBackend::OxFuncBacked,
                query_bundle,
                Some(&runtime),
            )
            .expect("Long UDF formula should evaluate");
        assert_eq!(
            long_result.published_worksheet_value,
            CalcValue::number(5.0)
        );
    }

    #[test]
    fn vba_host_rejects_unmapped_calcvalue_payloads_explicitly() {
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-1", "memory:AddThem"),
            add_them_project(""),
            "0.1.0-test",
            None,
        )
        .expect("runtime should load");

        let err = runtime
            .invoke_registered_udf(
                "AddThem",
                &[CalcValue::array(
                    oxfunc_core::value::CalcArray::from_scalar(CalcValue::number(2.0))
                        .expect("1x1 array"),
                )],
            )
            .expect_err("array arguments should remain outside the scalar VBA UDF lane");

        assert!(err.contains("array payloads"));
    }

    #[test]
    fn vba_host_keeps_rejected_udf_candidate_visible() {
        // UdfAdmissionPolicy rejects Subs (not Functions) and class members.
        // Add a Sub alongside AddThem to verify rejected candidates are retained.
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-1", "memory:AddThem"),
            add_them_project(concat!("\nPublic Sub DoNothing()\n", "End Sub\n")),
            "0.1.0-test",
            None,
        )
        .expect("runtime should load with rejected candidate retained");

        assert_eq!(runtime.registrations().len(), 1);
        assert_eq!(runtime.current_snapshot().entries.len(), 1);
        let rejected = runtime
            .candidates()
            .iter()
            .find(|candidate| candidate.procedure_name.eq_ignore_ascii_case("DoNothing"))
            .expect("DoNothing should remain visible as a rejected candidate");

        match &rejected.admission_status {
            VbaUdfAdmissionStatus::Rejected { reason } => {
                assert!(!reason.is_empty(), "rejection reason must not be empty");
            }
            VbaUdfAdmissionStatus::Admitted => panic!("DoNothing must not be admitted"),
        }
    }
}
