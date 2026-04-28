//! WS-14 Pre-MVP home-shell view-model.
//!
//! Pure projection function that reads the active formula space out of
//! `OneCalcHostState` and produces a small `HomeShellViewModel` describing
//! what the home shell needs to render: textarea text + caret, a five-way
//! `ResultView`, and a `StatusView` for the status-foot dot + green-tree key.
//!
//! Source of result projections is `formula_space.latest_evaluation_summary`
//! and `formula_space.effective_display_summary`, populated by
//! `services::editor_session::derive_formula_presentation` from the bridge
//! `EditorDocument.value_presentation` (or local fallbacks). Array results
//! flow through `formula_space.array_preview`.
//!
//! Reference: `docs/WS14_PRE_MVP_PATH.md` §4 Step 2.

use crate::state::{FormulaSpaceState, OneCalcHostState, ProjectionTruthSource};
use crate::ui::editor::state::EditorSurfaceState;

/// Top-level home-shell projection.
///
/// Built freshly per render via `build_home_shell_view_model`. Returns
/// `None` when there is no active formula space (the home shell renders an
/// empty state in that case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeShellViewModel {
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub result_view: ResultView,
    pub status: StatusView,
}

/// What the result block should render. Mirrors the shape called out in the
/// path doc and matches the five mutually-exclusive UI states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultView {
    /// Editor is empty — show a muted placeholder.
    Empty,
    /// Editor has text but no eval result yet — show "..." muted.
    Pending,
    /// A scalar / text / logical result we can render.
    Display { text: String, kind: ResultKind },
    /// A diagnostic, blocked-lane, or error code state.
    Error { code: String, surface_repr: Option<String> },
    /// An array result; preview is deferred to a later WS-14 phase.
    Array { rows: usize, cols: usize, label: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Number,
    Text,
    Logical,
    RichValue,
    Other,
}

/// Status-foot projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub bridge_health: BridgeHealth,
    pub truth_source: ProjectionTruthSource,
    pub green_tree_key: Option<String>,
}

/// Coarse bridge health for the status-foot dot. `Live` is sage; `Stale`
/// is amber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeHealth {
    Live,
    Stale,
}

/// Resolve the active formula space and project its view-model. Returns
/// `None` when no formula space is active or the active id has no entry.
pub fn build_home_shell_view_model(state: &OneCalcHostState) -> Option<HomeShellViewModel> {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref()?;
    let formula_space = state.formula_spaces.get(active_id)?;
    Some(project_formula_space(formula_space))
}

fn project_formula_space(formula_space: &FormulaSpaceState) -> HomeShellViewModel {
    HomeShellViewModel {
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        editor_surface_state: formula_space.editor_surface_state.clone(),
        result_view: project_result_view(formula_space),
        status: project_status_view(formula_space),
    }
}

fn project_result_view(formula_space: &FormulaSpaceState) -> ResultView {
    // Empty editor wins regardless of stale residuals.
    if formula_space.raw_entered_cell_text.is_empty() {
        return ResultView::Empty;
    }

    // Array result projects to shape only; preview grid is a later phase.
    if let Some(preview) = formula_space.array_preview.as_ref() {
        let rows = preview.rows.len();
        let cols = preview.rows.first().map(|row| row.len()).unwrap_or(0);
        return ResultView::Array {
            rows,
            cols,
            label: preview.label.clone(),
        };
    }

    // Blocked or diagnostic surfaces project to Error.
    if let Some(reason) = formula_space.context.blocked_reason.as_deref() {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason.to_string()),
        };
    }

    if let Some(summary) = formula_space.latest_evaluation_summary.as_deref() {
        if let Some(detail) = summary.strip_prefix("Diagnostic · ") {
            return ResultView::Error {
                code: "DIAGNOSTIC".to_string(),
                surface_repr: Some(detail.to_string()),
            };
        }
        if let Some(detail) = summary.strip_prefix("Blocked · ") {
            return ResultView::Error {
                code: "BLOCKED".to_string(),
                surface_repr: Some(detail.to_string()),
            };
        }
        if let Some(detail) = summary.strip_prefix("Number · ") {
            // Prefer the effective display when present (preserves locale + format
            // application from the OxFml publication pipeline); fall back to the
            // bare number if no effective display has been computed yet.
            let text = formula_space
                .effective_display_summary
                .clone()
                .unwrap_or_else(|| detail.to_string());
            return ResultView::Display {
                text,
                kind: ResultKind::Number,
            };
        }
        if let Some(detail) = summary.strip_prefix("Text · ") {
            return ResultView::Display {
                text: detail.to_string(),
                kind: ResultKind::Text,
            };
        }
        if let Some(detail) = summary.strip_prefix("Logical · ") {
            return ResultView::Display {
                text: detail.to_string(),
                kind: ResultKind::Logical,
            };
        }
        // Unknown kind prefix; fall back to displaying the effective display
        // when present, otherwise the raw evaluation summary.
        let text = formula_space
            .effective_display_summary
            .clone()
            .unwrap_or_else(|| summary.to_string());
        return ResultView::Display {
            text,
            kind: ResultKind::Other,
        };
    }

    // Have text but no evaluation summary yet — bridge is in flight or
    // local fallback hasn't classified.
    ResultView::Pending
}

