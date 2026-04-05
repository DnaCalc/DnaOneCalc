use std::collections::{BTreeMap, BTreeSet};

use crate::adapters::oxfml::EditorDocument;
use crate::domain::ids::FormulaSpaceId;

#[derive(Debug, Clone)]
pub struct OneCalcHostState {
    pub workspace_shell: WorkspaceShellState,
    pub formula_spaces: FormulaSpaceCollectionState,
    pub active_formula_space_view: ActiveFormulaSpaceViewState,
    pub retained_artifacts: RetainedArtifactOpenState,
    pub capability_and_environment: CapabilityAndEnvironmentState,
    pub extension_surface: ExtensionSurfaceState,
    pub global_ui_chrome: GlobalUiChromeState,
}

impl Default for OneCalcHostState {
    fn default() -> Self {
        Self {
            workspace_shell: WorkspaceShellState::default(),
            formula_spaces: FormulaSpaceCollectionState::default(),
            active_formula_space_view: ActiveFormulaSpaceViewState::default(),
            retained_artifacts: RetainedArtifactOpenState::default(),
            capability_and_environment: CapabilityAndEnvironmentState::default(),
            extension_surface: ExtensionSurfaceState::default(),
            global_ui_chrome: GlobalUiChromeState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceShellState {
    pub active_formula_space_id: Option<FormulaSpaceId>,
    pub open_formula_space_order: Vec<FormulaSpaceId>,
    pub pinned_formula_space_ids: BTreeSet<FormulaSpaceId>,
    pub navigation_selection: WorkspaceNavigationSelection,
}

impl Default for WorkspaceShellState {
    fn default() -> Self {
        Self {
            active_formula_space_id: None,
            open_formula_space_order: Vec::new(),
            pinned_formula_space_ids: BTreeSet::new(),
            navigation_selection: WorkspaceNavigationSelection::Overview,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaSpaceCollectionState {
    pub spaces: BTreeMap<FormulaSpaceId, FormulaSpaceState>,
}

impl FormulaSpaceCollectionState {
    pub fn insert(&mut self, formula_space: FormulaSpaceState) {
        self.spaces
            .insert(formula_space.formula_space_id.clone(), formula_space);
    }

    pub fn get(&self, formula_space_id: &FormulaSpaceId) -> Option<&FormulaSpaceState> {
        self.spaces.get(formula_space_id)
    }

    pub fn get_mut(
        &mut self,
        formula_space_id: &FormulaSpaceId,
    ) -> Option<&mut FormulaSpaceState> {
        self.spaces.get_mut(formula_space_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSpaceState {
    pub formula_space_id: FormulaSpaceId,
    pub raw_entered_cell_text: String,
    pub editor_document: Option<EditorDocument>,
    pub completion_help: CompletionHelpState,
    pub latest_evaluation_summary: Option<String>,
    pub effective_display_summary: Option<String>,
}

impl FormulaSpaceState {
    pub fn new(formula_space_id: FormulaSpaceId, raw_entered_cell_text: impl Into<String>) -> Self {
        Self {
            formula_space_id,
            raw_entered_cell_text: raw_entered_cell_text.into(),
            editor_document: None,
            completion_help: CompletionHelpState::default(),
            latest_evaluation_summary: None,
            effective_display_summary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionHelpState {
    pub completion_count: usize,
    pub has_signature_help: bool,
    pub function_help_lookup_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveFormulaSpaceViewState {
    pub active_mode: AppMode,
    pub selected_formula_space_id: Option<FormulaSpaceId>,
}

impl Default for ActiveFormulaSpaceViewState {
    fn default() -> Self {
        Self {
            active_mode: AppMode::Explore,
            selected_formula_space_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetainedArtifactOpenState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilityAndEnvironmentState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtensionSurfaceState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalUiChromeState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceNavigationSelection {
    Overview,
    Recent,
    Pinned,
    FormulaSpace(FormulaSpaceId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Explore,
    Inspect,
    Workbench,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFormulaSpaceRecord {
    pub formula_space_id: FormulaSpaceId,
    pub is_pinned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_space_tracks_raw_entered_cell_text() {
        let state = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "'123.4");
        assert_eq!(state.raw_entered_cell_text, "'123.4");
    }

    #[test]
    fn workspace_shell_defaults_to_explore_without_active_space() {
        let state = OneCalcHostState::default();
        assert_eq!(state.active_formula_space_view.active_mode, AppMode::Explore);
        assert!(state.workspace_shell.active_formula_space_id.is_none());
    }
}
