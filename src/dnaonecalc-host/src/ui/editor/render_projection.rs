use crate::adapters::oxfml::EditorSyntaxSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTokenRole {
    Operator,
    Function,
    Number,
    Delimiter,
    Identifier,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRun {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
    pub role: SyntaxTokenRole,
}

pub fn syntax_runs_from_snapshot(snapshot: &EditorSyntaxSnapshot) -> Vec<SyntaxRun> {
    snapshot
        .tokens
        .iter()
        .map(|token| SyntaxRun {
            text: token.text.clone(),
            span_start: token.span_start,
            span_len: token.span_len,
            role: classify_token_role(&token.text),
        })
        .collect()
}

fn classify_token_role(text: &str) -> SyntaxTokenRole {
    if text == "=" {
        SyntaxTokenRole::Operator
    } else if matches!(text, "(" | ")" | ",") {
        SyntaxTokenRole::Delimiter
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        SyntaxTokenRole::Number
    } else if !text.is_empty() && text.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
        SyntaxTokenRole::Function
    } else if !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        SyntaxTokenRole::Identifier
    } else {
        SyntaxTokenRole::Text
    }
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
        assert_eq!(runs[0].role, SyntaxTokenRole::Operator);
        assert_eq!(runs[1].role, SyntaxTokenRole::Function);
    }
}