fn project_status_view(formula_space: &FormulaSpaceState) -> StatusView {
    let truth_source = formula_space.context.truth_source.clone();
    let green_tree_key = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.green_tree_key.clone())
        .filter(|key| !key.is_empty());

    let bridge_health = match (&truth_source, &green_tree_key) {
        (ProjectionTruthSource::LiveBacked, Some(_)) => BridgeHealth::Live,
        _ => BridgeHealth::Stale,
    };

    StatusView {
        bridge_health,
        truth_source,
        green_tree_key,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{FormulaArrayPreviewState, FormulaSpaceState};
    use crate::test_support::{
        array_editor_document, blocked_editor_document, diagnostic_editor_document,
        sample_editor_document,
    };

    /// Build a one-formula-space host state with the given (optional)
    /// editor_document attached. Helper for the test cases below.
    fn host_state_with(formula_space: FormulaSpaceState) -> OneCalcHostState {
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space.formula_space_id.clone());
        state.formula_spaces.insert(formula_space);
        state
    }

    #[test]
    fn returns_none_when_no_active_formula_space() {
        let state = OneCalcHostState::default();
        assert!(build_home_shell_view_model(&state).is_none());
    }

    #[test]
    fn empty_text_projects_to_result_view_empty() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.raw_entered_cell_text, "");
        assert_eq!(vm.result_view, ResultView::Empty);
    }

    #[test]
    fn happy_sum_projects_to_result_view_display_number() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.latest_evaluation_summary = Some("Number · 3".to_string());
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "3");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '3'), got {other:?}"),
        }
    }

    #[test]
    fn diagnostic_summary_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        formula_space.latest_evaluation_summary =
            Some("Diagnostic · Missing trailing argument".to_string());
        formula_space.effective_display_summary = Some("Input incomplete".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "DIAGNOSTIC");
                assert_eq!(surface_repr.as_deref(), Some("Missing trailing argument"));
            }
            other => panic!("expected Error(DIAGNOSTIC, ...), got {other:?}"),
        }
    }

    #[test]
    fn blocked_reason_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        formula_space.editor_document = Some(blocked_editor_document("=XLOOKUP(...)"));
        formula_space.context.blocked_reason = Some("XLOOKUP not admitted on this host".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("XLOOKUP not admitted on this host")
                );
            }
            other => panic!("expected Error(BLOCKED, ...), got {other:?}"),
        }
    }

    #[test]
    fn array_preview_projects_to_result_view_array_with_shape() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Array { rows, cols, label } => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 3);
                assert_eq!(label, "Array[2 × 3]");
            }
            other => panic!("expected Array(2 × 3), got {other:?}"),
        }
    }

    #[test]
    fn pending_text_with_no_summary_projects_to_pending() {
        let formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_view, ResultView::Pending);
    }

    #[test]
    fn status_live_when_live_backed_with_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Live);
        assert_eq!(vm.status.green_tree_key.as_deref(), Some("green-1"));
    }

    #[test]
    fn status_stale_when_local_fallback() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LocalFallback;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
    }

    #[test]
    fn status_stale_when_live_backed_but_no_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // LiveBacked but no editor_document => no green-tree key => stale.
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
        assert!(vm.status.green_tree_key.is_none());
    }
}
