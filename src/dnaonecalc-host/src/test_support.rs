use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    DrivenRecalcSummary, DrivenSingleFormulaHost, OneCalcHostProfile, PersistedObservation,
    PersistedReplayCapture, PersistedScenarioRun, RecalcContext, RetainedScenarioStore,
    RuntimeAdapter,
};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) struct FixtureRoot {
    root: PathBuf,
}

impl FixtureRoot {
    pub(crate) fn new(label: &str) -> Self {
        let fixture_id = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "dnaonecalc-fixture-{label}-{}-{fixture_id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        Self { root }
    }

    pub(crate) fn join(&self, child: impl AsRef<Path>) -> PathBuf {
        self.root.join(child)
    }

    pub(crate) fn retained_store(&self) -> RetainedScenarioStore {
        RetainedScenarioStore::new(self.join("retained"))
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormulaScenarioFamily {
    ExplorerSum,
    ExplorerInvalid,
    ExplorerCompletionStem,
    ExplorerSequence23,
    RetainedSumBaseline,
    RetainedSumShifted,
    ObservationCompareSum,
}

impl FormulaScenarioFamily {
    pub(crate) const fn formula(self) -> &'static str {
        match self {
            Self::ExplorerSum | Self::RetainedSumBaseline => "=SUM(1,2,3)",
            Self::ExplorerInvalid => "=SUM(1,",
            Self::ExplorerCompletionStem => "=SU",
            Self::ExplorerSequence23 => "=SEQUENCE(2,3)",
            Self::RetainedSumShifted => "=SUM(1,2,4)",
            Self::ObservationCompareSum => "=SUM(10,20,12)",
        }
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        match self {
            Self::ExplorerSum => "fixture.oc-h0.explorer.sum",
            Self::ExplorerInvalid => "fixture.oc-h0.explorer.invalid",
            Self::ExplorerCompletionStem => "fixture.oc-h0.explorer.completion",
            Self::ExplorerSequence23 => "fixture.oc-h0.explorer.sequence",
            Self::RetainedSumBaseline | Self::RetainedSumShifted => {
                "fixture.oc-h1.retained.sum-family"
            }
            Self::ObservationCompareSum => "fixture.oc-h1.observation.compare",
        }
    }

    pub(crate) const fn scenario_slug(self) -> &'static str {
        match self {
            Self::ExplorerSum => "Explorer SUM",
            Self::ExplorerInvalid => "Explorer Invalid SUM",
            Self::ExplorerCompletionStem => "Explorer Completion Stem",
            Self::ExplorerSequence23 => "Explorer Sequence",
            Self::RetainedSumBaseline | Self::RetainedSumShifted => "SUM baseline",
            Self::ObservationCompareSum => "Twin compare family",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservationScenarioFamily {
    XlPlayCaptureValuesFormulae001,
}

impl ObservationScenarioFamily {
    fn source_root(self) -> PathBuf {
        match self {
            Self::XlPlayCaptureValuesFormulae001 => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("OxXlPlay")
                .join("states")
                .join("excel")
                .join("xlplay_capture_values_formulae_001"),
        }
    }
}

pub(crate) fn adapter_for(profile: OneCalcHostProfile) -> RuntimeAdapter {
    RuntimeAdapter::new(profile)
}

#[derive(Debug)]
pub(crate) struct DrivenHostFixture {
    pub adapter: RuntimeAdapter,
    pub host: DrivenSingleFormulaHost,
}

impl DrivenHostFixture {
    pub(crate) fn new(profile: OneCalcHostProfile, scenario: FormulaScenarioFamily) -> Self {
        let adapter = adapter_for(profile);
        let host = adapter
            .new_driven_single_formula_host(scenario.stable_id(), scenario.formula())
            .expect("fixture host should initialize");

        Self { adapter, host }
    }

    pub(crate) fn edit_accept(
        &mut self,
        scenario: FormulaScenarioFamily,
        now_serial: f64,
        random_value: f64,
    ) -> (RecalcContext, DrivenRecalcSummary) {
        let context = RecalcContext::edit_accept(Some(now_serial), Some(random_value));
        let summary = self
            .adapter
            .edit_accept_recalc(&mut self.host, scenario.formula(), context)
            .expect("fixture edit-accept recalc should succeed");

        (context, summary)
    }

    pub(crate) fn persist_run(
        &self,
        store: &RetainedScenarioStore,
        context: &RecalcContext,
        summary: &DrivenRecalcSummary,
        scenario: FormulaScenarioFamily,
    ) -> PersistedScenarioRun {
        self.adapter
            .persist_driven_scenario_run(
                store,
                &self.host,
                context,
                summary,
                scenario.scenario_slug(),
            )
            .expect("fixture retained run should persist")
    }

    pub(crate) fn emit_replay_capture(
        &self,
        store: &RetainedScenarioStore,
        retained_run: &PersistedScenarioRun,
    ) -> PersistedReplayCapture {
        self.adapter
            .emit_replay_capture_for_run(store, &retained_run.run.scenario_run_id)
            .expect("fixture replay capture should emit")
    }
}

pub(crate) fn persist_observation_fixture(
    adapter: &RuntimeAdapter,
    store: &RetainedScenarioStore,
    scenario: ObservationScenarioFamily,
) -> PersistedObservation {
    let source_root = scenario.source_root();
    adapter
        .persist_observation_from_existing_source(store, &source_root)
        .expect("fixture observation should persist")
}
