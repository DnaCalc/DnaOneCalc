use super::{ExtensionPlatform, FrozenExtensionHostContract};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionUpstreamPressureOwner {
    OxVba,
}

impl ExtensionUpstreamPressureOwner {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::OxVba => "oxvba",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionUpstreamPressureStatus {
    DraftOnly,
    PlannedUpstream,
    WindowsOnly,
}

impl ExtensionUpstreamPressureStatus {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::DraftOnly => "draft-only",
            Self::PlannedUpstream => "planned-upstream",
            Self::WindowsOnly => "windows-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionUpstreamPressureRecord {
    pub pressure_id: &'static str,
    pub owner: ExtensionUpstreamPressureOwner,
    pub status: ExtensionUpstreamPressureStatus,
    pub title: &'static str,
    pub summary: &'static str,
    pub local_posture: &'static str,
    pub affected_platforms: &'static [ExtensionPlatform],
    pub upstream_reference_paths: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtensionUpstreamPressureRegister {
    pub records: Vec<ExtensionUpstreamPressureRecord>,
}

impl ExtensionUpstreamPressureRegister {
    pub fn report(&self) -> ExtensionUpstreamPressureReport {
        ExtensionUpstreamPressureReport {
            total_records: self.records.len(),
            draft_only_records: self
                .records
                .iter()
                .filter(|record| {
                    matches!(record.status, ExtensionUpstreamPressureStatus::DraftOnly)
                })
                .count(),
            planned_upstream_records: self
                .records
                .iter()
                .filter(|record| {
                    matches!(
                        record.status,
                        ExtensionUpstreamPressureStatus::PlannedUpstream
                    )
                })
                .count(),
            windows_only_records: self
                .records
                .iter()
                .filter(|record| {
                    matches!(record.status, ExtensionUpstreamPressureStatus::WindowsOnly)
                })
                .count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionUpstreamPressureReport {
    pub total_records: usize,
    pub draft_only_records: usize,
    pub planned_upstream_records: usize,
    pub windows_only_records: usize,
}

impl ExtensionUpstreamPressureReport {
    pub fn summary(&self) -> String {
        format!(
            "{} open / {} draft / {} planned / {} windows-only",
            self.total_records,
            self.draft_only_records,
            self.planned_upstream_records,
            self.windows_only_records
        )
    }
}

pub fn default_extension_upstream_pressure_register(
    _contract: &FrozenExtensionHostContract,
) -> ExtensionUpstreamPressureRegister {
    ExtensionUpstreamPressureRegister {
        records: vec![
            ExtensionUpstreamPressureRecord {
                pressure_id: "OXVBA-ADDIN-TOOLCHAIN-PLANNED",
                owner: ExtensionUpstreamPressureOwner::OxVba,
                status: ExtensionUpstreamPressureStatus::PlannedUpstream,
                title: "OxVba add-in/XLL toolchain is still planned",
                summary: "OxVba documents XLL and add-in support as a planned workset rather than an executable toolchain.",
                local_posture: "Do not claim OxVba-produced add-ins locally; keep the OneCalc portable native-extension ABI as the executable packaging path.",
                affected_platforms: &[
                    ExtensionPlatform::WindowsDesktop,
                    ExtensionPlatform::LinuxDesktop,
                ],
                upstream_reference_paths: &[
                    "../OxVba/docs/worksets/WORKSET_2026-03-23_XLL_ADDIN_SUPPORT_P8.md",
                    "../OxVba/docs/spec/HOSTING_PROJECT_TOOLING_PROPOSAL.md",
                ],
            },
            ExtensionUpstreamPressureRecord {
                pressure_id: "OXVBA-CONSUMER-CONTRACT-DRAFT",
                owner: ExtensionUpstreamPressureOwner::OxVba,
                status: ExtensionUpstreamPressureStatus::DraftOnly,
                title: "OxVba consumer-facing hosting/tooling contract is still draft-grade",
                summary: "The OxVba hosting and packaging direction exists, but the current downstream-facing docs are still design-draft or working-draft rather than a frozen consumer ABI.",
                local_posture: "Treat OxVba as design input and co-development pressure instead of baking private OneCalc assumptions into the extension lane.",
                affected_platforms: &[
                    ExtensionPlatform::WindowsDesktop,
                    ExtensionPlatform::LinuxDesktop,
                    ExtensionPlatform::BrowserWasm,
                ],
                upstream_reference_paths: &[
                    "../OxVba/docs/spec/HOSTING_PROJECT_TOOLING_PROPOSAL.md",
                    "../OxVba/docs/spec/HAL_RUNTIME_PROFILE_MATRIX_V1.md",
                ],
            },
            ExtensionUpstreamPressureRecord {
                pressure_id: "OXVBA-CROSS-PLATFORM-RUNTIME-GAP",
                owner: ExtensionUpstreamPressureOwner::OxVba,
                status: ExtensionUpstreamPressureStatus::WindowsOnly,
                title: "OxVba add-in-facing runtime assumptions remain Windows-specific",
                summary: "OxVba's current runtime-profile and COM posture still centers Windows-capable embedded hosting; that does not yet produce a portable Linux or browser add-in story.",
                local_posture: "Keep Linux and browser extension claims on the OneCalc-owned ABI path instead of pretending OxVba already ships cross-platform add-in packaging.",
                affected_platforms: &[
                    ExtensionPlatform::LinuxDesktop,
                    ExtensionPlatform::BrowserWasm,
                ],
                upstream_reference_paths: &[
                    "../OxVba/docs/spec/HAL_RUNTIME_PROFILE_MATRIX_V1.md",
                    "../OxVba/docs/spec/HOSTING_PROJECT_TOOLING_PROPOSAL.md",
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::frozen_extension_host_contract;

    #[test]
    fn default_extension_pressure_register_keeps_oxvba_as_explicit_pressure() {
        let contract = frozen_extension_host_contract();
        let register = default_extension_upstream_pressure_register(&contract);

        assert_eq!(register.records.len(), 3);
        assert_eq!(
            register
                .records
                .iter()
                .map(|record| record.owner.slug())
                .collect::<Vec<_>>(),
            vec!["oxvba", "oxvba", "oxvba"]
        );
        assert_eq!(
            register
                .records
                .iter()
                .map(|record| record.status.slug())
                .collect::<Vec<_>>(),
            vec!["planned-upstream", "draft-only", "windows-only"]
        );
        assert!(register.records.iter().all(|record| {
            record.local_posture.contains("OneCalc")
                || record.local_posture.contains("design input")
                || record.local_posture.contains("Do not claim")
        }));
    }

    #[test]
    fn pressure_report_summarizes_open_draft_planned_and_windows_only_counts() {
        let register =
            default_extension_upstream_pressure_register(&frozen_extension_host_contract());
        let report = register.report();

        assert_eq!(report.total_records, 3);
        assert_eq!(report.draft_only_records, 1);
        assert_eq!(report.planned_upstream_records, 1);
        assert_eq!(report.windows_only_records, 1);
        assert_eq!(
            report.summary(),
            "3 open / 1 draft / 1 planned / 1 windows-only"
        );
    }
}
