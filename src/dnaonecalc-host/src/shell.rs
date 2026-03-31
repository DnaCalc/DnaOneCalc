use eframe::egui;
use egui::TextBuffer as _;

use crate::{
    FormulaEditPacketSummary, FormulaEditorSession, FormulaEvaluationSummary, OneCalcHostProfile,
    RuntimeAdapter,
};

pub const FORMULA_REGION_ID: &str = "formula";
pub const RESULT_REGION_ID: &str = "result";
pub const DIAGNOSTICS_REGION_ID: &str = "diagnostics";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditorState {
    pub buffer: String,
    pub cursor_index: usize,
    pub selection_start: usize,
    pub selection_end: usize,
    pub selected_text: String,
}

impl FormulaEditorState {
    fn new(buffer: impl Into<String>) -> Self {
        let buffer = buffer.into();
        let cursor_index = buffer.chars().count();

        Self {
            buffer,
            cursor_index,
            selection_start: cursor_index,
            selection_end: cursor_index,
            selected_text: String::new(),
        }
    }

    fn sync_from_output(&mut self, output: &egui::text_edit::TextEditOutput) {
        if let Some(cursor_range) = output.cursor_range {
            let sorted_chars = cursor_range.as_sorted_char_range();
            self.cursor_index = cursor_range.primary.ccursor.index;
            self.selection_start = sorted_chars.start;
            self.selection_end = sorted_chars.end;
            self.selected_text = self.buffer.char_range(sorted_chars).to_string();
        }
    }
}

pub struct OneCalcShellApp {
    runtime_adapter: RuntimeAdapter,
    edit_session: FormulaEditorSession,
    latest_edit_packet: FormulaEditPacketSummary,
    latest_evaluation: Option<FormulaEvaluationSummary>,
    completion_items: Vec<String>,
    function_help_text: String,
    rendered_diagnostics: Vec<String>,
    host_profile_id: String,
    packet_register_text: String,
    platform_gate_text: String,
    function_policy_text: String,
    editor_state: FormulaEditorState,
    result_text: String,
    diagnostics_text: String,
    editor_focus_requested: bool,
    smoke_mode: bool,
    smoke_reported: bool,
}

impl OneCalcShellApp {
    pub fn new(adapter: RuntimeAdapter, smoke_mode: bool) -> Self {
        Self::with_formula(adapter, "=SUM(1,2,3)".to_string(), smoke_mode)
    }

    fn with_formula(adapter: RuntimeAdapter, formula_text: String, smoke_mode: bool) -> Self {
        let host_profile_id = adapter.host_profile().id().to_string();
        let packet_register_text = adapter
            .packet_kinds()
            .iter()
            .map(|packet| packet.id())
            .collect::<Vec<_>>()
            .join(", ");
        let platform_gate_text = adapter.platform_gate().message().to_string();
        let function_summary = adapter.function_surface_summary();
        let function_policy_text = format!(
            "Function Policy: supported={} preview={} experimental={} deferred={} catalog_only={} executable=supported+preview only",
            function_summary.supported,
            function_summary.preview,
            function_summary.experimental,
            function_summary.deferred,
            function_summary.catalog_only
        );
        let probe = adapter.dependency_probe().ok();
        let mut edit_session = FormulaEditorSession::new("onecalc.editor");
        let latest_edit_packet =
            adapter.apply_formula_edit_packet(&mut edit_session, &formula_text);
        let completion_items = adapter
            .collect_completion_proposals(&edit_session, formula_text.chars().count())
            .into_iter()
            .map(|proposal| format!("{} {}", proposal.proposal_kind, proposal.display_text))
            .collect();
        let rendered_diagnostics = Self::render_live_diagnostics(&edit_session);
        let result_text = "result: not evaluated yet".to_string();
        let diagnostics_text = match &probe {
            Some(report) => format!(
                "probe_parse_diagnostic_count: {}\nedit_packet_diagnostic_count: {}\nreplay_ready: {}\npacket_kinds: {}",
                report.parse_diagnostic_count,
                latest_edit_packet.diagnostic_count,
                report.replay_ready,
                packet_register_text
            ),
            None => "dependency probe failed before shell render".to_string(),
        };

        let mut app = Self {
            runtime_adapter: adapter,
            edit_session,
            latest_edit_packet,
            latest_evaluation: None,
            completion_items,
            function_help_text: "Current Help: unavailable at cursor".to_string(),
            rendered_diagnostics,
            host_profile_id,
            packet_register_text,
            platform_gate_text,
            function_policy_text,
            editor_state: FormulaEditorState::new(formula_text),
            result_text,
            diagnostics_text,
            editor_focus_requested: false,
            smoke_mode,
            smoke_reported: false,
        };

        if smoke_mode {
            app.evaluate_current_formula();
        }
        app.refresh_function_help();

        app
    }

