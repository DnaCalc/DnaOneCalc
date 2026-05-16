use std::cell::RefCell;
use std::fs;
use std::path::Path;

use oxfml_core::interface::{
    HostFunctionInvocation, HostFunctionProvider, HostFunctionProviderError,
    LibraryContextProvider, LibraryContextSnapshotRef,
};
use oxfml_core::semantics::{
    LibraryAvailabilityState, LibraryContextSnapshot, LibraryContextSnapshotEntry,
    RegistrationSourceKind,
};
use oxfunc_core::value::EvalValue;
use oxvba_host::{
    HostUdfCallContext, HostUdfTypeMapEvidence, HostUdfTypedSignature, HostUdfTypedValue,
};

use crate::adapters::oxvba::{
    compile_hosted_project_manifest, compile_hosted_source_project, host_udf_catalog,
    host_udf_typed_signature, invoke_host_udf_typed, OxvbaHostedProjectSession, OxvbaSourceModule,
    OxvbaSourceProject,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VbaUdfAdmissionStatus {
    Admitted,
    Rejected { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VbaUdfRegistration {
    pub association_id: String,
    pub formula_name: String,
    pub stable_host_call_id: String,
    pub signature: HostUdfTypedSignature,
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
    pub stable_host_call_id: String,
    pub admission_status: VbaUdfAdmissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VbaUdfExcelOracleExpectation {
    pub case_id: String,
    pub formula_call: String,
    pub oracle_ref: String,
    pub expected_number: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbaUdfAdmissionEvidence {
    ExcelObserved,
    HostProvisional,
}

impl VbaUdfAdmissionEvidence {
    fn oxvba_evidence(self) -> HostUdfTypeMapEvidence {
        match self {
            Self::ExcelObserved => HostUdfTypeMapEvidence::ExcelObserved,
            Self::HostProvisional => HostUdfTypeMapEvidence::HostProvisional,
        }
    }

    fn type_map_status(self) -> &'static str {
        match self {
            Self::ExcelObserved => "excel-observed-double-first-slice",
            Self::HostProvisional => "host-provisional-double-first-slice",
        }
    }

    fn admission_interface_kind(self) -> &'static str {
        match self {
            Self::ExcelObserved => "excel_observed_double_first_slice",
            Self::HostProvisional => "host_provisional_double_first_slice",
        }
    }
}

pub struct VbaHostRuntime {
    association: VbaProjectAssociation,
    session: RefCell<OxvbaHostedProjectSession>,
    candidates: Vec<VbaUdfCandidate>,
    registrations: Vec<VbaUdfRegistration>,
    snapshot: LibraryContextSnapshot,
}

impl VbaHostRuntime {
    pub fn load_source_project(
        association: VbaProjectAssociation,
        project: OxvbaSourceProject,
        application_version: &str,
        oracle_ref: Option<String>,
    ) -> Result<Self, String> {
        Self::load_source_project_with_evidence(
            association,
            project,
            application_version,
            oracle_ref,
            VbaUdfAdmissionEvidence::ExcelObserved,
        )
    }

    pub fn load_source_project_for_evaluation(
        association: VbaProjectAssociation,
        project: OxvbaSourceProject,
        application_version: &str,
    ) -> Result<Self, String> {
        Self::load_source_project_with_evidence(
            association,
            project,
            application_version,
            None,
            VbaUdfAdmissionEvidence::HostProvisional,
        )
    }

    pub fn load_project_path_for_evaluation(
        mut association: VbaProjectAssociation,
        project_path: impl AsRef<Path>,
        application_version: &str,
    ) -> Result<Self, String> {
        let project_path = project_path.as_ref();
        let project_ref = project_path.display().to_string();
        association.project_ref = project_ref;
        if project_path.extension().and_then(|ext| ext.to_str()) == Some("bas") {
            let source = fs::read_to_string(project_path).map_err(|error| {
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
                OxvbaSourceProject {
                    project_name,
                    modules: vec![OxvbaSourceModule {
                        module_name,
                        source,
                    }],
                },
                application_version,
            );
        }

        let loaded = oxvba_project::load_workspace_target(project_path).map_err(|error| {
            format!(
                "failed to load OxVba workspace target `{}`: {error}",
                project_path.display()
            )
        })?;
        let session = compile_hosted_project_manifest(&loaded.manifest, application_version)
            .map_err(|err| err.to_string())?;
        association.project_identity = Some(loaded.manifest.project_name.clone());
        Self::from_session(
            association,
            session,
            Some(loaded.manifest.project_name),
            None,
            VbaUdfAdmissionEvidence::HostProvisional,
        )
    }

    pub fn load_source_project_with_evidence(
        association: VbaProjectAssociation,
        project: OxvbaSourceProject,
        application_version: &str,
        oracle_ref: Option<String>,
        evidence: VbaUdfAdmissionEvidence,
    ) -> Result<Self, String> {
        let session = compile_hosted_source_project(&project, application_version)
            .map_err(|err| err.to_string())?;
        Self::from_session(
            association,
            session,
            Some(project.project_name),
            oracle_ref,
            evidence,
        )
    }

    fn from_session(
        mut association: VbaProjectAssociation,
        session: OxvbaHostedProjectSession,
        project_identity: Option<String>,
        oracle_ref: Option<String>,
        evidence: VbaUdfAdmissionEvidence,
    ) -> Result<Self, String> {
        let catalog = host_udf_catalog(&session);
        let mut candidates = Vec::new();
        let mut registrations = Vec::new();
        for function in catalog.functions {
            if function.module_name.eq_ignore_ascii_case("Application") {
                continue;
            }
            let signature = match host_udf_typed_signature(
                &session,
                &function.stable_host_call_id,
                evidence.oxvba_evidence(),
            ) {
                Ok(signature) => signature,
                Err(err) => {
                    candidates.push(VbaUdfCandidate {
                        association_id: association.association_id.clone(),
                        project_name: function.project_name,
                        module_name: function.module_name,
                        procedure_name: function.procedure_name,
                        stable_host_call_id: function.stable_host_call_id,
                        admission_status: VbaUdfAdmissionStatus::Rejected {
                            reason: err.to_string(),
                        },
                    });
                    continue;
                }
            };
            candidates.push(VbaUdfCandidate {
                association_id: association.association_id.clone(),
                project_name: function.project_name,
                module_name: function.module_name,
                procedure_name: function.procedure_name.clone(),
                stable_host_call_id: function.stable_host_call_id.clone(),
                admission_status: VbaUdfAdmissionStatus::Admitted,
            });
            registrations.push(VbaUdfRegistration {
                association_id: association.association_id.clone(),
                formula_name: function.procedure_name.clone(),
                stable_host_call_id: function.stable_host_call_id,
                signature,
                admission_status: VbaUdfAdmissionStatus::Admitted,
                type_map_status: evidence.type_map_status().to_string(),
                excel_oracle_ref: oracle_ref.clone(),
            });
        }

        association.project_identity = project_identity;
        association.last_load_status = VbaProjectLoadStatus::Loaded;
        let snapshot = library_snapshot_for_registrations(&association, &registrations, evidence);
        Ok(Self {
            association,
            session: RefCell::new(session),
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
        args: &[EvalValue],
    ) -> Result<EvalValue, String> {
        let registration = self
            .registrations
            .iter()
            .find(|registration| {
                registration
                    .formula_name
                    .eq_ignore_ascii_case(function_name)
            })
            .ok_or_else(|| format!("VBA UDF `{function_name}` is not admitted"))?;

        let typed_args = args
            .iter()
            .map(eval_value_to_typed_double)
            .collect::<Result<Vec<_>, _>>()?;
        let mut session = self
            .session
            .try_borrow_mut()
            .map_err(|_| "VBA runtime session is already borrowed".to_string())?;
        let result = invoke_host_udf_typed(
            &mut session,
            &registration.signature,
            HostUdfCallContext::new().with_caller("DnaOneCalc!A1"),
            &typed_args,
        )
        .map_err(|err| err.to_string())?;

        match result.value {
            HostUdfTypedValue::Double(value) => Ok(EvalValue::Number(value)),
        }
    }
}

impl HostFunctionProvider for VbaHostRuntime {
    fn invoke_host_function(
        &self,
        invocation: &HostFunctionInvocation,
    ) -> Result<EvalValue, HostFunctionProviderError> {
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

fn eval_value_to_typed_double(value: &EvalValue) -> Result<HostUdfTypedValue, String> {
    match value {
        EvalValue::Number(number) => Ok(HostUdfTypedValue::Double(*number)),
        other => Err(format!(
            "VBA-UDF-T001 admits only numeric arguments for Double parameters; got {other:?}"
        )),
    }
}

fn library_snapshot_for_registrations(
    association: &VbaProjectAssociation,
    registrations: &[VbaUdfRegistration],
    evidence: VbaUdfAdmissionEvidence,
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
                surface_stable_id: Some(registration.stable_host_call_id.clone()),
                name_resolution_table_ref: Some(format!("vba:{}", association.association_id)),
                semantic_trait_profile_ref: Some("vba-udf-double.v1".to_string()),
                gating_profile_ref: None,
                metadata_status: Some("host_registered".to_string()),
                special_interface_kind: None,
                admission_interface_kind: Some(evidence.admission_interface_kind().to_string()),
                preparation_owner: Some("DnaOneCalc".to_string()),
                runtime_boundary_kind: Some("vba_host_callback".to_string()),
                interface_contract_ref: Some("dnaonecalc-vba-udf-first-slice".to_string()),
                registration_source_kind: RegistrationSourceKind::Vba,
                parse_bind_state: LibraryAvailabilityState::CatalogKnown,
                semantic_plan_state: LibraryAvailabilityState::CatalogKnown,
                runtime_capability_state: Some(LibraryAvailabilityState::CatalogKnown),
                post_dispatch_state: None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxvba::{
        invoke_procedure_with_variants, OxvbaSourceModule, OxvbaSourceProject,
    };
    use oxfml_core::format::oxfml_en_us_locale_context;
    use oxfml_core::interface::TypedContextQueryBundle;
    use oxfml_core::EvaluationBackend;
    use oxvba_runtime::bstr::BStr;

    fn add_them_project(extra_source: &str) -> OxvbaSourceProject {
        OxvbaSourceProject {
            project_name: "DnaVbaFixture".to_string(),
            modules: vec![OxvbaSourceModule {
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
        let mut hosted = crate::adapters::oxvba::compile_hosted_source_project(
            &add_them_project(concat!(
                "Public Function HostVersion() As String\n",
                "HostVersion = Application.Version\n",
                "End Function\n"
            )),
            "0.1.0-test",
        )
        .expect("hosted project should compile");

        let result = invoke_procedure_with_variants(&mut hosted, "Module1", "HostVersion")
            .expect("HostVersion should run");

        assert_eq!(result.as_bstr(), Some(BStr::from("0.1.0-test")));
    }

    #[test]
    fn vba_host_admits_addthem_as_excel_observed_double_udf() {
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
        assert_eq!(registration.formula_name, "addthem");
        assert_eq!(registration.signature.parameters.len(), 2);
        assert_eq!(
            registration.type_map_status,
            "excel-observed-double-first-slice"
        );
        assert_eq!(
            registration.excel_oracle_ref.as_deref(),
            Some("states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json")
        );
        assert_eq!(
            runtime.current_snapshot().entries[0].surface_name,
            "addthem"
        );
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
        let query_bundle =
            TypedContextQueryBundle::new(None, None, Some(&locale), Some(46000.0), Some(0.5))
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

        assert_eq!(result.published_worksheet_value, EvalValue::Number(5.0));
        assert!(result
            .typed_query_bundle_spec
            .families
            .contains(&oxfml_core::interface::TypedContextQueryFamily::HostFunction));
    }

    #[test]
    fn vba_host_rejects_non_numeric_arguments_for_first_slice() {
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
                &[
                    EvalValue::Number(2.0),
                    EvalValue::Text(oxfunc_core::value::ExcelText::from_interop_assignment("3")),
                ],
            )
            .expect_err("text coercion is not admitted in first slice");

        assert!(err.contains("VBA-UDF-T001 admits only numeric arguments"));
    }

    #[test]
    fn vba_host_keeps_rejected_udf_candidate_visible() {
        let runtime = VbaHostRuntime::load_source_project(
            VbaProjectAssociation::workspace_source("vba-assoc-1", "memory:AddThem"),
            add_them_project(concat!(
                "\nPublic Function EchoText(value As String) As String\n",
                "EchoText = value\n",
                "End Function\n"
            )),
            "0.1.0-test",
            None,
        )
        .expect("runtime should load with rejected candidate retained");

        assert_eq!(runtime.registrations().len(), 1);
        assert_eq!(runtime.current_snapshot().entries.len(), 1);
        let rejected = runtime
            .candidates()
            .iter()
            .find(|candidate| candidate.procedure_name.eq_ignore_ascii_case("EchoText"))
            .expect("EchoText should remain visible as a rejected candidate");

        match &rejected.admission_status {
            VbaUdfAdmissionStatus::Rejected { reason } => {
                assert!(reason.contains("not admitted by the typed first slice"));
            }
            VbaUdfAdmissionStatus::Admitted => panic!("EchoText must not be admitted"),
        }
    }
}
