use crate::adapters::oxfml::EditorSyntaxSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRun {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
}

pub fn syntax_runs_from_snapshot(snapshot: &EditorSyntaxSnapshot) -> Vec<SyntaxRun> {
    snapshot
        .tokens
        .iter()
        .map(|token| SyntaxRun {
            text: token.text.clone(),
            span_start: token.span_start,
            span_len: token.span_len,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{EditorSyntaxSnapshot, EditorToken};

    #[test]
    fn syntax_runs_follow_snapshot_tokens() {
        let snapshot = EditorSyntaxSnapshot {
            formula_stable_id: "formula-1".to_string(),
            green_tree_key: "green-1".to_string(),
            tokens: vec![
                EditorToken {
                    text: "=".to_string(),
                    span_start: 0,
                    span_len: 1,
                },
                EditorToken {
                    text: "SUM".to_string(),
                    span_start: 1,
                    span_len: 3,
                },
            ],
        };

        let runs = syntax_runs_from_snapshot(&snapshot);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].text, "SUM");
        assert_eq!(runs[1].span_start, 1);
    }
}
