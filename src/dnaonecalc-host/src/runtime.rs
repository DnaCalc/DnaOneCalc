use oxfml_core::{parse_formula, FormulaSourceRecord, ParseRequest};

use crate::{run_dependency_probe, DependencyProbeError, DependencyProbeReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneCalcHostProfile {
    OcH0,
    OcH1,
    OcH2,
}

impl OneCalcHostProfile {
    pub const ALL: [Self; 3] = [Self::OcH0, Self::OcH1, Self::OcH2];

    pub const fn id(self) -> &'static str {
        match self {
            Self::OcH0 => "OC-H0",
            Self::OcH1 => "OC-H1",
            Self::OcH2 => "OC-H2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPacketKind {
    FormulaEdit,
    EditAcceptRecalc,
    ManualRecalc,
    ForcedRecalc,
    ReplayCapture,
    ExtensionRegistration,
    RtdUpdate,
}

impl HostPacketKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::FormulaEdit => "formula_edit",
            Self::EditAcceptRecalc => "edit_accept_recalc",
            Self::ManualRecalc => "manual_recalc",
            Self::ForcedRecalc => "forced_recalc",
            Self::ReplayCapture => "replay_capture",
            Self::ExtensionRegistration => "extension_registration",
            Self::RtdUpdate => "rtd_update",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformGate {
    DesktopNativeOnly,
}

impl PlatformGate {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DesktopNativeOnly => "desktop_native_only",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::DesktopNativeOnly => {
                "Desktop native host only; browser and secondary hosts are not admitted yet."
            }
        }
    }
}

const OC_H0_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ReplayCapture,
];

const OC_H1_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ManualRecalc,
    HostPacketKind::ForcedRecalc,
    HostPacketKind::ReplayCapture,
];

const OC_H2_PACKET_KINDS: &[HostPacketKind] = &[
    HostPacketKind::FormulaEdit,
    HostPacketKind::EditAcceptRecalc,
    HostPacketKind::ManualRecalc,
    HostPacketKind::ForcedRecalc,
    HostPacketKind::ReplayCapture,
    HostPacketKind::ExtensionRegistration,
    HostPacketKind::RtdUpdate,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSnapshot {
    pub formula_token: String,
    pub token_count: usize,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAdapter {
    host_profile: OneCalcHostProfile,
}

impl RuntimeAdapter {
    pub const fn new(host_profile: OneCalcHostProfile) -> Self {
        Self { host_profile }
    }

    pub const fn host_profile(&self) -> OneCalcHostProfile {
        self.host_profile
    }

    pub fn packet_kinds(&self) -> &'static [HostPacketKind] {
        match self.host_profile {
            OneCalcHostProfile::OcH0 => OC_H0_PACKET_KINDS,
            OneCalcHostProfile::OcH1 => OC_H1_PACKET_KINDS,
            OneCalcHostProfile::OcH2 => OC_H2_PACKET_KINDS,
        }
    }

    pub const fn platform_gate(&self) -> PlatformGate {
        PlatformGate::DesktopNativeOnly
    }

    pub fn parse_formula_source(&self, source: FormulaSourceRecord) -> ParseSnapshot {
        let formula_token = source.formula_token().0;
        let parse = parse_formula(ParseRequest { source });

        ParseSnapshot {
            formula_token,
            token_count: parse.green_tree.full_fidelity_tokens.len(),
            diagnostic_count: parse.green_tree.diagnostics.len(),
        }
    }

    pub fn dependency_probe(&self) -> Result<DependencyProbeReport, DependencyProbeError> {
        run_dependency_probe()
    }
}
