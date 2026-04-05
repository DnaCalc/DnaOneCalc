use super::types::EditorDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAnalysisStage {
    SyntaxOnly,
    SyntaxAndBind,
    FullSemanticPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditRequest {
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub previous_green_tree_key: Option<String>,
    pub analysis_stage: EditorAnalysisStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditResult {
    pub document: EditorDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OxfmlEditorBridgeError {
    UpstreamFailure(String),
}

pub trait OxfmlEditorBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlEditorBridgeError>;
}
