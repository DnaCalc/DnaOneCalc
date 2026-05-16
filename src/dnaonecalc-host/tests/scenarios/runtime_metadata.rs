//! Runtime metadata propagation scenarios.

use dnaonecalc_host::adapters::oxfml::{
    EditorAnalysisStage, EvalValue, FormulaEditRequest, FormulaInputBindingRequest,
    LiveOxfmlBridge, OxfmlEditorBridge, RecalcModeRequest, ScenarioPolicyRequest, TraceModeRequest,
};

#[test]
fn sum_value_presentation_carries_oxfunc_kernel_and_admission_versions() {
    let bridge = LiveOxfmlBridge::default();
    let result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: "metadata-sum".to_string(),
            entered_text: "=SUM(1,2,3)".to_string(),
            cursor_offset: "=SUM(1,2,3)".len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            trace_mode: TraceModeRequest::PreparedCalls,
            language_tag: "en-US".to_string(),
            formal_input_bindings: Vec::new(),
        })
        .expect("live bridge should evaluate SUM");

    let presentation = result
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation");

    assert!(
        presentation
            .semantic_kernel_metadata_version
            .as_deref()
            .is_some_and(|version| version.contains("SequentialLeftFold")),
        "SUM should carry OxFunc semantic kernel metadata version; got {:?}",
        presentation.semantic_kernel_metadata_version,
    );
    assert!(
        presentation
            .arg_admission_metadata_version
            .as_deref()
            .is_some_and(|version| version.contains("values_only_pre_adapter")),
        "SUM should carry OxFunc arg admission metadata version; got {:?}",
        presentation.arg_admission_metadata_version,
    );
    assert!(presentation.producer_capability_set_keys.is_empty());
    assert!(presentation.exercised_capability_keys.is_empty());
}

#[test]
fn formal_input_binding_affects_single_formula_evaluation() {
    let bridge = LiveOxfmlBridge::default();
    let result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: "formal-input-rate".to_string(),
            entered_text: "=Rate*10".to_string(),
            cursor_offset: "=Rate*10".len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            trace_mode: TraceModeRequest::PreparedCalls,
            language_tag: "en-US".to_string(),
            formal_input_bindings: vec![FormulaInputBindingRequest {
                label: "Rate".to_string(),
                reference_descriptor: "name:Rate".to_string(),
                reference_handle: None,
                value: EvalValue::Number(0.2),
            }],
        })
        .expect("live bridge should evaluate with formal input");

    let presentation = result
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation");

    assert_eq!(presentation.effective_display_summary.as_deref(), Some("2"));
}
