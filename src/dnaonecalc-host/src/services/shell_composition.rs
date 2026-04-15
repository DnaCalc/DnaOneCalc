use serde_json::json;

use crate::services::explore_mode::{build_explore_view_model, ExploreViewModel};
use crate::services::inspect_mode::{build_inspect_view_model, InspectViewModel};
use crate::services::retained_artifacts::{
    active_retained_artifact, retained_artifacts_for_formula_space,
};
use crate::services::workbench_mode::{build_workbench_view_model, WorkbenchViewModel};
use crate::state::{AppMode, CapabilityDiffTarget, FormulaSpaceState, OneCalcHostState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModeProjection {
    Explore(ExploreViewModel),
    Inspect(InspectViewModel),
    Workbench(WorkbenchViewModel),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellModeTabViewModel {
    pub mode: AppMode,
    pub label: &'static str,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFormulaSpaceListItemViewModel {
    pub formula_space_id: String,
    pub label: String,
    pub truth_source_label: String,
    pub packet_kind_summary: String,
    pub mode_label: &'static str,
    pub is_active: bool,
    pub is_pinned: bool,
    pub is_dirty: bool,
    pub can_reopen: bool,
    pub section: ShellRailSection,
    pub retained_verdicts: Option<ShellRetainedVerdictsViewModel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellRailSection {
    Pinned,
    Open,
    Recent,
}

impl ShellRailSection {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Open => "open",
            Self::Recent => "recent",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pinned => "Pinned",
            Self::Open => "Open",
            Self::Recent => "Recent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellWorkspaceManifestViewModel {
    pub open_count: usize,
    pub pinned_count: usize,
    pub recent_count: usize,
    pub isolation_note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCenterFactViewModel {
    pub label: &'static str,
    pub value: String,
    pub tone: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDiffTargetOptionViewModel {
    pub slug: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDiffRowViewModel {
    pub label: &'static str,
    pub current_value: String,
    pub target_value: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCenterViewModel {
    pub title: String,
    pub subtitle: String,
    pub summary: Vec<CapabilityCenterFactViewModel>,
    pub snapshot_json: String,
    pub export_file_name: String,
    pub diff_target_options: Vec<CapabilityDiffTargetOptionViewModel>,
    pub selected_diff_target_slug: String,
    pub diff_rows: Vec<CapabilityDiffRowViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRetainedVerdictsViewModel {
    pub value_match: Option<bool>,
    pub display_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub comparison_lane_label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFrameViewModel {
    pub active_mode: AppMode,
    pub active_formula_space_label: String,
    pub active_mode_label: &'static str,
    pub active_truth_source_label: String,
    pub active_host_profile_summary: String,
    pub active_packet_kind_summary: String,
    pub active_capability_floor_summary: String,
    pub breadcrumb: ShellBreadcrumbViewModel,
    pub scope_strip: Vec<ShellScopeSegmentViewModel>,
    pub context_facts: Vec<ShellChromeFactViewModel>,
    pub footer_facts: Vec<ShellChromeFactViewModel>,
    pub workspace_summary: String,
    pub workspace_manifest: ShellWorkspaceManifestViewModel,
    pub capability_center: CapabilityCenterViewModel,
    pub mode_tabs: Vec<ShellModeTabViewModel>,
    pub formula_spaces: Vec<ShellFormulaSpaceListItemViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBreadcrumbViewModel {
    pub workspace_label: String,
    pub space_label: String,
    pub mode_label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellScopeSegmentViewModel {
    pub slug: &'static str,
    pub label: &'static str,
    pub value: String,
    pub status: ShellScopeSegmentStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellScopeSegmentStatus {
    Live,
    NotImplemented { seam_id: &'static str },
}

impl ShellScopeSegmentStatus {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::NotImplemented { .. } => "not-implemented",
        }
    }

    pub fn seam_id(&self) -> Option<&'static str> {
        match self {
            Self::Live => None,
            Self::NotImplemented { seam_id } => Some(*seam_id),
        }
    }
}

pub fn mode_accent_slug(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Explore => "explore",
        AppMode::Inspect => "inspect",
        AppMode::Workbench => "workbench",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellChromeFactViewModel {
    pub label: &'static str,
    pub value: String,
    pub tone: &'static str,
}

pub fn active_formula_space(state: &OneCalcHostState) -> Option<&FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .as_ref())?;
    state.formula_spaces.get(formula_space_id)
}

pub fn build_active_mode_projection(state: &OneCalcHostState) -> Option<ActiveModeProjection> {
    let formula_space = active_formula_space(state)?;
    match state.active_formula_space_view.active_mode {
        AppMode::Explore => Some(ActiveModeProjection::Explore(build_explore_view_model(
            formula_space,
            state.global_ui_chrome.editor_settings,
            state.global_ui_chrome.editor_settings_popover_open,
            state.global_ui_chrome.configure_drawer_open,
        ))),
        AppMode::Inspect => Some(ActiveModeProjection::Inspect(build_inspect_view_model(
            formula_space,
            active_retained_artifact(state)
                .filter(|artifact| artifact.formula_space_id == formula_space.formula_space_id),
        ))),
        AppMode::Workbench => Some(ActiveModeProjection::Workbench(build_workbench_view_model(
            formula_space,
            active_retained_artifact(state)
                .filter(|artifact| artifact.formula_space_id == formula_space.formula_space_id),
            &retained_artifacts_for_formula_space(state, &formula_space.formula_space_id),
        ))),
    }
}

pub fn build_shell_frame_view_model(state: &OneCalcHostState) -> Option<ShellFrameViewModel> {
    let active_formula_space = active_formula_space(state)?;
    let active_formula_space_id = &active_formula_space.formula_space_id;
    let active_mode = state.active_formula_space_view.active_mode;
    let active_mode_label = match active_mode {
        AppMode::Explore => "Explore",
        AppMode::Inspect => "Inspect",
        AppMode::Workbench => "Workbench",
    };

    let mode_tabs = [AppMode::Explore, AppMode::Inspect, AppMode::Workbench]
        .into_iter()
        .map(|mode| ShellModeTabViewModel {
            mode,
            label: match mode {
                AppMode::Explore => "Explore",
                AppMode::Inspect => "Inspect",
                AppMode::Workbench => "Workbench",
            },
            is_active: state.active_formula_space_view.active_mode == mode,
        })
        .collect();

    let formula_spaces: Vec<ShellFormulaSpaceListItemViewModel> = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .filter_map(|formula_space_id| {
            state
                .formula_spaces
                .get(formula_space_id)
                .map(|formula_space| {
                    let is_pinned = state
                        .workspace_shell
                        .pinned_formula_space_ids
                        .contains(&formula_space.formula_space_id);
                    let section = if is_pinned {
                        ShellRailSection::Pinned
                    } else {
                        ShellRailSection::Open
                    };
                    let live_state = formula_space.live_state();
                    let is_dirty = matches!(
                        live_state,
                        crate::ui::editor::state::EditorLiveState::EditingLive
                            | crate::ui::editor::state::EditorLiveState::ProofedScratch
                    );
                    let is_active = &formula_space.formula_space_id == active_formula_space_id;
                    let retained_verdicts = state
                        .retained_artifacts
                        .catalog
                        .values()
                        .find(|artifact| {
                            artifact.formula_space_id.as_str()
                                == formula_space.formula_space_id.as_str()
                        })
                        .map(|artifact| ShellRetainedVerdictsViewModel {
                            value_match: artifact.value_match,
                            display_match: artifact.display_match,
                            replay_equivalent: artifact.replay_equivalent,
                            comparison_lane_label: match artifact.comparison_status {
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Matched => "Matched",
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched => "Mismatched",
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked => "Blocked",
                            },
                        });
                    ShellFormulaSpaceListItemViewModel {
                        formula_space_id: formula_space.formula_space_id.as_str().to_string(),
                        label: formula_space.context.scenario_label.clone(),
                        truth_source_label: formula_space.context.truth_source.label().to_string(),
                        packet_kind_summary: formula_space.context.packet_kind.clone(),
                        mode_label: mode_label(if is_active {
                            active_mode
                        } else {
                            state
                                .workspace_shell
                                .formula_space_modes
                                .get(&formula_space.formula_space_id)
                                .copied()
                                .unwrap_or(AppMode::Explore)
                        }),
                        is_active,
                        is_pinned,
                        is_dirty,
                        can_reopen: false,
                        section,
                        retained_verdicts,
                    }
                })
        })
        .collect();

    let recent_formula_spaces = state
        .workspace_shell
        .recent_formula_space_order
        .iter()
        .filter_map(|formula_space_id| {
            state
                .workspace_shell
                .recent_formula_spaces
                .get(formula_space_id)
                .map(|record| {
                    let formula_space = &record.formula_space;
                    let live_state = formula_space.live_state();
                    let is_dirty = matches!(
                        live_state,
                        crate::ui::editor::state::EditorLiveState::EditingLive
                            | crate::ui::editor::state::EditorLiveState::ProofedScratch
                    );
                    let retained_verdicts = state
                        .retained_artifacts
                        .catalog
                        .values()
                        .find(|artifact| {
                            artifact.formula_space_id.as_str()
                                == formula_space.formula_space_id.as_str()
                        })
                        .map(|artifact| ShellRetainedVerdictsViewModel {
                            value_match: artifact.value_match,
                            display_match: artifact.display_match,
                            replay_equivalent: artifact.replay_equivalent,
                            comparison_lane_label: match artifact.comparison_status {
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Matched => "Matched",
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Mismatched => "Mismatched",
                                crate::services::programmatic_testing::ProgrammaticComparisonStatus::Blocked => "Blocked",
                            },
                        });
                    ShellFormulaSpaceListItemViewModel {
                        formula_space_id: formula_space.formula_space_id.as_str().to_string(),
                        label: formula_space.context.scenario_label.clone(),
                        truth_source_label: formula_space.context.truth_source.label().to_string(),
                        packet_kind_summary: formula_space.context.packet_kind.clone(),
                        mode_label: mode_label(record.last_active_mode),
                        is_active: false,
                        is_pinned: false,
                        is_dirty,
                        can_reopen: true,
                        section: ShellRailSection::Recent,
                        retained_verdicts,
                    }
                })
        })
        .collect::<Vec<_>>();

    let formula_spaces = formula_spaces
        .into_iter()
        .chain(recent_formula_spaces)
        .collect::<Vec<_>>();

    let workspace_summary = format!(
        "{} open · {} pinned · {} recent",
        state.workspace_shell.open_formula_space_order.len(),
        state.workspace_shell.pinned_formula_space_ids.len(),
        state.workspace_shell.recent_formula_space_order.len(),
    );
    let workspace_manifest = ShellWorkspaceManifestViewModel {
        open_count: state.workspace_shell.open_formula_space_order.len(),
        pinned_count: state.workspace_shell.pinned_formula_space_ids.len(),
        recent_count: state.workspace_shell.recent_formula_space_order.len(),
        isolation_note:
            "Documents remain isolated OneCalc instances. Reopen restores one space without cross-instance recalc or shared references.",
    };

    let context_facts = vec![
        ShellChromeFactViewModel {
            label: "Truth",
            value: active_formula_space
                .context
                .truth_source
                .label()
                .to_string(),
            tone: "accent",
        },
        ShellChromeFactViewModel {
            label: "Host",
            value: active_formula_space.context.host_profile.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Packet",
            value: active_formula_space.context.packet_kind.clone(),
            tone: "default",
        },
    ];

    let mut footer_facts = vec![
        ShellChromeFactViewModel {
            label: "Capability",
            value: active_formula_space.context.capability_floor.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Modes",
            value: active_formula_space.context.mode_availability.clone(),
            tone: "default",
        },
        ShellChromeFactViewModel {
            label: "Workspace",
            value: workspace_summary.clone(),
            tone: "muted",
        },
    ];
    if let Some(blocked_reason) = active_formula_space.context.blocked_reason.as_ref() {
        footer_facts.push(ShellChromeFactViewModel {
            label: "Blocked",
            value: blocked_reason.clone(),
            tone: "warning",
        });
    } else if let Some(trace_summary) = active_formula_space.context.trace_summary.as_ref() {
        footer_facts.push(ShellChromeFactViewModel {
            label: "Trace",
            value: trace_summary.clone(),
            tone: "muted",
        });
    }

    let breadcrumb = ShellBreadcrumbViewModel {
        workspace_label: "DNA OneCalc".to_string(),
        space_label: active_formula_space.context.scenario_label.clone(),
        mode_label: active_mode_label,
    };

    let scope_strip = vec![
        ShellScopeSegmentViewModel {
            slug: "locale",
            label: "Locale",
            value: "en-US".to_string(),
            status: ShellScopeSegmentStatus::NotImplemented {
                seam_id: "SEAM-OXFUNC-LOCALE-EXPAND",
            },
        },
        ShellScopeSegmentViewModel {
            slug: "date",
            label: "Date",
            value: "1900".to_string(),
            status: ShellScopeSegmentStatus::NotImplemented {
                seam_id: "SEAM-ONECALC-CAPABILITY-SNAPSHOT",
            },
        },
        ShellScopeSegmentViewModel {
            slug: "profile",
            label: "Profile",
            value: active_formula_space.context.host_profile.clone(),
            status: ShellScopeSegmentStatus::Live,
        },
        ShellScopeSegmentViewModel {
            slug: "policy",
            label: "Policy",
            value: "Deterministic".to_string(),
            status: ShellScopeSegmentStatus::NotImplemented {
                seam_id: "SEAM-ONECALC-CAPABILITY-SNAPSHOT",
            },
        },
        ShellScopeSegmentViewModel {
            slug: "format",
            label: "Format",
            value: "General".to_string(),
            status: ShellScopeSegmentStatus::NotImplemented {
                seam_id: "SEAM-ONECALC-EXTENDED-VALUE-ROUTING",
            },
        },
    ];
    let capability_center = build_capability_center_view_model(
        state,
        active_formula_space,
        active_mode,
        &workspace_manifest,
        &scope_strip,
    );

    Some(ShellFrameViewModel {
        active_mode,
        active_formula_space_label: active_formula_space.context.scenario_label.clone(),
        active_mode_label,
        breadcrumb,
        scope_strip,
        active_truth_source_label: active_formula_space
            .context
            .truth_source
            .label()
            .to_string(),
        active_host_profile_summary: active_formula_space.context.host_profile.clone(),
        active_packet_kind_summary: active_formula_space.context.packet_kind.clone(),
        active_capability_floor_summary: active_formula_space.context.capability_floor.clone(),
        context_facts,
        footer_facts,
        workspace_summary,
        workspace_manifest,
        capability_center,
        mode_tabs,
        formula_spaces,
    })
}

pub fn switch_active_mode(state: &mut OneCalcHostState, next_mode: AppMode) {
    state.active_formula_space_view.active_mode = next_mode;
    if let Some(active_formula_space_id) = state.workspace_shell.active_formula_space_id.as_ref() {
        state
            .workspace_shell
            .formula_space_modes
            .insert(active_formula_space_id.clone(), next_mode);
    }
}

pub fn select_active_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) {
    let Some(formula_space_id) = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .find(|id| id.as_str() == formula_space_id)
        .cloned()
    else {
        return;
    };

    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.active_mode = state
        .workspace_shell
        .formula_space_modes
        .get(&formula_space_id)
        .copied()
        .unwrap_or(AppMode::Explore);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(formula_space_id);
}

pub fn select_capability_diff_target(state: &mut OneCalcHostState, slug: &str) {
    let Some(parsed_target) = CapabilityDiffTarget::parse(slug) else {
        return;
    };
    let is_valid = match &parsed_target {
        CapabilityDiffTarget::WorkspaceBaseline => true,
        CapabilityDiffTarget::OpenFormulaSpace(formula_space_id) => state
            .workspace_shell
            .open_formula_space_order
            .iter()
            .any(|candidate| candidate == formula_space_id),
        CapabilityDiffTarget::RecentFormulaSpace(formula_space_id) => state
            .workspace_shell
            .recent_formula_space_order
            .iter()
            .any(|candidate| candidate == formula_space_id),
    };
    if is_valid {
        state.capability_and_environment.selected_diff_target = parsed_target;
    }
}

fn mode_label(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Explore => "Explore",
        AppMode::Inspect => "Inspect",
        AppMode::Workbench => "Workbench",
    }
}

fn build_capability_center_view_model(
    state: &OneCalcHostState,
    active_formula_space: &FormulaSpaceState,
    active_mode: AppMode,
    workspace_manifest: &ShellWorkspaceManifestViewModel,
    scope_strip: &[ShellScopeSegmentViewModel],
) -> CapabilityCenterViewModel {
    let current_snapshot = capability_snapshot_rows(
        active_formula_space.context.scenario_label.clone(),
        active_formula_space,
        active_mode,
        workspace_manifest,
        scope_strip,
    );
    let diff_target_options =
        capability_diff_target_options(state, &active_formula_space.formula_space_id);
    let selected_diff_target = normalize_capability_diff_target(
        state,
        &active_formula_space.formula_space_id,
        diff_target_options
            .first()
            .map(|option| option.slug.as_str()),
    );
    let selected_diff_target_slug = selected_diff_target.slug();
    let selected_diff_target_label = diff_target_options
        .iter()
        .find(|option| option.slug == selected_diff_target_slug)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| "Workspace baseline".to_string());
    let target_snapshot = capability_snapshot_for_diff_target(
        state,
        &selected_diff_target,
        workspace_manifest,
        scope_strip,
    );
    let diff_rows = current_snapshot
        .iter()
        .map(|current| {
            let target_value = target_snapshot
                .iter()
                .find(|target| target.label == current.label)
                .map(|target| target.value.clone())
                .unwrap_or_else(|| "Unavailable".to_string());
            CapabilityDiffRowViewModel {
                label: current.label,
                status: if current.value == target_value {
                    "same"
                } else {
                    "changed"
                },
                current_value: current.value.clone(),
                target_value,
            }
        })
        .collect::<Vec<_>>();
    let snapshot_json = serde_json::to_string_pretty(&json!({
        "scenario": active_formula_space.context.scenario_label,
        "mode": mode_label(active_mode),
        "truth_source": active_formula_space.context.truth_source.label(),
        "host_profile": active_formula_space.context.host_profile,
        "packet_kind": active_formula_space.context.packet_kind,
        "capability_floor": active_formula_space.context.capability_floor,
        "mode_availability": active_formula_space.context.mode_availability,
        "workspace": {
            "open": workspace_manifest.open_count,
            "pinned": workspace_manifest.pinned_count,
            "recent": workspace_manifest.recent_count,
        },
        "scope_strip": scope_strip.iter().map(|segment| {
            json!({
                "slug": segment.slug,
                "label": segment.label,
                "value": segment.value,
                "status": segment.status.slug(),
                "seam_id": segment.status.seam_id(),
            })
        }).collect::<Vec<_>>(),
    }))
    .unwrap_or_else(|error| format!("{{\"error\":\"{error}\"}}"));
    let summary = vec![
        CapabilityCenterFactViewModel {
            label: "Mode",
            value: mode_label(active_mode).to_string(),
            tone: "accent",
        },
        CapabilityCenterFactViewModel {
            label: "Floor",
            value: active_formula_space.context.capability_floor.clone(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Host",
            value: active_formula_space.context.host_profile.clone(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Diff target",
            value: selected_diff_target_label.clone(),
            tone: "muted",
        },
    ];

    CapabilityCenterViewModel {
        title: "Capability Center".to_string(),
        subtitle:
            "Supporting honesty surface for capability truth, diff targeting, copy, and export."
                .to_string(),
        summary,
        snapshot_json,
        export_file_name: format!(
            "onecalc-capability-{}.json",
            active_formula_space.formula_space_id.as_str()
        ),
        diff_target_options,
        selected_diff_target_slug,
        diff_rows,
    }
}

fn capability_diff_target_options(
    state: &OneCalcHostState,
    active_formula_space_id: &crate::domain::ids::FormulaSpaceId,
) -> Vec<CapabilityDiffTargetOptionViewModel> {
    let mut options = vec![CapabilityDiffTargetOptionViewModel {
        slug: CapabilityDiffTarget::WorkspaceBaseline.slug(),
        label: "Workspace baseline".to_string(),
    }];

    for formula_space_id in &state.workspace_shell.open_formula_space_order {
        if formula_space_id == active_formula_space_id {
            continue;
        }
        if let Some(formula_space) = state.formula_spaces.get(formula_space_id) {
            options.push(CapabilityDiffTargetOptionViewModel {
                slug: CapabilityDiffTarget::OpenFormulaSpace(formula_space_id.clone()).slug(),
                label: format!("Open · {}", formula_space.context.scenario_label),
            });
        }
    }

    for formula_space_id in &state.workspace_shell.recent_formula_space_order {
        if let Some(record) = state
            .workspace_shell
            .recent_formula_spaces
            .get(formula_space_id)
        {
            options.push(CapabilityDiffTargetOptionViewModel {
                slug: CapabilityDiffTarget::RecentFormulaSpace(formula_space_id.clone()).slug(),
                label: format!("Recent · {}", record.formula_space.context.scenario_label),
            });
        }
    }

    options
}

fn normalize_capability_diff_target(
    state: &OneCalcHostState,
    active_formula_space_id: &crate::domain::ids::FormulaSpaceId,
    fallback_slug: Option<&str>,
) -> CapabilityDiffTarget {
    let selected = state
        .capability_and_environment
        .selected_diff_target
        .clone();
    let valid = match &selected {
        CapabilityDiffTarget::WorkspaceBaseline => true,
        CapabilityDiffTarget::OpenFormulaSpace(formula_space_id) => {
            formula_space_id != active_formula_space_id
                && state
                    .workspace_shell
                    .open_formula_space_order
                    .iter()
                    .any(|candidate| candidate == formula_space_id)
        }
        CapabilityDiffTarget::RecentFormulaSpace(formula_space_id) => state
            .workspace_shell
            .recent_formula_space_order
            .iter()
            .any(|candidate| candidate == formula_space_id),
    };
    if valid {
        return selected;
    }
    fallback_slug
        .and_then(CapabilityDiffTarget::parse)
        .unwrap_or(CapabilityDiffTarget::WorkspaceBaseline)
}

fn capability_snapshot_for_diff_target(
    state: &OneCalcHostState,
    diff_target: &CapabilityDiffTarget,
    workspace_manifest: &ShellWorkspaceManifestViewModel,
    scope_strip: &[ShellScopeSegmentViewModel],
) -> Vec<CapabilityCenterFactViewModel> {
    match diff_target {
        CapabilityDiffTarget::WorkspaceBaseline => {
            capability_baseline_rows(workspace_manifest, scope_strip)
        }
        CapabilityDiffTarget::OpenFormulaSpace(formula_space_id) => state
            .formula_spaces
            .get(formula_space_id)
            .map(|formula_space| {
                capability_snapshot_rows(
                    formula_space.context.scenario_label.clone(),
                    formula_space,
                    state
                        .workspace_shell
                        .formula_space_modes
                        .get(formula_space_id)
                        .copied()
                        .unwrap_or(AppMode::Explore),
                    workspace_manifest,
                    scope_strip,
                )
            })
            .unwrap_or_else(|| capability_baseline_rows(workspace_manifest, scope_strip)),
        CapabilityDiffTarget::RecentFormulaSpace(formula_space_id) => state
            .workspace_shell
            .recent_formula_spaces
            .get(formula_space_id)
            .map(|record| {
                capability_snapshot_rows(
                    record.formula_space.context.scenario_label.clone(),
                    &record.formula_space,
                    record.last_active_mode,
                    workspace_manifest,
                    scope_strip,
                )
            })
            .unwrap_or_else(|| capability_baseline_rows(workspace_manifest, scope_strip)),
    }
}

fn capability_snapshot_rows(
    scenario_label: String,
    formula_space: &FormulaSpaceState,
    mode: AppMode,
    workspace_manifest: &ShellWorkspaceManifestViewModel,
    scope_strip: &[ShellScopeSegmentViewModel],
) -> Vec<CapabilityCenterFactViewModel> {
    let scope_summary = scope_strip
        .iter()
        .map(|segment| format!("{}={}", segment.label, segment.value))
        .collect::<Vec<_>>()
        .join(" · ");

    vec![
        CapabilityCenterFactViewModel {
            label: "Scenario",
            value: scenario_label,
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Mode",
            value: mode_label(mode).to_string(),
            tone: "accent",
        },
        CapabilityCenterFactViewModel {
            label: "Truth source",
            value: formula_space.context.truth_source.label().to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Host profile",
            value: formula_space.context.host_profile.clone(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Packet kind",
            value: formula_space.context.packet_kind.clone(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Capability floor",
            value: formula_space.context.capability_floor.clone(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Mode availability",
            value: formula_space.context.mode_availability.clone(),
            tone: "muted",
        },
        CapabilityCenterFactViewModel {
            label: "Workspace manifest",
            value: format!(
                "{} open · {} pinned · {} recent",
                workspace_manifest.open_count,
                workspace_manifest.pinned_count,
                workspace_manifest.recent_count
            ),
            tone: "muted",
        },
        CapabilityCenterFactViewModel {
            label: "Scope strip",
            value: scope_summary,
            tone: "muted",
        },
    ]
}

fn capability_baseline_rows(
    workspace_manifest: &ShellWorkspaceManifestViewModel,
    scope_strip: &[ShellScopeSegmentViewModel],
) -> Vec<CapabilityCenterFactViewModel> {
    let scope_summary = scope_strip
        .iter()
        .map(|segment| format!("{}={}", segment.label, segment.value))
        .collect::<Vec<_>>()
        .join(" · ");

    vec![
        CapabilityCenterFactViewModel {
            label: "Scenario",
            value: "Workspace baseline".to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Mode",
            value: "Shared".to_string(),
            tone: "accent",
        },
        CapabilityCenterFactViewModel {
            label: "Truth source",
            value: "workspace-shared".to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Host profile",
            value: "Workspace-scoped host truth".to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Packet kind",
            value: "Workspace ledger".to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Capability floor",
            value: "Current workspace floor".to_string(),
            tone: "default",
        },
        CapabilityCenterFactViewModel {
            label: "Mode availability",
            value: "Explore / Inspect / Workbench".to_string(),
            tone: "muted",
        },
        CapabilityCenterFactViewModel {
            label: "Workspace manifest",
            value: format!(
                "{} open · {} pinned · {} recent",
                workspace_manifest.open_count,
                workspace_manifest.pinned_count,
                workspace_manifest.recent_count
            ),
            tone: "muted",
        },
        CapabilityCenterFactViewModel {
            label: "Scope strip",
            value: scope_summary,
            tone: "muted",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::services::programmatic_testing::{
        ProgrammaticComparisonStatus, ProgrammaticOpenModeHint,
    };
    use crate::services::retained_artifacts::import_programmatic_artifact;
    use crate::services::retained_artifacts::RetainedArtifactImportRequest;
    use crate::state::{FormulaSpaceCollectionState, FormulaSpaceState, OneCalcHostState};
    use crate::test_support::sample_editor_document;

    #[test]
    fn active_formula_space_prefers_workspace_shell_selection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let active = active_formula_space(&state).expect("active space should exist");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
    }

    #[test]
    fn build_active_mode_projection_routes_to_explore_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Explore(view_model) => {
                assert_eq!(view_model.green_tree_key.as_deref(), Some("green-1"));
            }
            other => panic!("expected explore projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_to_inspect_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Inspect(view_model) => {
                assert_eq!(view_model.formula_walk_nodes.len(), 1);
                assert!(view_model.retained_artifact_context.is_none());
            }
            other => panic!("expected inspect projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_returns_none_without_active_space() {
        let state = OneCalcHostState {
            formula_spaces: FormulaSpaceCollectionState::default(),
            ..Default::default()
        };

        assert!(build_active_mode_projection(&state).is_none());
    }

    #[test]
    fn shell_frame_view_model_tracks_active_space_and_mode_tabs() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        state
            .workspace_shell
            .formula_space_modes
            .insert(formula_space_id.clone(), AppMode::Inspect);
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let frame = build_shell_frame_view_model(&state).expect("frame should exist");
        assert_eq!(frame.active_formula_space_label, "space-1");
        assert_eq!(frame.active_mode_label, "Inspect");
        assert_eq!(frame.active_truth_source_label, "local-fallback");
        assert_eq!(frame.workspace_summary, "1 open · 0 pinned · 0 recent");
        assert_eq!(frame.formula_spaces.len(), 1);
        assert!(frame.formula_spaces[0].is_active);
        assert!(!frame.formula_spaces[0].is_pinned);
        assert_eq!(frame.formula_spaces[0].mode_label, "Inspect");
        assert_eq!(frame.formula_spaces[0].truth_source_label, "local-fallback");
        assert_eq!(frame.workspace_manifest.open_count, 1);
        assert_eq!(frame.workspace_manifest.recent_count, 0);
        assert_eq!(frame.context_facts.len(), 3);
        assert!(frame
            .footer_facts
            .iter()
            .any(|fact| fact.label == "Capability" && fact.value == "pending"));
        assert!(frame
            .mode_tabs
            .iter()
            .any(|tab| tab.mode == AppMode::Inspect && tab.is_active));
    }

    #[test]
    fn switch_active_mode_updates_state() {
        let mut state = OneCalcHostState::default();
        let formula_space_id = FormulaSpaceId::new("space-1");
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state
            .workspace_shell
            .formula_space_modes
            .insert(formula_space_id, AppMode::Explore);
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Explore
        );

        switch_active_mode(&mut state, AppMode::Inspect);

        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Inspect
        );
        assert_eq!(
            state
                .workspace_shell
                .formula_space_modes
                .get(&FormulaSpaceId::new("space-1")),
            Some(&AppMode::Inspect)
        );
    }

    #[test]
    fn select_active_formula_space_updates_shell_selection() {
        let first_id = FormulaSpaceId::new("space-1");
        let second_id = FormulaSpaceId::new("space-2");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(first_id.clone());
        state.workspace_shell.open_formula_space_order = vec![first_id.clone(), second_id.clone()];
        state
            .workspace_shell
            .formula_space_modes
            .insert(first_id.clone(), AppMode::Explore);
        state
            .workspace_shell
            .formula_space_modes
            .insert(second_id.clone(), AppMode::Workbench);
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(first_id, "=SUM(1,2)"));
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(second_id.clone(), "=SEQUENCE(2,2)"));

        select_active_formula_space(&mut state, "space-2");

        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&second_id)
        );
        assert_eq!(
            state
                .active_formula_space_view
                .selected_formula_space_id
                .as_ref(),
            Some(&second_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Workbench
        );
    }

    #[test]
    fn shell_frame_view_model_lists_recent_reopenable_spaces() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state
            .workspace_shell
            .formula_space_modes
            .insert(formula_space_id.clone(), AppMode::Explore);
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let archived = FormulaSpaceState::new(FormulaSpaceId::new("space-2"), "=SEQUENCE(2,2)");
        state
            .workspace_shell
            .recent_formula_space_order
            .push(archived.formula_space_id.clone());
        state.workspace_shell.recent_formula_spaces.insert(
            archived.formula_space_id.clone(),
            crate::state::ClosedFormulaSpaceRecord {
                formula_space: archived,
                last_active_mode: AppMode::Inspect,
            },
        );

        let frame = build_shell_frame_view_model(&state).expect("frame should exist");
        let recent = frame
            .formula_spaces
            .iter()
            .find(|item| item.section == ShellRailSection::Recent)
            .expect("recent space should be listed");
        assert!(recent.can_reopen);
        assert_eq!(recent.mode_label, "Inspect");
        assert_eq!(frame.workspace_manifest.recent_count, 1);
    }

    #[test]
    fn shell_frame_view_model_exposes_capability_center_diff_targets() {
        let active_id = FormulaSpaceId::new("space-1");
        let open_id = FormulaSpaceId::new("space-2");
        let recent_id = FormulaSpaceId::new("space-3");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(active_id.clone());
        state.workspace_shell.open_formula_space_order = vec![active_id.clone(), open_id.clone()];
        state
            .workspace_shell
            .formula_space_modes
            .insert(active_id.clone(), AppMode::Explore);
        state
            .workspace_shell
            .formula_space_modes
            .insert(open_id.clone(), AppMode::Workbench);
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(active_id.clone(), "=SUM(1,2)"));
        let mut open_formula_space = FormulaSpaceState::new(open_id.clone(), "=SEQUENCE(2,2)");
        open_formula_space.context.scenario_label = "open diff target".to_string();
        state.formula_spaces.insert(open_formula_space);

        let mut recent_formula_space = FormulaSpaceState::new(recent_id.clone(), "=LET(x,1,x)");
        recent_formula_space.context.scenario_label = "recent diff target".to_string();
        state
            .workspace_shell
            .recent_formula_space_order
            .push(recent_id.clone());
        state.workspace_shell.recent_formula_spaces.insert(
            recent_id.clone(),
            crate::state::ClosedFormulaSpaceRecord {
                formula_space: recent_formula_space,
                last_active_mode: AppMode::Inspect,
            },
        );

        let frame = build_shell_frame_view_model(&state).expect("frame should exist");

        assert_eq!(
            frame.capability_center.selected_diff_target_slug,
            "workspace-baseline"
        );
        assert_eq!(frame.capability_center.diff_target_options.len(), 3);
        assert!(frame
            .capability_center
            .diff_target_options
            .iter()
            .any(|option| option.slug == "workspace-baseline"));
        assert!(frame
            .capability_center
            .diff_target_options
            .iter()
            .any(|option| option.label == "Open · open diff target"));
        assert!(frame
            .capability_center
            .diff_target_options
            .iter()
            .any(|option| option.label == "Recent · recent diff target"));
        assert!(frame
            .capability_center
            .diff_rows
            .iter()
            .any(|row| row.label == "Scenario" && row.status == "changed"));
        assert!(frame
            .capability_center
            .snapshot_json
            .contains("\"workspace\""));
    }

    #[test]
    fn select_capability_diff_target_ignores_active_open_space_and_invalid_slug() {
        let active_id = FormulaSpaceId::new("space-1");
        let other_id = FormulaSpaceId::new("space-2");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.open_formula_space_order = vec![active_id.clone(), other_id.clone()];
        state.capability_and_environment.selected_diff_target =
            CapabilityDiffTarget::WorkspaceBaseline;

        select_capability_diff_target(&mut state, "open:space-1");
        assert_eq!(
            state.capability_and_environment.selected_diff_target,
            CapabilityDiffTarget::OpenFormulaSpace(active_id.clone())
        );

        let frame = build_shell_frame_view_model(&OneCalcHostState {
            workspace_shell: crate::state::WorkspaceShellState {
                active_formula_space_id: Some(active_id.clone()),
                open_formula_space_order: vec![active_id.clone(), other_id.clone()],
                ..Default::default()
            },
            formula_spaces: {
                let mut collection = FormulaSpaceCollectionState::default();
                collection.insert(FormulaSpaceState::new(active_id.clone(), "=SUM(1,2)"));
                collection.insert(FormulaSpaceState::new(other_id.clone(), "=SUM(2,3)"));
                collection
            },
            capability_and_environment: state.capability_and_environment.clone(),
            ..Default::default()
        })
        .expect("frame should exist");
        assert_eq!(
            frame.capability_center.selected_diff_target_slug,
            "workspace-baseline"
        );

        select_capability_diff_target(&mut state, "bogus-target");
        assert_eq!(
            state.capability_and_environment.selected_diff_target,
            CapabilityDiffTarget::OpenFormulaSpace(active_id)
        );
    }

    #[test]
    fn build_active_mode_projection_routes_to_workbench_projection() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Number"));
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_open_retained_artifact_into_workbench() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );
        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Blocked"));
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_updates_open_catalog_item_when_active_artifact_changes() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Mismatched,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("dna=1 excel=2".to_string()),
            },
        );
        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-2".to_string(),
                        case_id: "case-2".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());
        let first_projection =
            build_active_mode_projection(&state).expect("first projection should be available");

        match first_projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(
                    view_model.retained_artifact_id.as_deref(),
                    Some("artifact-1")
                );
                let first_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-1")
                    .expect("artifact-1 catalog item");
                let second_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-2")
                    .expect("artifact-2 catalog item");
                assert!(first_item.is_open);
                assert!(!second_item.is_open);
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("dna=1 excel=2")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }

        state.retained_artifacts.open_artifact_id = Some("artifact-2".to_string());
        let second_projection =
            build_active_mode_projection(&state).expect("second projection should be available");

        match second_projection {
            ActiveModeProjection::Workbench(view_model) => {
                assert_eq!(
                    view_model.retained_artifact_id.as_deref(),
                    Some("artifact-2")
                );
                let first_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-1")
                    .expect("artifact-1 catalog item");
                let second_item = view_model
                    .retained_catalog_items
                    .iter()
                    .find(|item| item.artifact_id == "artifact-2")
                    .expect("artifact-2 catalog item");
                assert!(!first_item.is_open);
                assert!(second_item.is_open);
                assert_eq!(view_model.outcome_summary.as_deref(), Some("Blocked"));
                assert_eq!(
                    view_model.retained_discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected workbench projection, got {other:?}"),
        }
    }

    #[test]
    fn build_active_mode_projection_routes_open_retained_artifact_into_inspect_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        state.formula_spaces.insert(formula_space);

        import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id,
                catalog_entry:
                    crate::services::programmatic_testing::ProgrammaticArtifactCatalogEntry {
                        artifact_id: "artifact-1".to_string(),
                        case_id: "case-1".to_string(),
                        comparison_status: ProgrammaticComparisonStatus::Blocked,
                        open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                    },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );
        state.retained_artifacts.open_artifact_id = Some("artifact-1".to_string());

        let projection =
            build_active_mode_projection(&state).expect("projection should be available");
        match projection {
            ActiveModeProjection::Inspect(view_model) => {
                let context = view_model
                    .retained_artifact_context
                    .expect("retained context");
                assert_eq!(context.artifact_id, "artifact-1");
                assert_eq!(context.comparison_status, "blocked");
                assert_eq!(
                    context.discrepancy_summary.as_deref(),
                    Some("excel lane unavailable")
                );
            }
            other => panic!("expected inspect projection, got {other:?}"),
        }
    }
}
