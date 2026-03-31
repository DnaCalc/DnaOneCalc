pub mod function_surface;
pub mod runtime;
pub mod shell;

use oxfml_core::{parse_formula, FormulaChannelKind, FormulaSourceRecord, ParseRequest};
use oxfunc_core::functions::sum::eval_sum_surface;
use oxfunc_core::resolver::{RefResolutionError, ReferenceResolver, ResolverCapabilities};
use oxfunc_core::value::{CallArgValue, EvalValue, ReferenceLike};
use oxreplay_abstractions::{LaneId, RegistryRef};
use oxreplay_core::{is_replay_ready, ReplayEvent, ReplayScenario};

pub use function_surface::{
    AdmissionCategory, FunctionSurfaceCatalog, FunctionSurfaceEntry, SurfaceLabelSummary,
};
pub use runtime::{
    FormulaEditPacketSummary, FormulaEditorSession, FormulaEvaluationSummary, HostPacketKind,
    OneCalcHostProfile, ParseSnapshot, PlatformGate, RuntimeAdapter,
};
pub use shell::{launch_shell, launch_shell_with_formula, OneCalcShellApp};

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyProbeReport {
    pub formula_token: String,
    pub parse_token_count: usize,
    pub parse_diagnostic_count: usize,
    pub sum_result: f64,
    pub replay_ready: bool,
    pub replay_registry_ref_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyProbeError {
    SumDidNotReturnNumber,
}

struct NoReferenceResolver;

impl ReferenceResolver for NoReferenceResolver {
    fn capabilities(&self) -> ResolverCapabilities {
        ResolverCapabilities::permissive_local()
    }

    fn resolve_reference(
        &self,
        reference: &ReferenceLike,
    ) -> Result<EvalValue, RefResolutionError> {
        Err(RefResolutionError::UnresolvedReference {
            target: reference.target.clone(),
        })
    }
}

pub fn run_dependency_probe() -> Result<DependencyProbeReport, DependencyProbeError> {
    let source = FormulaSourceRecord::new("onecalc.probe", 1, "=SUM(1,2,3)")
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let formula_token = source.formula_token().0;

    let parse = parse_formula(ParseRequest { source });
    let parse_token_count = parse.green_tree.full_fidelity_tokens.len();
    let parse_diagnostic_count = parse.green_tree.diagnostics.len();

    let args = [
        CallArgValue::Eval(EvalValue::Number(1.0)),
        CallArgValue::Eval(EvalValue::Number(2.0)),
        CallArgValue::Eval(EvalValue::Number(3.0)),
    ];
    let sum_result = match eval_sum_surface(&args, &NoReferenceResolver) {
        Ok(EvalValue::Number(number)) => number,
        Ok(_) | Err(_) => return Err(DependencyProbeError::SumDidNotReturnNumber),
    };

    let replay = ReplayScenario {
        scenario_id: "onecalc.probe.sum".to_string(),
        lane_id: LaneId("onecalc".to_string()),
        events: vec![ReplayEvent {
            event_id: "event-001".to_string(),
            source_label: "sum_probe".to_string(),
            normalized_family: "evaluation.sum".to_string(),
        }],
        registry_refs: vec![RegistryRef {
            family: "probe".to_string(),
            version: "v1".to_string(),
        }],
    };

    Ok(DependencyProbeReport {
        formula_token,
        parse_token_count,
        parse_diagnostic_count,
        sum_result,
        replay_ready: is_replay_ready(&replay),
        replay_registry_ref_count: replay.registry_refs.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_probe_uses_real_upstream_libraries() {
        let report = run_dependency_probe().expect("dependency probe should succeed");

        assert!(!report.formula_token.is_empty());
        assert!(report.parse_token_count > 0);
        assert_eq!(report.parse_diagnostic_count, 0);
        assert_eq!(report.sum_result, 6.0);
        assert!(report.replay_ready);
        assert_eq!(report.replay_registry_ref_count, 1);
    }

    #[test]
    fn runtime_adapter_exposes_profile_and_packet_register() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);

        assert_eq!(adapter.host_profile(), OneCalcHostProfile::OcH0);
        assert_eq!(adapter.host_profile().id(), "OC-H0");
        assert_eq!(
            adapter.packet_kinds(),
            &[
                HostPacketKind::FormulaEdit,
                HostPacketKind::EditAcceptRecalc,
                HostPacketKind::ReplayCapture,
            ]
        );
    }

    #[test]
    fn runtime_adapter_evaluates_admitted_formula_through_upstream_host() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
        let summary = adapter
            .evaluate_formula("=SUM(1,2,3)")
            .expect("admitted SUM formula should evaluate");

        assert!(!summary.formula_token.is_empty());
        assert_eq!(summary.worksheet_value_summary, "Number(6)");
        assert_eq!(summary.commit_decision_kind, "accepted");
    }
}
