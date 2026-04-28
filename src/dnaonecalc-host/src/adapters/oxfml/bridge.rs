use super::types::{EditorAnalysisStage, EditorDocument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditRequest {
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub previous_green_tree_key: Option<String>,
    pub analysis_stage: EditorAnalysisStage,
}

// `Eq` cannot be derived: `EditorDocument.value_presentation.published_value`
// is the upstream `EvalValue`, which contains `f64` (not `Eq`).
#[derive(Debug, Clone, PartialEq)]
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
