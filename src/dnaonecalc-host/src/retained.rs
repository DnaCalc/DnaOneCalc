use std::fs;
use std::path::{Path, PathBuf};

use oxreplay_abstractions::ReplayArtifactRef;
use oxreplay_core::ReplayScenario;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityModeAvailabilityRecord {
    pub mode_id: String,
    pub state: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityLedgerSnapshotRecord {
    pub envelope: ArtifactEnvelope,
    pub capability_snapshot_id: String,
    pub emitted_at_unix_ms: u64,
    pub emitter_build_id: String,
    pub host_kind: String,
    pub runtime_platform: String,
    pub runtime_class: String,
    pub dependency_set: Vec<String>,
    pub function_surface_snapshot_ref: String,
    pub seam_pin_set_id: String,
    pub capability_floor: String,
    pub packet_kind_register: Vec<String>,
    pub function_surface_policy_id: String,
    pub mode_availability: Vec<CapabilityModeAvailabilityRecord>,
    pub provisional_seams: Vec<String>,
    pub capability_ceilings: Vec<String>,
    pub lossiness: Vec<String>,
    pub diff_base_refs: Vec<StableArtifactRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayCaptureRecord {
    pub envelope: ArtifactEnvelope,
    pub replay_capture_id: String,
    pub scenario_id: String,
    pub scenario_run_id: String,
    pub scenario_run_ref: StableArtifactRef,
    pub capability_snapshot_ref: StableArtifactRef,
    pub replay_floor: String,
    pub replay_artifact: ReplayArtifactRef,
    pub emitted_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessRecord {
    pub envelope: ArtifactEnvelope,
    pub witness_id: String,
    pub scenario_id: String,
    pub left_run_ref: StableArtifactRef,
    pub right_run_ref: StableArtifactRef,
    pub explain_floor: String,
    pub explanation_lines: Vec<String>,
    pub blocked_dimensions: Vec<String>,
    pub emitted_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandoffReadinessRecord {
    pub item_id: String,
    pub satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffPacketRecord {
    pub envelope: ArtifactEnvelope,
    pub handoff_id: String,
    pub scenario_id: String,
    pub source_run_ref: StableArtifactRef,
    pub witness_ref: StableArtifactRef,
    pub capability_snapshot_ref: StableArtifactRef,
    pub requested_action_kind: String,
    pub target_lane: String,
    pub expected_behavior: String,
    pub observed_behavior: String,
    pub supporting_artifact_refs: Vec<StableArtifactRef>,
    pub reliability_state: String,
    pub status: String,
    pub readiness: Vec<HandoffReadinessRecord>,
    pub emitted_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCapabilitySnapshot {
    pub snapshot: CapabilityLedgerSnapshotRecord,
    pub snapshot_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedScenarioRun {
    pub capability_snapshot: PersistedCapabilitySnapshot,
    pub scenario: ScenarioRecord,
    pub run: ScenarioRunRecord,
    pub scenario_path: PathBuf,
    pub run_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedReplayCapture {
    pub capture: ReplayCaptureRecord,
    pub capture_path: PathBuf,
    pub replay_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedWitness {
    pub witness: WitnessRecord,
    pub witness_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedHandoffPacket {
    pub handoff: HandoffPacketRecord,
    pub handoff_path: PathBuf,
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
        capability_snapshot: &CapabilityLedgerSnapshotRecord,
        scenario: &ScenarioRecord,
        run: &ScenarioRunRecord,
    ) -> Result<PersistedScenarioRun, String> {
        fs::create_dir_all(self.scenarios_dir()).map_err(|error| error.to_string())?;
        fs::create_dir_all(self.runs_dir()).map_err(|error| error.to_string())?;
        fs::create_dir_all(self.capability_snapshots_dir()).map_err(|error| error.to_string())?;

        let capability_snapshot_path =
            self.capability_snapshot_path(&capability_snapshot.capability_snapshot_id);
        let scenario_path = self.scenario_path(&scenario.scenario_id);
        let run_path = self.run_path(&run.scenario_run_id);
        write_json(&capability_snapshot_path, capability_snapshot)?;
        write_json(&scenario_path, scenario)?;
        write_json(&run_path, run)?;

        Ok(PersistedScenarioRun {
            capability_snapshot: PersistedCapabilitySnapshot {
                snapshot: capability_snapshot.clone(),
                snapshot_path: capability_snapshot_path,
            },
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

    pub fn read_scenario(&self, scenario_id: &str) -> Result<ScenarioRecord, String> {
        read_json::<ScenarioRecord>(&self.scenario_path(scenario_id))
    }

    pub fn read_run(&self, scenario_run_id: &str) -> Result<ScenarioRunRecord, String> {
        read_json::<ScenarioRunRecord>(&self.run_path(scenario_run_id))
    }

    pub fn read_capability_snapshot(
        &self,
        capability_snapshot_id: &str,
    ) -> Result<CapabilityLedgerSnapshotRecord, String> {
        read_json::<CapabilityLedgerSnapshotRecord>(
            &self.capability_snapshot_path(capability_snapshot_id),
        )
    }

    pub fn read_replay_capture(
        &self,
        replay_capture_id: &str,
    ) -> Result<ReplayCaptureRecord, String> {
        read_json::<ReplayCaptureRecord>(&self.replay_capture_path(replay_capture_id))
    }

    pub fn read_witness(&self, witness_id: &str) -> Result<WitnessRecord, String> {
        read_json::<WitnessRecord>(&self.witness_path(witness_id))
    }

    pub fn read_handoff_packet(&self, handoff_id: &str) -> Result<HandoffPacketRecord, String> {
        read_json::<HandoffPacketRecord>(&self.handoff_path(handoff_id))
    }

    pub fn persist_replay_capture(
        &self,
        capture: &ReplayCaptureRecord,
        replay_scenario: &ReplayScenario,
    ) -> Result<PersistedReplayCapture, String> {
        fs::create_dir_all(self.replay_captures_dir()).map_err(|error| error.to_string())?;
        let capture_path = self.replay_capture_path(&capture.replay_capture_id);
        let replay_path = self.replay_scenario_path(&capture.replay_capture_id);
        write_json(&capture_path, capture)?;
        write_json(&replay_path, replay_scenario)?;
        Ok(PersistedReplayCapture {
            capture: capture.clone(),
            capture_path,
            replay_path,
        })
    }

    pub fn persist_witness(&self, witness: &WitnessRecord) -> Result<PersistedWitness, String> {
        fs::create_dir_all(self.witnesses_dir()).map_err(|error| error.to_string())?;
        let witness_path = self.witness_path(&witness.witness_id);
        write_json(&witness_path, witness)?;
        Ok(PersistedWitness {
            witness: witness.clone(),
            witness_path,
        })
    }

    pub fn persist_handoff_packet(
        &self,
        handoff: &HandoffPacketRecord,
    ) -> Result<PersistedHandoffPacket, String> {
        fs::create_dir_all(self.handoffs_dir()).map_err(|error| error.to_string())?;
        let handoff_path = self.handoff_path(&handoff.handoff_id);
        write_json(&handoff_path, handoff)?;
        Ok(PersistedHandoffPacket {
            handoff: handoff.clone(),
            handoff_path,
        })
    }

    pub fn overwrite_run(&self, run: &ScenarioRunRecord) -> Result<(), String> {
        write_json(&self.run_path(&run.scenario_run_id), run)
    }

    fn scenarios_dir(&self) -> PathBuf {
        self.root.join("scenarios")
    }

    fn runs_dir(&self) -> PathBuf {
        self.root.join("scenario-runs")
    }

    fn capability_snapshots_dir(&self) -> PathBuf {
        self.root.join("capability-snapshots")
    }

    fn replay_captures_dir(&self) -> PathBuf {
        self.root.join("replay-captures")
    }

    fn witnesses_dir(&self) -> PathBuf {
        self.root.join("witnesses")
    }

    fn handoffs_dir(&self) -> PathBuf {
        self.root.join("handoffs")
    }

    fn scenario_path(&self, scenario_id: &str) -> PathBuf {
        self.scenarios_dir().join(format!("{scenario_id}.json"))
    }

    fn run_path(&self, scenario_run_id: &str) -> PathBuf {
        self.runs_dir().join(format!("{scenario_run_id}.json"))
    }

    fn capability_snapshot_path(&self, capability_snapshot_id: &str) -> PathBuf {
        self.capability_snapshots_dir()
            .join(format!("{capability_snapshot_id}.json"))
    }

    fn replay_capture_path(&self, replay_capture_id: &str) -> PathBuf {
        self.replay_captures_dir()
            .join(format!("{replay_capture_id}.json"))
    }

    fn replay_scenario_path(&self, replay_capture_id: &str) -> PathBuf {
        self.replay_captures_dir()
            .join(format!("{replay_capture_id}.replay.json"))
    }

    fn witness_path(&self, witness_id: &str) -> PathBuf {
        self.witnesses_dir().join(format!("{witness_id}.json"))
    }

    fn handoff_path(&self, handoff_id: &str) -> PathBuf {
        self.handoffs_dir().join(format!("{handoff_id}.json"))
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
