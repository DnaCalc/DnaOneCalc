use std::env;

use dnaonecalc_host::{
    launch_shell, launch_shell_with_formula, OneCalcHostProfile, RecalcContext,
    RetainedScenarioStore, RuntimeAdapter,
};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("--probe") => run_probe(),
        Some("--function-surface-smoke") => run_function_surface_smoke(),
        Some("--capability-snapshot-smoke") => run_capability_snapshot_smoke(),
        Some("--h1-smoke") => run_h1_smoke(),
        Some("--h1-retained-smoke") => run_h1_retained_smoke(),
        Some("--h1-compare-smoke") => run_h1_compare_smoke(),
        Some("--document-roundtrip-smoke") => run_document_roundtrip_smoke(),
        Some("--shell-smoke") => run_shell(true),
        Some("--editor-diagnostic-smoke") => run_editor_diagnostic_smoke(),
        Some(flag) => {
            eprintln!("unknown flag: {flag}");
            eprintln!(
                "supported flags: --probe, --function-surface-smoke, --capability-snapshot-smoke, --h1-smoke, --h1-retained-smoke, --h1-compare-smoke, --document-roundtrip-smoke, --shell-smoke, --editor-diagnostic-smoke"
            );
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

fn run_function_surface_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let catalog = adapter.load_function_surface_catalog();
    let summary = catalog.label_summary();

    let abs = catalog
        .get("ABS")
        .expect("ABS should exist in the snapshot");
    let call = catalog
        .get("CALL")
        .expect("CALL should exist in the snapshot");
    let accrint = catalog
        .get("ACCRINT")
        .expect("ACCRINT should exist in the snapshot");
    let encodeurl = catalog
        .get("ENCODEURL")
        .expect("ENCODEURL should exist in the snapshot");

    println!("dnaonecalc-host function surface smoke");
    println!(
        "label_summary=supported:{};preview:{};experimental:{};deferred:{};catalog_only:{}",
        summary.supported,
        summary.preview,
        summary.experimental,
        summary.deferred,
        summary.catalog_only
    );
    println!(
        "ABS={} CALL={} ACCRINT={} ENCODEURL={}",
        abs.admission_category.id(),
        call.admission_category.id(),
        accrint.admission_category.id(),
        encodeurl.admission_category.id()
    );
}

fn run_capability_snapshot_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let snapshot = adapter
        .emit_capability_snapshot("edit_accept_recalc", None)
        .expect("capability snapshot should emit");

    println!("dnaonecalc-host capability snapshot smoke");
    println!("capability_snapshot_id={}", snapshot.capability_snapshot_id);
    println!("capability_floor={}", snapshot.capability_floor);
    println!(
        "function_surface_snapshot_ref={}",
        snapshot.function_surface_snapshot_ref
    );
    println!("packet_kinds={}", snapshot.packet_kind_register.join(","));
    for mode in &snapshot.mode_availability {
        println!(
            "mode={} state={} reason={}",
            mode.mode_id,
            mode.state,
            mode.reason.as_deref().unwrap_or("none")
        );
    }
}

fn run_h1_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit = adapter
        .edit_accept_recalc(
            &mut host,
            "=SUM(1,2,3)",
            RecalcContext::edit_accept(Some(46_000.0), Some(0.25)),
        )
        .expect("edit-and-accept recalc should succeed");
    let manual = adapter
        .manual_recalc(&mut host, RecalcContext::manual(Some(46_000.0), Some(0.25)))
        .expect("manual recalc should succeed");
    let forced = adapter
        .forced_recalc(&mut host, RecalcContext::forced(Some(46_000.0), Some(0.25)))
        .expect("forced recalc should succeed");

    println!("dnaonecalc-host h1 smoke");
    println!("host_profile={}", edit.host_profile_id);
    println!(
        "edit_accept=trigger:{};packet_kind:{};formula_text_version:{};structure_context_version:{};worksheet_value:{}",
        edit.trigger_kind,
        edit.packet_kind,
        edit.formula_text_version,
        edit.structure_context_version,
        edit.evaluation.worksheet_value_summary
    );
    println!(
        "manual_recalc=trigger:{};packet_kind:{};formula_text_version:{};worksheet_value:{}",
        manual.trigger_kind,
        manual.packet_kind,
        manual.formula_text_version,
        manual.evaluation.worksheet_value_summary
    );
    println!(
        "forced_recalc=trigger:{};packet_kind:{};formula_text_version:{};worksheet_value:{}",
        forced.trigger_kind,
        forced.packet_kind,
        forced.formula_text_version,
        forced.evaluation.worksheet_value_summary
    );
}

