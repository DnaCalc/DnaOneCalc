//! Adapter from native OneCalc state to the Leptos-free shared session core.

impl dnaonecalc_core::OneCalcSessionHost for crate::state::OneCalcHostState {
    fn snapshot(&self) -> dnacalc_skin_ir::SkinSnapshot {
        crate::services::home_shell_view_model::build_home_shell_view_model(self)
            .expect("OneCalc session requires an active formula space")
            .skin_snapshot
    }

    #[allow(unreachable_code, unused_variables)]
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
        if let dnacalc_skin_ir::SkinIntent::Shell(shell) = &intent {
            match shell {
                dnacalc_skin_ir::SkinShellIntent::Save => {
                    crate::persistence::save_workspace_to_local_storage(self);
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::SaveAs { suggested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnaonecalc_core::DispatchOutcome::Rejected(
                        dnacalc_skin_ir::SkinIntentDiagnostic {
                            code: "browser_file_picker_required".into(),
                            message:
                                "browser SaveAs requires the asynchronous file-download adapter"
                                    .into(),
                            recoverable: true,
                        },
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let Some(path) = suggested_path.as_deref() else {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "save_as_path_required".into(),
                                    message: "native SaveAs requires a path".into(),
                                    recoverable: true,
                                },
                            );
                        };
                        if let Err(message) = crate::persistence::save_workspace_to_path(self, path)
                        {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "save_as_failed".into(),
                                    message,
                                    recoverable: true,
                                },
                            );
                        }
                    }
                    self.workspace_shell.current_workspace_path = suggested_path.clone();
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::Open { requested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnaonecalc_core::DispatchOutcome::Rejected(
                        dnacalc_skin_ir::SkinIntentDiagnostic {
                            code: "browser_file_picker_required".into(),
                            message: "browser Open requires the asynchronous file-input adapter"
                                .into(),
                            recoverable: true,
                        },
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let Some(path) = requested_path.as_deref() else {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "open_path_required".into(),
                                    message: "native Open requires a path".into(),
                                    recoverable: true,
                                },
                            );
                        };
                        if let Err(message) =
                            crate::persistence::open_workspace_from_path(self, path)
                        {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "open_failed".into(),
                                    message,
                                    recoverable: true,
                                },
                            );
                        }
                    }
                    self.workspace_shell.current_workspace_path = requested_path.clone();
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                _ => {}
            }
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_save_as_and_open_use_the_requested_path() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/ws16/native-path-workspace.json");
        let path_text = path.to_string_lossy().to_string();
        let mut save = dnaonecalc_core::OneCalcSession::new(
            crate::app::preview_state::preview_minimal_host_state(),
        );
        let receipt = save.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "save-as",
            None,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::SaveAs {
                suggested_path: Some(path_text.clone()),
            }),
        ));
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
        ));
        assert!(path.exists());

        let mut open = dnaonecalc_core::OneCalcSession::new(
            crate::app::preview_state::preview_minimal_host_state(),
        );
        let receipt = open.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "open",
            None,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::Open {
                requested_path: Some(path_text.clone()),
            }),
        ));
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
        ));
        assert_eq!(
            open.host()
                .workspace_shell
                .current_workspace_path
                .as_deref(),
            Some(path_text.as_str())
        );
    }
}
