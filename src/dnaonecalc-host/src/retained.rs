use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{ArtifactEnvelope, StableArtifactRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedRecalcContextRecord {
    pub trigger_kind: String,
    pub packet_kind: String,
    pub now_serial: Option<String>,
    pub random_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedProvenanceRecord {
    pub formula_stable_id: String,
    pub formula_text_version: u64,
    pub structure_context_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRecord {
    pub envelope: ArtifactEnvelope,
    pub scenario_id: String,
    pub scenario_slug: String,
    pub formula_text: String,
    pub formula_channel_kind: String,
    pub host_profile_id: String,
    pub host_driving_packet_kind: String,
    pub host_driving_block: String,
    pub recalc_context: RetainedRecalcContextRecord,
    pub display_context: String,
    pub library_context_snapshot_ref: Option<String>,
    pub function_surface_policy_id: String,
    pub retained_notes: Vec<String>,
    pub provenance: RetainedProvenanceRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRunRecord {
    pub envelope: ArtifactEnvelope,
    pub scenario_run_id: String,
    pub scenario_id: String,
    pub scenario_ref: StableArtifactRef,
    pub formula_text_version: u64,
    pub formula_token: String,
    pub authored_formula_text: String,
    pub build_id: String,
    pub runtime_platform: String,
    pub seam_pin_set_id: String,
    pub effective_capability_floor: String,
    pub result_surface_ref: StableArtifactRef,
    pub candidate_ref: Option<StableArtifactRef>,
    pub commit_ref: Option<StableArtifactRef>,
    pub reject_ref: Option<StableArtifactRef>,
    pub trace_ref: Option<StableArtifactRef>,
    pub replay_capture_ref: Option<StableArtifactRef>,
    pub function_surface_effective_id: String,
    pub projection_status: String,
    pub provisionality_status: String,
    pub worksheet_value_summary: String,
    pub payload_summary: String,
    pub returned_value_surface_kind: String,
    pub effective_display_status: String,
    pub commit_decision_kind: String,
    pub executed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedScenarioRun {
    pub scenario: ScenarioRecord,
    pub run: ScenarioRunRecord,
    pub scenario_path: PathBuf,
    pub run_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReopenedScenarioRun {
    pub scenario: ScenarioRecord,
    pub run: ScenarioRunRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedScenarioStore {
    root: PathBuf,
}

impl RetainedScenarioStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn persist_scenario_and_run(
        &self,
        scenario: &ScenarioRecord,
        run: &ScenarioRunRecord,
    ) -> Result<PersistedScenarioRun, String> {
        fs::create_dir_all(self.scenarios_dir()).map_err(|error| error.to_string())?;
        fs::create_dir_all(self.runs_dir()).map_err(|error| error.to_string())?;

        let scenario_path = self.scenario_path(&scenario.scenario_id);
        let run_path = self.run_path(&run.scenario_run_id);
        write_json(&scenario_path, scenario)?;
        write_json(&run_path, run)?;

        Ok(PersistedScenarioRun {
            scenario: scenario.clone(),
            run: run.clone(),
            scenario_path,
            run_path,
        })
    }

    pub fn reopen_run(&self, scenario_run_id: &str) -> Result<ReopenedScenarioRun, String> {
        let run = read_json::<ScenarioRunRecord>(&self.run_path(scenario_run_id))?;
        let scenario = read_json::<ScenarioRecord>(&self.scenario_path(&run.scenario_id))?;
        Ok(ReopenedScenarioRun { scenario, run })
    }

    fn scenarios_dir(&self) -> PathBuf {
        self.root.join("scenarios")
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join("scenario-runs")
    }

    fn scenario_path(&self, scenario_id: &str) -> PathBuf {
        self.scenarios_dir().join(format!("{scenario_id}.json"))
    }

    fn run_path(&self, scenario_run_id: &str) -> PathBuf {
        self.runs_dir().join(format!("{scenario_run_id}.json"))
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let body = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, body).map_err(|error| error.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let body = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}
