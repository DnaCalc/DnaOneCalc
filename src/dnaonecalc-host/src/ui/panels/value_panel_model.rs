//! OneCalc-local model for the Value Panel component.
//!
//! These types mirror the shape of `oxfunc_value_types::ExtendedValue` /
//! `EvalValue` / `PresentationHint` / `RichValue` / etc., but live entirely
//! inside `dnaonecalc-host` so the UI layer does not depend on the
//! full engine type graph. An adapter from the real engine types can be
//! added later once `ExtendedValue` is routed through `FormulaSpaceState`.
//!
//! Every engine surface that is not yet implemented is marked in the pipeline
//! with a `SEAM-*` id so the gap is visible to users and engineers.

use crate::ui::editor::state::EditorLiveState;

#[derive(Debug, Clone, PartialEq)]
pub enum ValuePanelValue {
    Number {
        raw: f64,
        /// Optional extra rows to surface under the main number (scientific,
        /// hex bits, nearest Excel integer, etc.). Left as free-form rows so
        /// the builder can populate whichever facts it has today.
        detail_rows: Vec<ValuePanelKeyValue>,
    },
    Text {
        content: String,
        utf16_code_units: usize,
        utf16_code_unit_limit: usize,
        has_dangling_surrogate: bool,
    },
    Logical(bool),
    Error {
        code_label: String,
        code_numeric: Option<u16>,
        surface: Option<ValuePanelErrorSurface>,
        detail: Option<String>,
    },
    Array {
        shape: ValuePanelArrayShape,
        rows: Vec<Vec<ValuePanelArrayCell>>,
        truncated: bool,
    },
    Reference {
        kind: ValuePanelReferenceKind,
        target: String,
        multi_area_targets: Vec<String>,
        normalized_differs: bool,
    },
    Lambda {
        callable_token: String,
        origin_kind: String,
        arity_min: usize,
        arity_max: Option<usize>,
        capture_mode: String,
        invocation_contract_ref: String,
    },
    RichValue(Box<ValuePanelRichValue>),
    /// The formula space has no proved result yet. The panel still renders
    /// effective display and presentation sections if available.
    Unevaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePanelKeyValue {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePanelErrorSurface {
    Worksheet,
    XllTransferable,
    ExtendedWorksheetOnly,
}

impl ValuePanelErrorSurface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Worksheet => "Worksheet",
            Self::XllTransferable => "XLL transferable",
            Self::ExtendedWorksheetOnly => "Extended worksheet only",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Worksheet => "worksheet",
            Self::XllTransferable => "xll-transferable",
            Self::ExtendedWorksheetOnly => "extended-worksheet-only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValuePanelArrayShape {
    pub rows: usize,
    pub cols: usize,
}

impl ValuePanelArrayShape {
    pub fn cell_count(self) -> usize {
        self.rows.saturating_mul(self.cols)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValuePanelArrayCell {
    Number(f64),
    Text(String),
    Logical(bool),
    Error(String),
    EmptyCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePanelReferenceKind {
    A1,
    Area,
    MultiArea,
    ThreeD,
    Structured,
    SpillAnchor,
}

impl ValuePanelReferenceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::A1 => "A1",
            Self::Area => "Area",
            Self::MultiArea => "Multi-area",
            Self::ThreeD => "3-D",
            Self::Structured => "Structured",
            Self::SpillAnchor => "Spill anchor",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::A1 => "a1",
            Self::Area => "area",
            Self::MultiArea => "multi-area",
            Self::ThreeD => "three-d",
            Self::Structured => "structured",
            Self::SpillAnchor => "spill-anchor",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuePanelRichValue {
    pub type_name: String,
    pub fallback: ValuePanelRichValueData,
    pub key_value_pairs: Vec<ValuePanelRichKvp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValuePanelRichValueData {
    Number(f64),
    Text(String),
    Logical(bool),
    Error(String),
    EmptyCell,
    Array {
        shape: ValuePanelArrayShape,
        rows: Vec<Vec<ValuePanelRichValueData>>,
    },
    Nested(Box<ValuePanelRichValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuePanelRichKvp {
    pub key: String,
    pub value: ValuePanelRichValueData,
    pub exclude_from_calc_comparison: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePanelPresentation {
    pub number_format_hint: Option<ValuePanelNumberFormatHint>,
    pub style_hint: Option<ValuePanelStyleHint>,
    pub format_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePanelNumberFormatHint {
    General,
    DateLike,
    Percentage,
    Currency,
    Scientific,
    Fraction,
    Custom,
}

impl ValuePanelNumberFormatHint {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::DateLike => "Date-like",
            Self::Percentage => "Percentage",
            Self::Currency => "Currency",
            Self::Scientific => "Scientific",
            Self::Fraction => "Fraction",
            Self::Custom => "Custom",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::DateLike => "date-like",
            Self::Percentage => "percentage",
            Self::Currency => "currency",
            Self::Scientific => "scientific",
            Self::Fraction => "fraction",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuePanelStyleHint {
    Hyperlink,
}

impl ValuePanelStyleHint {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hyperlink => "Hyperlink",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Hyperlink => "hyperlink",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePanelPipelineStep {
    pub label: String,
    pub value_preview: Option<String>,
    pub status: ValuePanelPipelineStepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuePanelPipelineStepStatus {
    Live,
    NotImplemented { seam_id: String },
}

impl ValuePanelPipelineStepStatus {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::NotImplemented { .. } => "not-implemented",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePanelProvenance {
    pub green_tree_key: Option<String>,
    pub walk_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuePanelViewModel {
    pub value: ValuePanelValue,
    pub presentation: Option<ValuePanelPresentation>,
    pub effective_display_text: Option<String>,
    pub display_pipeline: Vec<ValuePanelPipelineStep>,
    pub provenance: Option<ValuePanelProvenance>,
    pub live_state: EditorLiveState,
}

impl ValuePanelValue {
    /// Stable header slug for CSS/data-attribute selection.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Number { .. } => "number",
            Self::Text { .. } => "text",
            Self::Logical(_) => "logical",
            Self::Error { .. } => "error",
            Self::Array { .. } => "array",
            Self::Reference { .. } => "reference",
            Self::Lambda { .. } => "lambda",
            Self::RichValue(_) => "rich-value",
            Self::Unevaluated => "unevaluated",
        }
    }

    /// Header label shown to the user.
    pub fn header_label(&self) -> String {
        match self {
            Self::Number { .. } => "Number".to_string(),
            Self::Text { .. } => "Text".to_string(),
            Self::Logical(_) => "Logical".to_string(),
            Self::Error { code_label, .. } => format!("Error ({code_label})"),
            Self::Array { shape, .. } => format!("Array [{} × {}]", shape.rows, shape.cols),
            Self::Reference { kind, .. } => format!("Reference ({})", kind.label()),
            Self::Lambda { .. } => "Lambda".to_string(),
            Self::RichValue(rich) => format!("Rich value ({})", rich.type_name),
            Self::Unevaluated => "Unevaluated".to_string(),
        }
    }
}

/// Construct a minimal ValuePanel view model from the legacy string-only
/// Explore projection. Until `ExtendedValue` is routed through the formula
/// space (see `SEAM-ONECALC-EXTENDED-VALUE-ROUTING`) this is what the Explore
/// result hero can render, and it already covers the common case of a scalar
/// display + effective-display text.
pub fn build_value_panel_from_explore_strings(
    result_value_summary: Option<&str>,
    effective_display_summary: Option<&str>,
    green_tree_key: Option<&str>,
    live_state: EditorLiveState,
) -> ValuePanelViewModel {
    let value = classify_result_summary(result_value_summary);
    let mut display_pipeline = Vec::new();
    display_pipeline.push(ValuePanelPipelineStep {
        label: "raw".to_string(),
        value_preview: Some(value.header_label()),
        status: ValuePanelPipelineStepStatus::Live,
    });
    display_pipeline.push(ValuePanelPipelineStep {
        label: "format code".to_string(),
        value_preview: None,
        status: ValuePanelPipelineStepStatus::NotImplemented {
            seam_id: "SEAM-ONECALC-EXTENDED-VALUE-ROUTING".to_string(),
        },
    });
    display_pipeline.push(ValuePanelPipelineStep {
        label: "locale".to_string(),
        value_preview: None,
        status: ValuePanelPipelineStepStatus::NotImplemented {
            seam_id: "SEAM-OXFUNC-LOCALE-EXPAND".to_string(),
        },
    });
    display_pipeline.push(ValuePanelPipelineStep {
        label: "CF override".to_string(),
        value_preview: None,
        status: ValuePanelPipelineStepStatus::NotImplemented {
            seam_id: "SEAM-OXFML-CF-COLORSCALE".to_string(),
        },
    });
    display_pipeline.push(ValuePanelPipelineStep {
        label: "effective".to_string(),
        value_preview: effective_display_summary.map(|s| s.to_string()),
        status: if effective_display_summary.is_some() {
            ValuePanelPipelineStepStatus::Live
        } else {
            ValuePanelPipelineStepStatus::NotImplemented {
                seam_id: "SEAM-ONECALC-EXTENDED-VALUE-ROUTING".to_string(),
            }
        },
    });

    ValuePanelViewModel {
        value,
        presentation: None,
        effective_display_text: effective_display_summary.map(|s| s.to_string()),
        display_pipeline,
        provenance: green_tree_key.map(|key| ValuePanelProvenance {
            green_tree_key: Some(key.to_string()),
            walk_node_id: None,
        }),
        live_state,
    }
}

fn classify_result_summary(summary: Option<&str>) -> ValuePanelValue {
    let Some(text) = summary else {
        return ValuePanelValue::Unevaluated;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ValuePanelValue::Unevaluated;
    }

    let lowered = trimmed.to_ascii_lowercase();
    if lowered == "number" || lowered.starts_with("number ") || lowered.starts_with("number·") {
        return ValuePanelValue::Number {
            raw: f64::NAN,
            detail_rows: vec![ValuePanelKeyValue {
                label: "summary".to_string(),
                value: trimmed.to_string(),
            }],
        };
    }
    if lowered == "text" || lowered.starts_with("text ") {
        return ValuePanelValue::Text {
            content: trimmed.to_string(),
            utf16_code_units: trimmed.encode_utf16().count(),
            utf16_code_unit_limit: 32_767,
            has_dangling_surrogate: false,
        };
    }
    if lowered == "logical" || lowered == "true" || lowered == "false" {
        return ValuePanelValue::Logical(lowered == "true");
    }
    if lowered == "array" || lowered.starts_with("array") {
        return ValuePanelValue::Array {
            shape: ValuePanelArrayShape { rows: 0, cols: 0 },
            rows: Vec::new(),
            truncated: true,
        };
    }
    if lowered.starts_with("error") || trimmed.starts_with('#') {
        return ValuePanelValue::Error {
            code_label: trimmed.to_string(),
            code_numeric: None,
            surface: None,
            detail: None,
        };
    }
    // Fallback: treat as a raw textual payload.
    ValuePanelValue::Text {
        content: trimmed.to_string(),
        utf16_code_units: trimmed.encode_utf16().count(),
        utf16_code_unit_limit: 32_767,
        has_dangling_surrogate: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_strings_classifies_number_summary() {
        let panel = build_value_panel_from_explore_strings(
            Some("Number"),
            Some("3"),
            Some("green-1"),
            EditorLiveState::EditingLive,
        );
        assert!(matches!(panel.value, ValuePanelValue::Number { .. }));
        assert_eq!(panel.effective_display_text.as_deref(), Some("3"));
        assert_eq!(panel.display_pipeline.len(), 5);
        assert_eq!(
            panel
                .provenance
                .as_ref()
                .and_then(|p| p.green_tree_key.clone()),
            Some("green-1".to_string())
        );
    }

    #[test]
    fn build_from_strings_classifies_error_summary() {
        let panel = build_value_panel_from_explore_strings(
            Some("#DIV/0!"),
            None,
            None,
            EditorLiveState::Idle,
        );
        assert!(matches!(panel.value, ValuePanelValue::Error { .. }));
        assert!(panel.effective_display_text.is_none());
        // The final pipeline step must be marked not-implemented when the
        // effective display is unavailable.
        let last = panel.display_pipeline.last().expect("effective step");
        assert_eq!(last.label, "effective");
        assert!(matches!(
            last.status,
            ValuePanelPipelineStepStatus::NotImplemented { .. }
        ));
    }

    #[test]
    fn build_from_strings_falls_back_to_unevaluated_when_empty() {
        let panel = build_value_panel_from_explore_strings(None, None, None, EditorLiveState::Idle);
        assert!(matches!(panel.value, ValuePanelValue::Unevaluated));
        assert_eq!(panel.value.slug(), "unevaluated");
        assert_eq!(panel.value.header_label(), "Unevaluated");
    }

    #[test]
    fn array_shape_cell_count_is_saturating() {
        let shape = ValuePanelArrayShape {
            rows: usize::MAX,
            cols: 2,
        };
        assert_eq!(shape.cell_count(), usize::MAX);
    }

    #[test]
    fn reference_kind_labels_are_stable() {
        assert_eq!(ValuePanelReferenceKind::MultiArea.label(), "Multi-area");
        assert_eq!(ValuePanelReferenceKind::MultiArea.slug(), "multi-area");
    }
}
