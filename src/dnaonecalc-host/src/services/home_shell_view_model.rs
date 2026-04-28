//! WS-14 home-shell view-model.
//!
//! Pure projection function that reads the active formula space out of
//! `OneCalcHostState` and produces a small `HomeShellViewModel` describing
//! what the home shell renders: textarea text + caret, a five-way
//! `ResultView`, and a `StatusView` for the status-foot dot + green-tree key.
//!
//! The result projection dispatches **typed**: it reads the bridge's typed
//! `published_value: EvalValue` from `editor_document.value_presentation`,
//! the live diagnostics list from `editor_document.live_diagnostics`, the
//! provenance blocked reason from `editor_document.provenance_summary`, and
//! the host-derived `context.blocked_reason` — never re-parsing the
//! `latest_evaluation_summary` string. Pre-bridge text/number cells (input
//! that does not start with `=`) are hand-evaluated inline against the raw
//! source text. Array results flow through `formula_space.array_preview`.
//!
//! Reference: `docs/WS14_PRE_MVP_PATH.md` §4 Step 2.

use crate::adapters::oxfml::{worksheet_error_literal, EvalValue};
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
    Error {
        code: String,
        surface_repr: Option<String>,
    },
    /// An array result; preview is deferred to a later WS-14 phase.
    Array {
        rows: usize,
        cols: usize,
        label: String,
    },
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
    let raw_text = formula_space.raw_entered_cell_text.as_str();

    // Empty editor wins regardless of stale residuals.
    if raw_text.is_empty() {
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

    // Host-derived blocked reason wins regardless of value type.
    if let Some(reason) = formula_space.context.blocked_reason.as_deref() {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason.to_string()),
        };
    }

    // Bridge-side blocked-reason on the editor document (provenance summary
    // populated by the live bridge for capability-denied lanes).
    if let Some(reason) = formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.provenance_summary.as_ref())
        .and_then(|prov| prov.blocked_reason.clone())
    {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason),
        };
    }

    // Live diagnostic on the editor document. The bridge does not produce a
    // typed value when it stops at a parse / bind diagnostic, so the
    // diagnostic itself is the visible result. We surface the first one;
    // the drill-down is responsible for the full list.
    if !has_published_value(formula_space) {
        if let Some(message) = formula_space
            .editor_document
            .as_ref()
            .and_then(|doc| doc.live_diagnostics.diagnostics.first())
            .map(|diag| diag.message.clone())
        {
            return ResultView::Error {
                code: "DIAGNOSTIC".to_string(),
                surface_repr: Some(message),
            };
        }
    }

    // Typed dispatch on the bridge's published `EvalValue`.
    if let Some(published_value) = bridge_published_value(formula_space) {
        return project_typed_value(formula_space, published_value);
    }

    // Pre-bridge hand-evaluation for raw text / number cells: anything that
    // doesn't start with `=` is a literal cell entry. The live bridge
    // doesn't run for these; the home shell evaluates them inline against
    // the raw text. Forced-text cells (`'1.5`) keep the leading apostrophe
    // out of the rendered display.
    if let Some(forced_text) = raw_text.strip_prefix('\'') {
        return ResultView::Display {
            text: forced_text.to_string(),
            kind: ResultKind::Text,
        };
    }
    if !raw_text.starts_with('=') {
        if let Ok(number) = raw_text.parse::<f64>() {
            return ResultView::Display {
                text: format_literal_number(number),
                kind: ResultKind::Number,
            };
        }
        return ResultView::Display {
            text: raw_text.to_string(),
            kind: ResultKind::Text,
        };
    }

    ResultView::Pending
}

fn has_published_value(formula_space: &FormulaSpaceState) -> bool {
    bridge_published_value(formula_space).is_some()
}

fn bridge_published_value(formula_space: &FormulaSpaceState) -> Option<&EvalValue> {
    formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.value_presentation.as_ref())
        .map(|vp| &vp.published_value)
}

fn project_typed_value(formula_space: &FormulaSpaceState, value: &EvalValue) -> ResultView {
    let display_text = || {
        formula_space
            .effective_display_summary
            .clone()
            .unwrap_or_default()
    };
    match value {
        EvalValue::Number(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Number,
        },
        EvalValue::Text(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Text,
        },
        EvalValue::Logical(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Logical,
        },
        EvalValue::Error(code) => ResultView::Error {
            code: worksheet_error_literal(*code).to_string(),
            surface_repr: None,
        },
        EvalValue::Array(_) => {
            // Array path normally goes through `formula_space.array_preview`
            // (handled above). If we reach here without a preview, surface
            // the effective display string.
            ResultView::Display {
                text: display_text(),
                kind: ResultKind::Other,
            }
        }
        EvalValue::Reference(_) | EvalValue::Lambda(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Other,
        },
    }
}

fn format_literal_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
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
    use crate::adapters::oxfml::{FormulaValuePresentation, ProvenanceSummary};
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

    /// Attach a typed Number `value_presentation` to the document so the
    /// home view-model dispatches via the typed path.
    fn attach_number_value_presentation(
        document: &mut crate::adapters::oxfml::EditorDocument,
        number: f64,
        display: &str,
    ) {
        document.value_presentation = Some(FormulaValuePresentation {
            evaluation_summary: format!("Number · {display}"),
            effective_display_summary: Some(display.to_string()),
            array_preview: None,
            blocked_reason: None,
            published_value: EvalValue::Number(number),
        });
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
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
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
    fn diagnostic_in_editor_document_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
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
    fn host_blocked_reason_projects_to_result_view_error() {
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
    fn bridge_blocked_provenance_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        let mut document = blocked_editor_document("=XLOOKUP(...)");
        // `blocked_editor_document` already sets a provenance blocked reason.
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OxFml blocked lane".to_string(),
            blocked_reason: Some("Excel comparison lane unavailable".to_string()),
        });
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("Excel comparison lane unavailable")
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
        // `=SU` starts with `=`, so the pre-bridge hand-eval doesn't fire;
        // there's also no published_value or diagnostics → Pending.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_view, ResultView::Pending);
    }

    #[test]
    fn literal_number_input_renders_inline_as_display_number() {
        // `1.5` is a literal number cell entry; the bridge doesn't run for
        // these. The home shell evaluates inline.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "1.5");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "1.5");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '1.5'), got {other:?}"),
        }
    }

    #[test]
    fn literal_text_input_renders_inline_as_display_text() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "hello");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "hello");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, 'hello'), got {other:?}"),
        }
    }

    #[test]
    fn forced_text_input_strips_leading_apostrophe() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "'123");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind } => {
                assert_eq!(text, "123");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, '123'), got {other:?}"),
        }
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
