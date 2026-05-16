use oxvba_compiler::{ModuleAttributes, ModuleKind, ModuleUnit, ProjectKind, ProjectManifest};
use oxvba_host::{
    Engine, HostConfig, HostUdfCallContext, HostUdfCatalog, HostUdfTypedInvokeResult,
    HostUdfTypedSignature, HostUdfTypedValue, PhaseDiagnostic, ProjectRuntimeSession,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OxvbaSourceModule {
    pub module_name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OxvbaSourceProject {
    pub project_name: String,
    pub modules: Vec<OxvbaSourceModule>,
}

pub struct OxvbaHostedProjectSession {
    pub engine: Engine,
    pub session: ProjectRuntimeSession,
}

pub fn compile_hosted_source_project(
    project: &OxvbaSourceProject,
    application_version: &str,
) -> Result<OxvbaHostedProjectSession, PhaseDiagnostic> {
    let mut engine = Engine::new(HostConfig {
        enable_jit: false,
        root_object_name: Some("Application".to_string()),
    });
    engine.register_root_object("Application", "DnaOneCalc.Application");
    let manifest = build_hosted_manifest(project, application_version);
    let session = engine.compile_and_prepare_session(&manifest)?;
    Ok(OxvbaHostedProjectSession { engine, session })
}

pub fn host_udf_catalog(session: &OxvbaHostedProjectSession) -> HostUdfCatalog {
    session.engine.host_udf_catalog(&session.session)
}

pub fn host_udf_typed_signature(
    session: &OxvbaHostedProjectSession,
    stable_host_call_id: &str,
    evidence: oxvba_host::HostUdfTypeMapEvidence,
) -> Result<HostUdfTypedSignature, PhaseDiagnostic> {
    session
        .engine
        .host_udf_typed_signature(&session.session, stable_host_call_id, evidence)
}

pub fn invoke_host_udf_typed(
    session: &mut OxvbaHostedProjectSession,
    signature: &HostUdfTypedSignature,
    context: HostUdfCallContext,
    args: &[HostUdfTypedValue],
) -> Result<HostUdfTypedInvokeResult, PhaseDiagnostic> {
    session
        .engine
        .invoke_host_udf_typed(&mut session.session, signature, context, args)
}

pub fn invoke_procedure_with_variants(
    session: &mut OxvbaHostedProjectSession,
    module_name: &str,
    procedure_name: &str,
) -> Result<oxvba_runtime::Variant, PhaseDiagnostic> {
    session.engine.invoke_procedure_with_variants(
        &mut session.session,
        module_name,
        procedure_name,
        &[],
    )
}

fn build_hosted_manifest(
    project: &OxvbaSourceProject,
    application_version: &str,
) -> ProjectManifest {
    let mut modules = Vec::with_capacity(project.modules.len() + 1);
    modules.push(host_application_module(application_version));
    modules.extend(project.modules.iter().map(source_module));

    ProjectManifest {
        project_name: project.project_name.clone(),
        project_kind: ProjectKind::Library,
        modules,
        references: Vec::new(),
        reference_projects: Vec::new(),
        conditional_constants: Default::default(),
    }
}

fn source_module(module: &OxvbaSourceModule) -> ModuleUnit {
    ModuleUnit {
        module_name: module.module_name.clone(),
        module_kind: ModuleKind::Procedural,
        attributes: ModuleAttributes {
            vb_name: module.module_name.clone(),
            ..Default::default()
        },
        source: module.source.clone(),
    }
}

fn host_application_module(application_version: &str) -> ModuleUnit {
    let escaped = application_version.replace('"', "\"\"");
    ModuleUnit {
        module_name: "Application".to_string(),
        module_kind: ModuleKind::Class,
        attributes: ModuleAttributes {
            vb_name: "Application".to_string(),
            vb_predeclared_id: true,
            vb_exposed: true,
            ..Default::default()
        },
        source: format!(
            "Public Property Get Version() As String\nVersion = \"{escaped}\"\nEnd Property\n"
        ),
    }
}
