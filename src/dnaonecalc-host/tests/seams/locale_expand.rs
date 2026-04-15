//! SEAM-OXFUNC-LOCALE-EXPAND
//!
//! Target: `LocaleProfileId` enum + `format_profile()` table must cover
//! at least `de_DE`, `fr_FR`, `es_ES`, `it_IT`, `nl_NL`, `pt_BR`, `ja_JP`,
//! `zh_CN`, `ko_KR`, `ru_RU`. Today OxFunc only ships `EnUs` and
//! `CurrentExcelHost`.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFUNC-LOCALE-EXPAND"]
fn capability_snapshot_enumerates_at_least_three_locales() {
    seam_pending(
        "SEAM-OXFUNC-LOCALE-EXPAND",
        "CapabilityAndEnvironmentState.locales must enumerate at least 3 locales beyond EnUs",
    );
}
