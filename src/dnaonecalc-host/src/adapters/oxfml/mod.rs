mod bridge;
mod types;

pub use bridge::{
    EditorAnalysisStage, FormulaEditRequest, FormulaEditResult, OxfmlEditorBridge,
    OxfmlEditorBridgeError,
};
pub use types::{
    CompletionProposal, EditorDocument, EditorSyntaxSnapshot, EditorToken,
    FormulaEditReuseSummary, FormulaTextChangeRange, FunctionHelpPacket, LiveDiagnostic,
    LiveDiagnosticSnapshot, SignatureHelpContext,
};
