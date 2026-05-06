//! SEAM-OXFML-LOCALE-EXPAND
//!
//! Target: `LocaleProfileId` enum + per-locale data tables (month
//! names, weekday names, separators, currency) must cover at least
//! `de_DE`, `fr_FR`, `es_ES`, `it_IT`, `nl_NL`, `pt_BR`, `ja_JP`,
//! `zh_CN`, `ko_KR`, `ru_RU`. Today OxFunc only ships `EnUs` and
//! `CurrentExcelHost`. Tracked under
//! `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md`.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-LOCALE-EXPAND"]
fn capability_snapshot_enumerates_at_least_three_locales() {
    seam_pending(
        "SEAM-OXFML-LOCALE-EXPAND",
        "CapabilityAndEnvironmentState.locales must enumerate at least 3 locales beyond EnUs",
    );
}
