use dnaonecalc_host::{OneCalcHostProfile, RuntimeAdapter};

fn main() {
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
