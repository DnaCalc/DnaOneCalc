use super::bridge::{
    FormulaEditRequest, FormulaEditResult, OxfmlEditorBridge, OxfmlEditorBridgeError,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct LiveOxfmlBridge;

impl OxfmlEditorBridge for LiveOxfmlBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError> {
        let _stage = map_analysis_stage(request.analysis_stage);
        Err(OxfmlEditorBridgeError::UpstreamFailure(
            "Live OxFml bridge seam is enabled but packet mapping is not implemented yet".to_string(),
        ))
    }
}

fn map_analysis_stage(stage: super::bridge::EditorAnalysisStage) -> oxfml_core::consumer::editor::EditorAnalysisStage {
    match stage {
        super::bridge::EditorAnalysisStage::SyntaxOnly => {
            oxfml_core::consumer::editor::EditorAnalysisStage::SyntaxOnly
        }
        super::bridge::EditorAnalysisStage::SyntaxAndBind => {
            oxfml_core::consumer::editor::EditorAnalysisStage::SyntaxAndBind
        }
        super::bridge::EditorAnalysisStage::FullSemanticPlan => {
            oxfml_core::consumer::editor::EditorAnalysisStage::FullSemanticPlan
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_bridge_maps_local_analysis_stage_to_upstream_stage() {
        let stage = map_analysis_stage(super::super::bridge::EditorAnalysisStage::SyntaxAndBind);
        assert_eq!(stage, oxfml_core::consumer::editor::EditorAnalysisStage::SyntaxAndBind);
    }
}
