//! `.dnafml` (and equivalent `.xml`) persistence — slice 1.
//!
//! Schema is the XML Spreadsheet 2003 + `dna:` extension lane defined
//! in [PERSISTENCE_FORMAT_PLAN.md §5](../../../../../docs/PERSISTENCE_FORMAT_PLAN.md):
//!
//!   * `<Worksheet>` carries the formula in cell A1 so Excel double-click
//!     opens it,
//!   * `<dna:Formula>` carries identity, entry, context, and UI prefs that
//!     Excel doesn't represent,
//!   * `<dna:CompareBundle>` siblings (slice 4) accumulate compare-with-Excel
//!     evidence as history.
//!
//! This slice (1) ships the in-memory `Scenario` shape, a hand-rolled XML
//! emitter, and a `roxmltree`-backed reader, with end-to-end round-trip
//! tests. Wiring `Save as…` / `Open…` actions to the breadcrumb dropdown
//! is slice 1b; the compare-bundle merge is slice 4.
//!
//! Internal architectural name: `scenario`. User-facing name: `formula`
//! (per [APP_UX_BRIEF.md §1A](../../../../../docs/APP_UX_BRIEF.md)).

use std::fmt;

use roxmltree::{Document, Node};

const SS_NAMESPACE: &str = "urn:schemas-microsoft-com:office:spreadsheet";
const DNA_NAMESPACE: &str = "urn:dnakode:dnaonecalc:formula:1";

const FORMULA_VERSION: &str = "1";

// ---------------------------------------------------------------------------
// In-memory shape
// ---------------------------------------------------------------------------

/// One persisted formula scenario. The on-disk XML round-trips this
/// struct verbatim — every field maps to either an Excel-native location
/// or a `dna:` extension element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub identity: Identity,
    pub entry: Entry,
    pub context: Context,
    pub ui_preferences: UiPreferences,
}

/// Stable identifying metadata. Timestamps are ISO-8601 UTC strings;
/// the persistence layer does not parse them, just round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
}

/// The formula text plus its entry mode (Formula / Value / Text / Empty).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    pub mode: EntryMode,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryMode {
    Formula,
    Value,
    Text,
    #[default]
    Empty,
}

impl EntryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formula => "Formula",
            Self::Value => "Value",
            Self::Text => "Text",
            Self::Empty => "Empty",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Formula" => Some(Self::Formula),
            "Value" => Some(Self::Value),
            "Text" => Some(Self::Text),
            "Empty" => Some(Self::Empty),
            _ => None,
        }
    }
}

/// Presentation + execution context that determines how the formula
/// renders and how it would be compared with Excel. Mirrors the host
/// state's scenario-context fields plus the publication-context plane
/// per `APP_UX_REALIZATION §5.1`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Context {
    pub host_profile: HostProfile,
    pub locale: Locale,
    pub publication_context: PublicationContext,
    pub scenario_policy: ScenarioPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostProfile {
    pub profile_id: String,
    pub requires_excel_observation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Locale {
    pub id: String,
    pub date1904: bool,
}

/// Authoritative formatting + style + CF context for the cell. Mirrors
/// the upstream `VerificationPublicationContext` shape, kept simple in
/// slice 1 — `style_hierarchy` and `cf_rules` are reserved schema slots
/// and are not yet round-tripped (empty on read; only written when the
/// in-memory state has them populated, which today never happens).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PublicationContext {
    pub format_profile: String,
    pub number_format_code: String,
    pub style_id: String,
    pub font_color: String,
    pub fill_color: String,
    pub style_hierarchy: Vec<String>,
    pub cf_rules: Vec<CfRule>,
}

/// One CF rule. Minimal shape for slice 1; richer fields land alongside
/// `OxFml::publication::VerificationConditionalFormattingRule` mapping
/// in slice 2.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CfRule {
    pub range: String,
    pub formula: Option<String>,
    pub rule_kind: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScenarioPolicy {
    #[default]
    Deterministic,
    LiveRecalc,
}

