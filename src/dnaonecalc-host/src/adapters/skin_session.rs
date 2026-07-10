//! Adapter from native OneCalc state to the Leptos-free shared session core.

impl dnaonecalc_core::OneCalcSessionHost for crate::state::OneCalcHostState {
    fn snapshot(&self) -> dnacalc_skin_ir::SkinSnapshot {
        crate::services::home_shell_view_model::build_home_shell_view_model(self)
            .expect("OneCalc session requires an active formula space")
            .skin_snapshot
    }

    fn dispatch(
        &mut self,
        intent: dnacalc_skin_ir::SkinIntent,
    ) -> dnaonecalc_core::DispatchOutcome {
        if matches!(intent, dnacalc_skin_ir::SkinIntent::TreeWorkspace(_)) {
            return dnaonecalc_core::DispatchOutcome::Rejected(
                dnacalc_skin_ir::SkinIntentDiagnostic {
                    code: "tree_workspace_unsupported".into(),
                    message: "OneCalc has no tree workspace surface".into(),
                    recoverable: false,
                },
            );
        }
        if crate::app::reducer::apply_skin_intent_to_host_state(self, intent) {
            dnaonecalc_core::DispatchOutcome::Applied
        } else {
            dnaonecalc_core::DispatchOutcome::NoChange
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_state_round_trips_snapshot_and_serialized_intent_receipt() {
        let state = crate::app::preview_state::preview_minimal_host_state();
        let formula_space_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .unwrap()
            .as_str()
            .to_string();
        let mut session = dnaonecalc_core::OneCalcSession::new(state);
        let envelope = dnacalc_skin_ir::SkinIntentEnvelope::new(
            "edit-1",
            None,
            dnacalc_skin_ir::SkinIntent::OneFormula(dnacalc_skin_ir::OneFormulaIntent::EditText {
                formula_space_id,
                text: "=SUM(1,2,3)".into(),
                caret_offset: 11,
            }),
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let receipt_json = session.handle_json(&json).unwrap();
        let receipt: dnacalc_skin_ir::SkinIntentReceipt =
            serde_json::from_str(&receipt_json).unwrap();
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied {
                snapshot_revision: 1,
                ..
            }
        ));
        match session.snapshot().document {
            dnacalc_skin_ir::SkinDocumentProjection::OneFormula(formula) => {
                assert_eq!(formula.raw_entered_cell_text, "=SUM(1,2,3)")
            }
            _ => panic!("expected OneFormula snapshot"),
        }
    }
}
