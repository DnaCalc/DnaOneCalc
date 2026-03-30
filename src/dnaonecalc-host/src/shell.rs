use eframe::egui;
use egui::TextBuffer as _;

use crate::{FormulaEditPacketSummary, FormulaEditorSession, OneCalcHostProfile, RuntimeAdapter};

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
    rendered_diagnostics: Vec<String>,
    host_profile_id: String,
    packet_register_text: String,
    platform_gate_text: String,
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
        let probe = adapter.dependency_probe().ok();
        let mut edit_session = FormulaEditorSession::new("onecalc.editor");
        let latest_edit_packet =
            adapter.apply_formula_edit_packet(&mut edit_session, &formula_text);
        let rendered_diagnostics = Self::render_live_diagnostics(&edit_session);
        let result_text = match &probe {
            Some(report) => format!(
                "result: {}\nformula_token: {}\nhost_profile: {}",
                report.sum_result, report.formula_token, host_profile_id
            ),
            None => format!("result: unavailable\nhost_profile: {}", host_profile_id),
        };
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

        Self {
            runtime_adapter: adapter,
            edit_session,
            latest_edit_packet,
            rendered_diagnostics,
            host_profile_id,
            packet_register_text,
            platform_gate_text,
            editor_state: FormulaEditorState::new(formula_text),
            result_text,
            diagnostics_text,
            editor_focus_requested: false,
            smoke_mode,
            smoke_reported: false,
        }
    }

    pub const fn region_ids() -> &'static [&'static str] {
        &[FORMULA_REGION_ID, RESULT_REGION_ID, DIAGNOSTICS_REGION_ID]
    }

    fn sync_edit_packet(&mut self) {
        self.latest_edit_packet = self
            .runtime_adapter
            .apply_formula_edit_packet(&mut self.edit_session, self.editor_state.buffer.clone());
        self.rendered_diagnostics = Self::render_live_diagnostics(&self.edit_session);
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
                "shell_truth=host_profile:{};packet_kinds:{};platform_gate:{}",
                self.host_profile_id,
                self.packet_register_text.replace(", ", "|"),
                self.platform_gate_text
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
        assert!(app.result_text.contains("result: 6"));
        assert!(app
            .diagnostics_text
            .contains("edit_packet_diagnostic_count"));
        assert_eq!(app.host_profile_id, "OC-H0");
        assert!(app.packet_register_text.contains("formula_edit"));
        assert!(app.platform_gate_text.contains("Desktop native host only"));
        assert_eq!(
            app.editor_state.cursor_index,
            app.editor_state.buffer.chars().count()
        );
        assert!(!app.latest_edit_packet.formula_token.is_empty());
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
}
