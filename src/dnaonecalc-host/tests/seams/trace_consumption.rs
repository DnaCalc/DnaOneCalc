//! SEAM-ONECALC-TRACE-CONSUMPTION
//!
//! Target: `RetainedArtifactRecord` carries `oxreplay` trace events and
//! the Workbench cluster exposes a trace timeline. Today no trace events
//! are carried.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-TRACE-CONSUMPTION: `RetainedArtifactRecord` must
/// carry `trace_events: Vec<OxReplayTraceEvent>` populated from the
/// verification bundle report.
///
/// Passes when a bundle import that includes trace events produces a
/// retained artifact record with the event list populated.
///
/// Ownership: Phase B step 5 (Parity Matrix) or a dedicated trace bead.
#[test]
#[ignore = "pending SEAM-ONECALC-TRACE-CONSUMPTION"]
fn retained_artifact_carries_oxreplay_trace_events() {
    seam_pending(
        "SEAM-ONECALC-TRACE-CONSUMPTION",
        "RetainedArtifactRecord must carry trace_events from the verification bundle",
    );
}

/// Pending SEAM-ONECALC-TRACE-CONSUMPTION: the Workbench cluster projects
/// the trace events as a timeline with query id linkage.
#[test]
#[ignore = "pending SEAM-ONECALC-TRACE-CONSUMPTION"]
fn workbench_cluster_exposes_trace_timeline() {
    seam_pending(
        "SEAM-ONECALC-TRACE-CONSUMPTION",
        "WorkbenchViewModel must expose a typed trace_timeline linked by query_id",
    );
}