fn run_h1_retained_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.retained", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");
    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context.clone())
        .expect("edit-and-accept recalc should succeed");

    let root = env::temp_dir().join("dnaonecalc-h1-retained-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let persisted = adapter
        .persist_driven_scenario_run(&store, &host, &edit_context, &edit, "SUM retained smoke")
        .expect("retained scenario and run should persist");
    let mut reopened = adapter
        .reopen_driven_scenario_run(&store, &persisted.run.scenario_run_id)
        .expect("retained run should reopen");
    let reopened_summary = adapter
        .manual_recalc(
            &mut reopened.driven_host,
            RecalcContext::manual(Some(46_000.0), Some(0.25)),
        )
        .expect("reopened driven host should recalc");

    println!("dnaonecalc-host h1 retained smoke");
    println!("scenario_id={}", persisted.scenario.scenario_id);
    println!("scenario_run_id={}", persisted.run.scenario_run_id);
    println!("scenario_path={}", persisted.scenario_path.display());
    println!("run_path={}", persisted.run_path.display());
    println!(
        "reopened=host_profile:{};formula_text_version:{};worksheet_value:{}",
        reopened_summary.host_profile_id,
        reopened_summary.formula_text_version,
        reopened_summary.evaluation.worksheet_value_summary
    );
}

fn run_h1_compare_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-h1-compare-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.compare", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let first_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", first_context)
        .expect("first retained run should succeed");
    let first = adapter
        .persist_driven_scenario_run(&store, &host, &first_context, &first_summary, "SUM compare")
        .expect("first retained run should persist");

    let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let second_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,4)", second_context)
        .expect("second retained run should succeed");
    let second = adapter
        .persist_driven_scenario_run(
            &store,
            &host,
            &second_context,
            &second_summary,
            "SUM compare",
        )
        .expect("second retained run should persist");

    let comparison = adapter
        .compare_retained_driven_runs(
            &store,
            &first.run.scenario_run_id,
            &second.run.scenario_run_id,
        )
        .expect("retained driven runs should compare");

    println!("dnaonecalc-host h1 compare smoke");
    println!("left_run_id={}", comparison.left_run_id);
    println!("right_run_id={}", comparison.right_run_id);
    println!(
        "comparison=same_scenario:{};formula_version_changed:{};formula_text_changed:{};worksheet_value_match:{};reliability:{}",
        comparison.same_scenario,
        comparison.formula_version_changed,
        comparison.formula_text_changed,
        comparison.worksheet_value_match,
        comparison.reliability_badge
    );
}

fn run_document_roundtrip_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-document-roundtrip-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let document_path = root.join("sum-document.xml");
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.document", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context)
        .expect("document recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&store, &host, &edit_context, &edit, "SUM document")
        .expect("retained run should persist");
    let persisted_document = adapter
        .persist_isolated_document(
            &document_path,
            &host,
            &edit_context,
            &edit,
            "SUM document",
            Some(&retained),
        )
        .expect("isolated document should persist");
    let invariants = adapter
        .verify_isolated_document_roundtrip_invariants(&persisted_document)
        .expect("document invariants should survive round-trip");
    let mut reopened = adapter
        .reopen_isolated_document(&persisted_document.document_path)
        .expect("isolated document should reopen");
    let reopened_summary = adapter
        .manual_recalc(
            &mut reopened.driven_host,
            RecalcContext::manual(Some(46_000.0), Some(0.25)),
        )
        .expect("reopened document should recalc");

    println!("dnaonecalc-host document roundtrip smoke");
    println!("document_id={}", reopened.document.document_id);
    println!(
        "document_path={}",
        persisted_document.document_path.display()
    );
    println!(
        "document_scope={};format={};artifacts={}",
        reopened.document.document_scope,
        reopened.document.persistence_format_id,
        reopened.document.artifact_index.len()
    );
    println!(
        "invariants=document_id:{};formula_identity:{};structure_context:{};library_context_snapshot_ref:{};artifact_index:{};effective_display_status:{}",
        invariants.document_id_preserved,
        invariants.formula_identity_preserved,
        invariants.structure_context_preserved,
        invariants.library_context_snapshot_ref_preserved,
        invariants.artifact_index_preserved,
        invariants.effective_display_status_preserved
    );
    println!(
        "reopened=host_profile:{};formula_text_version:{};worksheet_value:{}",
        reopened_summary.host_profile_id,
        reopened_summary.formula_text_version,
        reopened_summary.evaluation.worksheet_value_summary
    );
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
