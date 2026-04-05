use crate::adapters::oxfml::EditorAnalysisStage;
use crate::domain::ids::FormulaSpaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIntent {
    ApplyFormulaEdit(ApplyFormulaEditIntent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFormulaEditIntent {
    pub formula_space_id: FormulaSpaceId,
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub analysis_stage: EditorAnalysisStage,
}