    pub const fn region_ids() -> &'static [&'static str] {
        &[FORMULA_REGION_ID, RESULT_REGION_ID, DIAGNOSTICS_REGION_ID]
    }

    fn sync_edit_packet(&mut self) {
        self.latest_edit_packet = self
            .runtime_adapter
            .apply_formula_edit_packet(&mut self.edit_session, self.editor_state.buffer.clone());
        self.refresh_completion_proposals();
        self.rendered_diagnostics = Self::render_live_diagnostics(&self.edit_session);
    }

    fn refresh_completion_proposals(&mut self) {
        self.completion_items = self
            .runtime_adapter
            .collect_completion_proposals(&self.edit_session, self.editor_state.cursor_index)
            .into_iter()
            .map(|proposal| format!("{} {}", proposal.proposal_kind, proposal.display_text))
            .collect();
        self.refresh_function_help();
    }

    fn refresh_function_help(&mut self) {
        self.function_help_text = match self
            .runtime_adapter
            .current_function_help(&self.edit_session, self.editor_state.cursor_index)
        {
            Some(help) => format!(
                "Current Help: {}\nsignature: {}\nactive_argument: {}\navailability: {}\nprovisional: {}",
                help.display_name,
                help.display_signature,
                help.active_argument_index,
                help.availability_summary,
                help.provisional
            ),
            None => "Current Help: unavailable at cursor".to_string(),
        };
    }

    fn evaluate_current_formula(&mut self) {
        match self
            .runtime_adapter
            .evaluate_formula(self.editor_state.buffer.clone())
        {
            Ok(summary) => {
                self.result_text = format!(
                    "worksheet_value: {}\npayload_summary: {}\nreturned_surface: {}\neffective_display: {}\ncommit_decision: {}",
                    summary.worksheet_value_summary,
                    summary.payload_summary,
                    summary.returned_value_surface_kind,
                    summary.effective_display_status,
                    summary.commit_decision_kind
                );
                self.latest_evaluation = Some(summary);
            }
            Err(error) => {
                self.result_text = format!("evaluation failed: {error}");
                self.latest_evaluation = None;
            }
        }
    }

    fn render_live_diagnostics(edit_session: &FormulaEditorSession) -> Vec<String> {
        edit_session
            .latest_result()
            .map(|result| {
                result
                    .live_diagnostics
                    .diagnostics
                    .iter()
                    .map(|diagnostic| {
                        format!(
                            "{:?} {:?} {}..{} {}",
                            diagnostic.severity,
                            diagnostic.stage,
                            diagnostic.primary_span.start,
                            diagnostic.primary_span.end(),
                            diagnostic.message
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl eframe::App for OneCalcShellApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("context_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Host Profile: {}", self.host_profile_id));
                ui.separator();
                ui.label(format!("Packet Kinds: {}", self.packet_register_text));
                ui.separator();
                ui.colored_label(egui::Color32::YELLOW, &self.platform_gate_text);
                ui.separator();
                ui.label(&self.function_policy_text);
            });
        });

        egui::TopBottomPanel::top(FORMULA_REGION_ID).show(ctx, |ui| {
            ui.heading("Formula");
            let output = egui::TextEdit::multiline(&mut self.editor_state.buffer)
                .desired_rows(3)
                .hint_text("Enter a formula")
                .show(ui);
            if !self.editor_focus_requested {
                output.response.request_focus();
                self.editor_focus_requested = true;
            }
            self.editor_state.sync_from_output(&output);
            if output.response.changed() {
                self.sync_edit_packet();
            }
            if ui.button("Evaluate").clicked() {
                self.evaluate_current_formula();
            }
            ui.separator();
            ui.label("Completions");
            if self.completion_items.is_empty() {
                ui.small("No deterministic proposals at the current cursor.");
            } else {
                for proposal in self.completion_items.iter().take(6) {
                    ui.monospace(proposal);
                }
            }
            ui.separator();
            ui.label("Current Help");
            ui.monospace(&self.function_help_text);
            ui.small(format!(
                "cursor={} selection={}..{} selected_text=\"{}\"",
                self.editor_state.cursor_index,
                self.editor_state.selection_start,
                self.editor_state.selection_end,
                self.editor_state.selected_text
            ));
        });

        egui::SidePanel::right(DIAGNOSTICS_REGION_ID)
            .resizable(true)
            .min_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Diagnostics");
                ui.separator();
                ui.label(&self.diagnostics_text);
                ui.separator();
                ui.monospace(format!(
                    "buffer_len={}\ncursor_index={}\nselection={}..{}\nselected_text={}\nedit_formula_token={}\nedit_diagnostic_count={}\ntext_change_range={:?}\nreused_green_tree={}\nreused_red_projection={}\nreused_bound_formula={}",
                    self.editor_state.buffer.chars().count(),
                    self.editor_state.cursor_index,
                    self.editor_state.selection_start,
                    self.editor_state.selection_end,
                    self.editor_state.selected_text,
                    self.latest_edit_packet.formula_token,
                    self.latest_edit_packet.diagnostic_count,
                    self.latest_edit_packet.text_change_range,
                    self.latest_edit_packet.reused_green_tree,
                    self.latest_edit_packet.reused_red_projection,
                    self.latest_edit_packet.reused_bound_formula
                ));
                ui.separator();
                ui.heading("Live Diagnostics");
                if self.rendered_diagnostics.is_empty() {
                    ui.label("No live diagnostics.");
                } else {
                    for diagnostic in &self.rendered_diagnostics {
                        ui.label(diagnostic);
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.push_id(RESULT_REGION_ID, |ui| {
                ui.heading("Result");
                ui.separator();
                ui.code(&self.result_text);
            });
        });

        if self.smoke_mode && !self.smoke_reported {
            println!("shell_regions={}", Self::region_ids().join(","));
            println!(
                "shell_truth=host_profile:{};packet_kinds:{};platform_gate:{};function_policy:{}",
                self.host_profile_id,
                self.packet_register_text.replace(", ", "|"),
                self.platform_gate_text,
                self.function_policy_text
                    .replace(": ", "=")
                    .replace(" ", "_")
            );
            println!(
                "editor_truth=buffer_len:{};cursor_index:{};selection:{}..{}",
                self.editor_state.buffer.chars().count(),
                self.editor_state.cursor_index,
                self.editor_state.selection_start,
                self.editor_state.selection_end
            );
            println!(
                "edit_packet=formula_token:{};diagnostic_count:{};text_change_range:{:?};reused_green_tree:{};reused_red_projection:{};reused_bound_formula:{}",
                self.latest_edit_packet.formula_token,
                self.latest_edit_packet.diagnostic_count,
                self.latest_edit_packet.text_change_range,
                self.latest_edit_packet.reused_green_tree,
                self.latest_edit_packet.reused_red_projection,
                self.latest_edit_packet.reused_bound_formula
            );
            println!("live_diagnostic_lines={}", self.rendered_diagnostics.len());
            if let Some(summary) = &self.latest_evaluation {
                println!(
                    "evaluation_truth=formula_token:{};worksheet_value:{};payload_summary:{};returned_surface:{};effective_display:{};commit_decision:{};trace_event_count:{}",
                    summary.formula_token,
                    summary.worksheet_value_summary,
                    summary.payload_summary,
                    summary.returned_value_surface_kind,
                    summary.effective_display_status,
                    summary.commit_decision_kind,
                    summary.trace_event_count
                );
            }
            self.smoke_reported = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn launch_shell(smoke_mode: bool) -> Result<(), eframe::Error> {
    launch_shell_with_formula("=SUM(1,2,3)", smoke_mode)
}

pub fn launch_shell_with_formula(
    formula_text: impl Into<String>,
    smoke_mode: bool,
) -> Result<(), eframe::Error> {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let formula_text = formula_text.into();
    let title = if smoke_mode {
        "DNA OneCalc Shell Smoke"
    } else {
        "DNA OneCalc"
    };

    eframe::run_native(
        title,
        eframe::NativeOptions::default(),
        Box::new(move |_cc| {
            Ok(Box::new(OneCalcShellApp::with_formula(
                adapter,
                formula_text,
                smoke_mode,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_app_exposes_the_three_core_regions() {
        assert_eq!(
            OneCalcShellApp::region_ids(),
            &[FORMULA_REGION_ID, RESULT_REGION_ID, DIAGNOSTICS_REGION_ID]
        );
    }

    #[test]
    fn shell_app_seeds_result_and_diagnostics_from_runtime_truth() {
        let app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), true);

        assert!(app.editor_state.buffer.contains("SUM"));
        assert!(app.result_text.contains("worksheet_value: Number(6"));
        assert!(app.result_text.contains("payload_summary: Number"));
        assert!(app.result_text.contains("effective_display: none"));
        assert!(app
            .diagnostics_text
            .contains("edit_packet_diagnostic_count"));
        assert_eq!(app.host_profile_id, "OC-H0");
        assert!(app.packet_register_text.contains("formula_edit"));
        assert!(app.platform_gate_text.contains("Desktop native host only"));
        assert!(app.function_policy_text.contains("supported="));
        assert!(app
            .function_policy_text
            .contains("executable=supported+preview only"));
        assert!(app.function_help_text.contains("Current Help"));
        assert_eq!(
            app.editor_state.cursor_index,
            app.editor_state.buffer.chars().count()
        );
        assert!(!app.latest_edit_packet.formula_token.is_empty());
        assert!(app.latest_evaluation.is_some());
        assert!(app.rendered_diagnostics.is_empty());
    }

    #[test]
    fn shell_app_projects_live_diagnostics_and_spans_for_invalid_formula() {
        let app = OneCalcShellApp::with_formula(
            RuntimeAdapter::new(OneCalcHostProfile::OcH0),
            "=SUM(1,".to_string(),
            true,
        );

        assert!(!app.rendered_diagnostics.is_empty());
        assert!(app.rendered_diagnostics[0].contains("Syntax"));
        assert!(app.rendered_diagnostics[0].contains(".."));
    }

    #[test]
    fn shell_app_projects_function_completion_into_editor_flow() {
        let app = OneCalcShellApp::with_formula(
            RuntimeAdapter::new(OneCalcHostProfile::OcH0),
            "=SU".to_string(),
            false,
        );

        assert!(app
            .completion_items
            .iter()
            .any(|proposal| proposal == "Function SUM"));
    }

    #[test]
    fn shell_app_projects_current_function_help_into_editor_flow() {
        let app = OneCalcShellApp::with_formula(
            RuntimeAdapter::new(OneCalcHostProfile::OcH0),
            "=SUM(1,2,3".to_string(),
            false,
        );

        assert!(app.function_help_text.contains("Current Help: SUM"));
        assert!(app.function_help_text.contains("signature: SUM(1..255)"));
        assert!(app.function_help_text.contains("availability: supported"));
    }
}
