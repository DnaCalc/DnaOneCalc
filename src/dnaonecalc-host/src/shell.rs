use eframe::egui;

use crate::{OneCalcHostProfile, RuntimeAdapter};

pub const FORMULA_REGION_ID: &str = "formula";
pub const RESULT_REGION_ID: &str = "result";
pub const DIAGNOSTICS_REGION_ID: &str = "diagnostics";

pub struct OneCalcShellApp {
    host_profile_id: String,
    packet_register_text: String,
    platform_gate_text: String,
    formula_text: String,
    result_text: String,
    diagnostics_text: String,
    smoke_mode: bool,
    smoke_reported: bool,
}

impl OneCalcShellApp {
    pub fn new(adapter: RuntimeAdapter, smoke_mode: bool) -> Self {
        let host_profile_id = adapter.host_profile().id().to_string();
        let packet_register_text = adapter
            .packet_kinds()
            .iter()
            .map(|packet| packet.id())
            .collect::<Vec<_>>()
            .join(", ");
        let platform_gate_text = adapter.platform_gate().message().to_string();
        let probe = adapter.dependency_probe().ok();
        let formula_text = "=SUM(1,2,3)".to_string();
        let result_text = match &probe {
            Some(report) => format!(
                "result: {}\nformula_token: {}\nhost_profile: {}",
                report.sum_result, report.formula_token, host_profile_id
            ),
            None => format!("result: unavailable\nhost_profile: {}", host_profile_id),
        };
        let diagnostics_text = match &probe {
            Some(report) => format!(
                "parse_diagnostic_count: {}\nreplay_ready: {}\npacket_kinds: {}",
                report.parse_diagnostic_count, report.replay_ready, packet_register_text
            ),
            None => "dependency probe failed before shell render".to_string(),
        };

        Self {
            host_profile_id,
            packet_register_text,
            platform_gate_text,
            formula_text,
            result_text,
            diagnostics_text,
            smoke_mode,
            smoke_reported: false,
        }
    }

    pub const fn region_ids() -> &'static [&'static str] {
        &[FORMULA_REGION_ID, RESULT_REGION_ID, DIAGNOSTICS_REGION_ID]
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
            ui.add(
                egui::TextEdit::multiline(&mut self.formula_text)
                    .desired_rows(3)
                    .hint_text("Enter a formula"),
            );
        });

        egui::SidePanel::right(DIAGNOSTICS_REGION_ID)
            .resizable(true)
            .min_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Diagnostics");
                ui.separator();
                ui.label(&self.diagnostics_text);
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
            self.smoke_reported = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

pub fn launch_shell(smoke_mode: bool) -> Result<(), eframe::Error> {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let title = if smoke_mode {
        "DNA OneCalc Shell Smoke"
    } else {
        "DNA OneCalc"
    };

    eframe::run_native(
        title,
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Ok(Box::new(OneCalcShellApp::new(adapter, smoke_mode)))),
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

        assert!(app.formula_text.contains("SUM"));
        assert!(app.result_text.contains("result: 6"));
        assert!(app.diagnostics_text.contains("parse_diagnostic_count: 0"));
        assert_eq!(app.host_profile_id, "OC-H0");
        assert!(app.packet_register_text.contains("formula_edit"));
        assert!(app.platform_gate_text.contains("Desktop native host only"));
    }
}
