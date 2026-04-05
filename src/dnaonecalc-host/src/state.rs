#[derive(Debug, Clone, Default)]
pub struct OneCalcHostState {
    pub workspace_shell: WorkspaceShellState,
    pub formula_spaces: FormulaSpaceCollectionState,
    pub active_formula_space_view: ActiveFormulaSpaceViewState,
    pub retained_artifacts: RetainedArtifactOpenState,
    pub capability_and_environment: CapabilityAndEnvironmentState,
    pub extension_surface: ExtensionSurfaceState,
    pub global_ui_chrome: GlobalUiChromeState,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceShellState;

#[derive(Debug, Clone, Default)]
pub struct FormulaSpaceCollectionState;

#[derive(Debug, Clone, Default)]
pub struct ActiveFormulaSpaceViewState;

#[derive(Debug, Clone, Default)]
pub struct RetainedArtifactOpenState;

#[derive(Debug, Clone, Default)]
pub struct CapabilityAndEnvironmentState;

#[derive(Debug, Clone, Default)]
pub struct ExtensionSurfaceState;

#[derive(Debug, Clone, Default)]
pub struct GlobalUiChromeState;
