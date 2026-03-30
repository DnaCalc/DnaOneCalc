use std::env;

use dnaonecalc_host::{
    launch_shell, launch_shell_with_formula, OneCalcHostProfile, RuntimeAdapter,
};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("--probe") => run_probe(),
        Some("--shell-smoke") => run_shell(true),
        Some("--editor-diagnostic-smoke") => run_editor_diagnostic_smoke(),
        Some(flag) => {
            eprintln!("unknown flag: {flag}");
            eprintln!("supported flags: --probe, --shell-smoke, --editor-diagnostic-smoke");
            std::process::exit(2);
        }
        None => run_shell(false),
    }
}

fn run_probe() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let packet_register = adapter
        .packet_kinds()
        .iter()
        .map(|packet| packet.id())
        .collect::<Vec<_>>()
        .join(",");

    match adapter.dependency_probe() {
        Ok(report) => {
            println!("dnaonecalc-host dependency probe");
            println!("host_profile={}", adapter.host_profile().id());
            println!("packet_kinds={packet_register}");
            println!("formula_token={}", report.formula_token);
            println!("parse_token_count={}", report.parse_token_count);
            println!("parse_diagnostic_count={}", report.parse_diagnostic_count);
            println!("sum_result={}", report.sum_result);
            println!("replay_ready={}", report.replay_ready);
            println!(
                "replay_registry_ref_count={}",
                report.replay_registry_ref_count
            );
        }
        Err(error) => {
            eprintln!("dependency probe failed: {error:?}");
            std::process::exit(1);
        }
    }
}

fn run_shell(smoke_mode: bool) {
    if let Err(error) = launch_shell(smoke_mode) {
        eprintln!("failed to launch dnaonecalc shell: {error}");
        std::process::exit(1);
    }
}

fn run_editor_diagnostic_smoke() {
    if let Err(error) = launch_shell_with_formula("=SUM(1,", true) {
        eprintln!("failed to launch dnaonecalc editor diagnostic smoke: {error}");
        std::process::exit(1);
    }
}