impl ScenarioPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "Deterministic",
            Self::LiveRecalc => "LiveRecalc",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Deterministic" => Some(Self::Deterministic),
            "LiveRecalc" => Some(Self::LiveRecalc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UiPreferences {
    pub formula_drill_expanded: bool,
    pub result_drill_expanded: bool,
    pub expanded_editor: bool,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaFileError {
    /// XML is not well-formed.
    Parse(String),
    /// XML is well-formed but no recognisable `<dna:Formula>` extension
    /// AND no usable Excel-native fallback (e.g. missing `<Worksheet>` or
    /// no cell). Slice 3 will widen the fallback path.
    NotADnaFormula(String),
    /// `<dna:Formula>` carries a `version` attribute we do not understand.
    /// The caller is expected to surface this honestly to the user rather
    /// than silently downgrade.
    UnsupportedVersion(String),
}

impl fmt::Display for FormulaFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "failed to parse XML: {message}"),
            Self::NotADnaFormula(message) => write!(
                f,
                "file is not a recognisable DnaOneCalc formula: {message}",
            ),
            Self::UnsupportedVersion(version) => write!(
                f,
                "dna:Formula version `{version}` is not supported by this build",
            ),
        }
    }
}

impl std::error::Error for FormulaFileError {}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Serialise a `Scenario` to the `.dnafml` (or `.xml`) byte form.
/// Output is UTF-8 with `\n` line endings; callers responsible for
/// platform newlines if they care. The output begins with the XML
/// processing instruction and the `<?mso-application?>` PI so Excel
/// associates it with the spreadsheet renderer.
///
/// The cell value (`<Data>` text node) is the formula text when the
/// entry is a literal value/text, the empty string for a Formula entry
/// (Excel will recompute the value when it opens the file), and the
/// raw text for the `Empty` entry mode (writes an empty cell).
pub fn write_formula_xml(scenario: &Scenario) -> String {
    let mut out = String::with_capacity(2048);
    out.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    out.push('\n');
    out.push_str(r#"<?mso-application progid="Excel.Sheet"?>"#);
    out.push('\n');
    out.push_str(r#"<Workbook xmlns=""#);
    out.push_str(SS_NAMESPACE);
    out.push_str("\"\n");
    out.push_str(r#"          xmlns:o="urn:schemas-microsoft-com:office:office""#);
    out.push('\n');
    out.push_str(r#"          xmlns:x="urn:schemas-microsoft-com:office:excel""#);
    out.push('\n');
    out.push_str(r#"          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet""#);
    out.push('\n');
    out.push_str(r#"          xmlns:dna=""#);
    out.push_str(DNA_NAMESPACE);
    out.push_str("\">\n");

    write_worksheet(&mut out, scenario);
    write_dna_formula(&mut out, scenario);

    out.push_str("</Workbook>\n");
    out
}

fn write_worksheet(out: &mut String, scenario: &Scenario) {
    out.push_str("  <Worksheet ss:Name=\"Formula\">\n");
    out.push_str("    <Table>\n");
    out.push_str("      <Row>\n");
    write_cell(out, scenario);
    out.push_str("      </Row>\n");
    out.push_str("    </Table>\n");
    out.push_str("  </Worksheet>\n");
}

fn write_cell(out: &mut String, scenario: &Scenario) {
    let raw = &scenario.entry.text;
    match scenario.entry.mode {
        EntryMode::Formula => {
            out.push_str("        <Cell ss:Formula=\"");
            out.push_str(&xml_attr_escape(raw));
            out.push_str("\"><Data ss:Type=\"String\"></Data></Cell>\n");
        }
        EntryMode::Value => {
            // Try to render as a number when the raw text parses; else
            // render as a string Cell. This is what Excel users expect
            // from the canonical "value" entry mode.
            if let Ok(number) = raw.parse::<f64>() {
                if number.is_finite() {
                    out.push_str("        <Cell><Data ss:Type=\"Number\">");
                    out.push_str(&xml_text_escape(raw));
                    out.push_str("</Data></Cell>\n");
                    return;
                }
            }
            out.push_str("        <Cell><Data ss:Type=\"String\">");
            out.push_str(&xml_text_escape(raw));
            out.push_str("</Data></Cell>\n");
        }
        EntryMode::Text => {
            // Forced text via leading apostrophe — drop the apostrophe
            // for the Excel-visible cell since Excel handles that prefix
            // natively.
            let stripped = raw.strip_prefix('\'').unwrap_or(raw);
            out.push_str("        <Cell><Data ss:Type=\"String\">");
            out.push_str(&xml_text_escape(stripped));
            out.push_str("</Data></Cell>\n");
        }
        EntryMode::Empty => {
            out.push_str("        <Cell><Data ss:Type=\"String\"></Data></Cell>\n");
        }
    }
}

fn write_dna_formula(out: &mut String, scenario: &Scenario) {
    out.push_str("  <dna:Formula version=\"");
    out.push_str(FORMULA_VERSION);
    out.push_str("\">\n");

    write_dna_identity(out, &scenario.identity);
    write_dna_entry(out, &scenario.entry);
    write_dna_context(out, &scenario.context);
    write_dna_ui_preferences(out, &scenario.ui_preferences);

    out.push_str("  </dna:Formula>\n");
}

fn write_dna_identity(out: &mut String, identity: &Identity) {
    out.push_str("    <dna:Identity");
    write_attr(out, "id", &identity.id);
    write_attr(out, "name", &identity.name);
    write_attr(out, "created-at", &identity.created_at);
    write_attr(out, "modified-at", &identity.modified_at);
    out.push_str("/>\n");
}

fn write_dna_entry(out: &mut String, entry: &Entry) {
    out.push_str("    <dna:Entry mode=\"");
    out.push_str(entry.mode.as_str());
    out.push_str("\">");
    out.push_str(&xml_text_escape(&entry.text));
    out.push_str("</dna:Entry>\n");
}

fn write_dna_context(out: &mut String, context: &Context) {
    out.push_str("    <dna:Context>\n");
    out.push_str("      <dna:HostProfile");
    write_attr(out, "profile-id", &context.host_profile.profile_id);
    write_attr(
        out,
        "requires-excel-observation",
        bool_attr(context.host_profile.requires_excel_observation),
    );
    out.push_str("/>\n");

    out.push_str("      <dna:Locale");
    write_attr(out, "id", &context.locale.id);
    write_attr(out, "date1904", bool_attr(context.locale.date1904));
    out.push_str("/>\n");

    write_dna_publication_context(out, &context.publication_context);

    out.push_str("      <dna:ScenarioPolicy>");
    out.push_str(context.scenario_policy.as_str());
    out.push_str("</dna:ScenarioPolicy>\n");
    out.push_str("    </dna:Context>\n");
}

fn write_dna_publication_context(out: &mut String, pc: &PublicationContext) {
    out.push_str("      <dna:PublicationContext");
    write_attr(out, "format-profile", &pc.format_profile);
    write_attr(out, "number-format-code", &pc.number_format_code);
    write_attr(out, "style-id", &pc.style_id);
    write_attr(out, "font-color", &pc.font_color);
    write_attr(out, "fill-color", &pc.fill_color);
    out.push_str(">\n");

    out.push_str("        <dna:StyleHierarchy>\n");
    for level in &pc.style_hierarchy {
        out.push_str("          <dna:StyleLevel");
        write_attr(out, "id", level);
        out.push_str("/>\n");
    }
    out.push_str("        </dna:StyleHierarchy>\n");

    out.push_str("        <dna:CfRules>\n");
    for rule in &pc.cf_rules {
        out.push_str("          <dna:CfRule");
        write_attr(out, "range", &rule.range);
        if let Some(formula) = rule.formula.as_deref() {
            write_attr(out, "formula", formula);
        }
        if let Some(rule_kind) = rule.rule_kind.as_deref() {
            write_attr(out, "rule-kind", rule_kind);
        }
        out.push_str("/>\n");
    }
    out.push_str("        </dna:CfRules>\n");
    out.push_str("      </dna:PublicationContext>\n");
}

fn write_dna_ui_preferences(out: &mut String, prefs: &UiPreferences) {
    out.push_str("    <dna:UiPreferences");
    write_attr(
        out,
        "formula-drill-expanded",
        bool_attr(prefs.formula_drill_expanded),
    );
    write_attr(
        out,
        "result-drill-expanded",
        bool_attr(prefs.result_drill_expanded),
    );
    write_attr(out, "expanded-editor", bool_attr(prefs.expanded_editor));
    out.push_str("/>\n");
}

fn write_attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    out.push_str(&xml_attr_escape(value));
    out.push('"');
}

fn bool_attr(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// Escape an attribute value. Replaces the five XML metacharacters
/// (`& < > " '`) plus all control characters except tab/CR/LF.
fn xml_attr_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(ch),
            ch if (ch as u32) < 0x20 => {
                // Drop other control characters; XML 1.0 forbids them.
            }
            ch => out.push(ch),
        }
    }
    out
}

/// Escape a text-node value. Same rules as attribute except `'` and
/// `"` don't strictly need escaping inside text — but we keep parity
/// for safety (no harm; readability unchanged).
fn xml_text_escape(value: &str) -> String {
    xml_attr_escape(value)
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Parse a `.dnafml` (or `.xml`) byte form back into a `Scenario`.
///
/// The reader prefers the `<dna:Formula>` extension when present (full
/// fidelity). When absent — e.g. file was saved by Excel after a round
/// trip — the parser falls back to the `<Worksheet>` cell text and
/// fills sensible defaults for everything else, surfacing a
/// `FormulaFileError::NotADnaFormula` only when the file isn't even
/// recognisable as SpreadsheetML 2003. (Slice 3 will replace the
/// hard error with a soft "imported from Excel-only file" load
/// diagnostic; today the partial-fallback path is on but the warning
/// channel is not yet plumbed.)
pub fn read_formula_xml(xml: &str) -> Result<Scenario, FormulaFileError> {
    let document =
        Document::parse(xml).map_err(|error| FormulaFileError::Parse(error.to_string()))?;

    let workbook = document.root_element();
    if workbook.tag_name().name() != "Workbook" {
        return Err(FormulaFileError::NotADnaFormula(format!(
            "root element is `{}`, expected `Workbook`",
            workbook.tag_name().name(),
        )));
    }

    let dna_formula = find_child_in_namespace(workbook, DNA_NAMESPACE, "Formula");

    if let Some(dna_formula) = dna_formula {
        return read_with_dna_formula(workbook, dna_formula);
    }

    // Excel-only fallback: pull from the worksheet cell.
    read_excel_only(workbook)
}

fn read_with_dna_formula(
    workbook: Node<'_, '_>,
    dna_formula: Node<'_, '_>,
) -> Result<Scenario, FormulaFileError> {
    let version = dna_formula
        .attribute("version")
        .unwrap_or(FORMULA_VERSION)
        .to_string();
    if version != FORMULA_VERSION {
        return Err(FormulaFileError::UnsupportedVersion(version));
    }

    let identity = read_identity(dna_formula);
    let entry = read_entry(dna_formula).unwrap_or_else(|| read_excel_entry(workbook));
    let context = read_context(dna_formula).unwrap_or_default();
    let ui_preferences = read_ui_preferences(dna_formula);

    Ok(Scenario {
        identity,
        entry,
        context,
        ui_preferences,
    })
}

fn read_excel_only(workbook: Node<'_, '_>) -> Result<Scenario, FormulaFileError> {
    let entry = read_excel_entry(workbook);
    Ok(Scenario {
        identity: Identity::default(),
        entry,
        context: Context::default(),
        ui_preferences: UiPreferences::default(),
    })
}

fn read_identity(dna_formula: Node<'_, '_>) -> Identity {
    let identity_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Identity");
    let Some(identity_node) = identity_node else {
        return Identity::default();
    };
    Identity {
        id: identity_node.attribute("id").unwrap_or_default().to_string(),
        name: identity_node
            .attribute("name")
            .unwrap_or_default()
            .to_string(),
        created_at: identity_node
            .attribute("created-at")
            .unwrap_or_default()
            .to_string(),
        modified_at: identity_node
            .attribute("modified-at")
            .unwrap_or_default()
            .to_string(),
    }
}

fn read_entry(dna_formula: Node<'_, '_>) -> Option<Entry> {
    let entry_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Entry")?;
    let mode = entry_node
        .attribute("mode")
        .and_then(EntryMode::parse)
        .unwrap_or_default();
    let text = entry_node.text().unwrap_or("").to_string();
    Some(Entry { mode, text })
}

fn read_excel_entry(workbook: Node<'_, '_>) -> Entry {
    let cell = find_first_cell(workbook);
    let Some(cell) = cell else {
        return Entry::default();
    };
    if let Some(formula) = cell_attribute_in_namespace(cell, SS_NAMESPACE, "Formula") {
        return Entry {
            mode: EntryMode::Formula,
            text: formula.to_string(),
        };
    }
    let data = cell
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "Data");
    let Some(data) = data else {
        return Entry::default();
    };
    let data_text = data.text().unwrap_or("").to_string();
    let data_type =
        cell_attribute_in_namespace(data, SS_NAMESPACE, "Type").unwrap_or("String");
    let mode = match data_type {
        _ if data_text.is_empty() => EntryMode::Empty,
        "Number" => EntryMode::Value,
        "Boolean" => EntryMode::Value,
        _ => EntryMode::Text,
    };
    Entry {
        mode,
        text: data_text,
    }
}

fn find_first_cell<'a>(workbook: Node<'a, '_>) -> Option<Node<'a, 'a>> {
    workbook
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Cell")
}

fn read_context(dna_formula: Node<'_, '_>) -> Option<Context> {
    let context_node = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "Context")?;
    let host_profile = read_host_profile(context_node);
    let locale = read_locale(context_node);
    let publication_context = read_publication_context(context_node);
    let scenario_policy = find_child_in_namespace(context_node, DNA_NAMESPACE, "ScenarioPolicy")
        .and_then(|node| node.text())
        .and_then(ScenarioPolicy::parse)
        .unwrap_or_default();
    Some(Context {
        host_profile,
        locale,
        publication_context,
        scenario_policy,
    })
}

fn read_host_profile(context_node: Node<'_, '_>) -> HostProfile {
    let Some(node) = find_child_in_namespace(context_node, DNA_NAMESPACE, "HostProfile") else {
        return HostProfile::default();
    };
    HostProfile {
        profile_id: node.attribute("profile-id").unwrap_or_default().to_string(),
        requires_excel_observation: parse_bool_attr(node, "requires-excel-observation"),
    }
}

fn read_locale(context_node: Node<'_, '_>) -> Locale {
    let Some(node) = find_child_in_namespace(context_node, DNA_NAMESPACE, "Locale") else {
        return Locale::default();
    };
    Locale {
        id: node.attribute("id").unwrap_or_default().to_string(),
        date1904: parse_bool_attr(node, "date1904"),
    }
}

fn read_publication_context(context_node: Node<'_, '_>) -> PublicationContext {
    let Some(node) =
        find_child_in_namespace(context_node, DNA_NAMESPACE, "PublicationContext")
    else {
        return PublicationContext::default();
    };
    let style_hierarchy =
        find_child_in_namespace(node, DNA_NAMESPACE, "StyleHierarchy")
            .map(|hierarchy| {
                hierarchy
                    .children()
                    .filter(|child| {
                        child.is_element()
                            && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                            && child.tag_name().name() == "StyleLevel"
                    })
                    .map(|level| level.attribute("id").unwrap_or_default().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    let cf_rules = find_child_in_namespace(node, DNA_NAMESPACE, "CfRules")
        .map(|rules| {
            rules
                .children()
                .filter(|child| {
                    child.is_element()
                        && child.tag_name().namespace() == Some(DNA_NAMESPACE)
                        && child.tag_name().name() == "CfRule"
                })
                .map(|rule| CfRule {
                    range: rule.attribute("range").unwrap_or_default().to_string(),
                    formula: rule.attribute("formula").map(ToOwned::to_owned),
                    rule_kind: rule.attribute("rule-kind").map(ToOwned::to_owned),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    PublicationContext {
        format_profile: node.attribute("format-profile").unwrap_or_default().to_string(),
        number_format_code: node
            .attribute("number-format-code")
            .unwrap_or_default()
            .to_string(),
        style_id: node.attribute("style-id").unwrap_or_default().to_string(),
        font_color: node.attribute("font-color").unwrap_or_default().to_string(),
        fill_color: node.attribute("fill-color").unwrap_or_default().to_string(),
        style_hierarchy,
        cf_rules,
    }
}

fn read_ui_preferences(dna_formula: Node<'_, '_>) -> UiPreferences {
    let Some(node) = find_child_in_namespace(dna_formula, DNA_NAMESPACE, "UiPreferences") else {
        return UiPreferences::default();
    };
    UiPreferences {
        formula_drill_expanded: parse_bool_attr(node, "formula-drill-expanded"),
        result_drill_expanded: parse_bool_attr(node, "result-drill-expanded"),
        expanded_editor: parse_bool_attr(node, "expanded-editor"),
    }
}

fn parse_bool_attr(node: Node<'_, '_>, attr: &str) -> bool {
    matches!(
        node.attribute(attr).map(str::to_ascii_lowercase).as_deref(),
        Some("true" | "1" | "yes")
    )
}

fn find_child_in_namespace<'a>(
    parent: Node<'a, '_>,
    namespace: &str,
    local_name: &str,
) -> Option<Node<'a, 'a>> {
    parent.children().find(|child| {
        child.is_element()
            && child.tag_name().namespace() == Some(namespace)
            && child.tag_name().name() == local_name
    })
}

fn cell_attribute_in_namespace<'a>(
    node: Node<'a, '_>,
    namespace: &str,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes().find_map(|attr| {
        if attr.namespace() == Some(namespace) && attr.name() == local_name {
            Some(attr.value())
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_full_scenario() -> Scenario {
        Scenario {
            identity: Identity {
                id: "invoice-eu-tax".to_string(),
                name: "invoice-eu-tax".to_string(),
                created_at: "2026-04-22T10:14:22Z".to_string(),
                modified_at: "2026-04-26T14:22:01Z".to_string(),
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=SUM(1,2,3)".to_string(),
            },
            context: Context {
                host_profile: HostProfile {
                    profile_id: "Excel365Win".to_string(),
                    requires_excel_observation: true,
                },
                locale: Locale {
                    id: "EnUs".to_string(),
                    date1904: false,
                },
                publication_context: PublicationContext {
                    format_profile: String::new(),
                    number_format_code: "€ #,##0.00".to_string(),
                    style_id: String::new(),
                    font_color: String::new(),
                    fill_color: String::new(),
                    style_hierarchy: vec!["base".to_string(), "currency".to_string()],
                    cf_rules: vec![CfRule {
                        range: "A1".to_string(),
                        formula: Some("=A1>0".to_string()),
                        rule_kind: Some("CellIs".to_string()),
                    }],
                },
                scenario_policy: ScenarioPolicy::Deterministic,
            },
            ui_preferences: UiPreferences {
                formula_drill_expanded: false,
                result_drill_expanded: true,
                expanded_editor: false,
            },
        }
    }

    fn round_trip(scenario: &Scenario) -> Scenario {
        let xml = write_formula_xml(scenario);
        read_formula_xml(&xml).expect("round-trip parse must succeed")
    }

    #[test]
    fn full_scenario_round_trips_verbatim() {
        let scenario = sample_full_scenario();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn empty_scenario_round_trips() {
        let scenario = Scenario {
            identity: Identity::default(),
            entry: Entry::default(),
            context: Context::default(),
            ui_preferences: UiPreferences::default(),
        };
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn formula_text_with_all_xml_metacharacters_round_trips() {
        let mut scenario = sample_full_scenario();
        scenario.entry.text = r#"=IF(A1<5, "<&>'\"", "OK")"#.to_string();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn unicode_formula_text_round_trips() {
        let mut scenario = sample_full_scenario();
        scenario.entry.text = "=\"日本語と数式 → 結果\"".to_string();
        let restored = round_trip(&scenario);
        assert_eq!(restored, scenario);
    }

    #[test]
    fn value_entry_with_numeric_text_writes_number_data_type() {
        let scenario = Scenario {
            entry: Entry {
                mode: EntryMode::Value,
                text: "12345.67".to_string(),
            },
            ..Scenario {
                identity: Identity::default(),
                entry: Entry::default(),
                context: Context::default(),
                ui_preferences: UiPreferences::default(),
            }
        };
        let xml = write_formula_xml(&scenario);
        assert!(
            xml.contains(r#"<Data ss:Type="Number">12345.67</Data>"#),
            "expected Number-typed Data; got xml:\n{xml}",
        );
        let restored = read_formula_xml(&xml).expect("round-trip");
        assert_eq!(restored.entry.text, "12345.67");
        assert_eq!(restored.entry.mode, EntryMode::Value);
    }

    #[test]
    fn text_entry_strips_apostrophe_in_excel_cell_but_preserves_in_dna_entry() {
        let scenario = Scenario {
            entry: Entry {
                mode: EntryMode::Text,
                text: "'42".to_string(),
            },
            ..Scenario {
                identity: Identity::default(),
                entry: Entry::default(),
                context: Context::default(),
                ui_preferences: UiPreferences::default(),
            }
        };
        let xml = write_formula_xml(&scenario);
        // Excel cell sees `42` (no leading apostrophe).
        assert!(
            xml.contains(r#"<Data ss:Type="String">42</Data>"#),
            "expected leading apostrophe stripped from cell; got xml:\n{xml}",
        );
        // dna:Entry preserves the raw `'42`.
        assert!(
            xml.contains("<dna:Entry mode=\"Text\">&apos;42</dna:Entry>"),
            "expected dna:Entry to preserve the raw '42; got xml:\n{xml}",
        );
        let restored = read_formula_xml(&xml).expect("round-trip");
        assert_eq!(restored.entry.text, "'42");
        assert_eq!(restored.entry.mode, EntryMode::Text);
    }

    #[test]
    fn dna_formula_wins_when_excel_cell_diverges() {
        // Excel-side cell reads `=ABS(1)`; dna:Entry reads `=SUM(1,2)`.
        // Per §5.3 of the plan the dna: branch wins for fields Excel
        // could not have edited; for the formula text both can plausibly
        // edit it, so we accept whichever the writer produces. This test
        // pins the current behaviour: dna:Entry wins on read.
        let xml = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="{dna}">
  <Worksheet ss:Name="Formula">
    <Table>
      <Row>
        <Cell ss:Formula="=ABS(1)"><Data ss:Type="String"></Data></Cell>
      </Row>
    </Table>
  </Worksheet>
  <dna:Formula version="1">
    <dna:Entry mode="Formula">=SUM(1,2)</dna:Entry>
  </dna:Formula>
</Workbook>
"#,
            dna = DNA_NAMESPACE
        );
        let scenario = read_formula_xml(&xml).expect("parse");
        assert_eq!(scenario.entry.text, "=SUM(1,2)");
    }

    #[test]
    fn excel_only_fallback_when_dna_formula_absent_pulls_cell_into_entry() {
        let xml = r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet">
  <Worksheet ss:Name="Sheet1">
    <Table>
      <Row>
        <Cell ss:Formula="=SUM(7,8)"><Data ss:Type="String"></Data></Cell>
      </Row>
    </Table>
  </Worksheet>
</Workbook>
"#;
        let scenario = read_formula_xml(xml).expect("parse");
        assert_eq!(scenario.entry.text, "=SUM(7,8)");
        assert_eq!(scenario.entry.mode, EntryMode::Formula);
        // Identity / context / ui-prefs default since dna: extension was absent.
        assert_eq!(scenario.identity, Identity::default());
        assert_eq!(scenario.context, Context::default());
        assert_eq!(scenario.ui_preferences, UiPreferences::default());
    }

    #[test]
    fn unknown_dna_formula_version_returns_unsupported_error() {
        let xml = format!(
            r#"<?xml version="1.0"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet"
          xmlns:dna="{dna}">
  <dna:Formula version="9999">
    <dna:Entry mode="Formula">=A1</dna:Entry>
  </dna:Formula>
</Workbook>
"#,
            dna = DNA_NAMESPACE
        );
        let result = read_formula_xml(&xml);
        match result {
            Err(FormulaFileError::UnsupportedVersion(version)) => assert_eq!(version, "9999"),
            other => panic!("expected UnsupportedVersion(9999), got {other:?}"),
        }
    }

    #[test]
    fn malformed_xml_returns_parse_error() {
        let result = read_formula_xml("<not-well-formed");
        assert!(matches!(result, Err(FormulaFileError::Parse(_))));
    }

    #[test]
    fn non_workbook_root_returns_not_a_dna_formula_error() {
        let result = read_formula_xml(r#"<root xmlns="x"></root>"#);
        match result {
            Err(FormulaFileError::NotADnaFormula(message)) => {
                assert!(message.contains("root"), "got message: {message}");
            }
            other => panic!("expected NotADnaFormula, got {other:?}"),
        }
    }

    #[test]
    fn output_starts_with_xml_declaration_and_mso_application_pi() {
        let scenario = sample_full_scenario();
        let xml = write_formula_xml(&scenario);
        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="utf-8"?>"#));
        assert!(xml.contains(r#"<?mso-application progid="Excel.Sheet"?>"#));
    }

    #[test]
    fn output_contains_dna_namespace_declaration_at_workbook_root() {
        let xml = write_formula_xml(&sample_full_scenario());
        assert!(xml.contains(r#"xmlns:dna="urn:dnakode:dnaonecalc:formula:1""#));
    }

    #[test]
    fn round_trip_with_publication_context_style_hierarchy_and_cf_rules() {
        let scenario = sample_full_scenario();
        let restored = round_trip(&scenario);
        assert_eq!(
            restored.context.publication_context.style_hierarchy,
            scenario.context.publication_context.style_hierarchy,
        );
        assert_eq!(
            restored.context.publication_context.cf_rules,
            scenario.context.publication_context.cf_rules,
        );
    }

    #[test]
    fn round_trip_with_live_recalc_scenario_policy() {
        let mut scenario = sample_full_scenario();
        scenario.context.scenario_policy = ScenarioPolicy::LiveRecalc;
        let restored = round_trip(&scenario);
        assert_eq!(restored.context.scenario_policy, ScenarioPolicy::LiveRecalc);
    }

    #[test]
    fn entry_mode_round_trips_for_all_four_variants() {
        for &mode in &[
            EntryMode::Formula,
            EntryMode::Value,
            EntryMode::Text,
            EntryMode::Empty,
        ] {
            let scenario = Scenario {
                entry: Entry {
                    mode,
                    text: match mode {
                        EntryMode::Formula => "=1+1".to_string(),
                        EntryMode::Value => "42".to_string(),
                        EntryMode::Text => "'hello".to_string(),
                        EntryMode::Empty => String::new(),
                    },
                },
                ..Scenario {
                    identity: Identity::default(),
                    entry: Entry::default(),
                    context: Context::default(),
                    ui_preferences: UiPreferences::default(),
                }
            };
            let restored = round_trip(&scenario);
            assert_eq!(restored.entry.mode, mode, "mode {mode:?}");
            assert_eq!(restored.entry.text, scenario.entry.text, "text for {mode:?}");
        }
    }
}
