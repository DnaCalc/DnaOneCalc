use leptos::prelude::*;

pub const ONECALC_THEME_CSS: &str = r#"
:root {
  --oc-color-bg: #f5efe4;
  --oc-color-surface: #fffaf2;
  --oc-color-panel: #f2eadc;
  --oc-color-ink: #1f2a2c;
  --oc-color-muted: #6b695f;
  --oc-color-border: #d9cfbe;
  --oc-color-accent: #245d5a;
  --oc-color-accent-soft: #d7ebe7;
  --oc-color-warm: #b76545;
  --oc-color-warning: #c58b2f;
  --oc-color-success: #4f7b57;
  --oc-space-1: 0.25rem;
  --oc-space-2: 0.5rem;
  --oc-space-3: 0.75rem;
  --oc-space-4: 1rem;
  --oc-space-5: 1.5rem;
  --oc-radius-panel: 14px;
  --oc-radius-pill: 999px;
  --oc-shadow-panel: 0 6px 18px rgba(31, 42, 44, 0.08);
  --oc-font-ui: \"Segoe UI\", \"Inter\", sans-serif;
  --oc-font-mono: \"Cascadia Code\", \"Consolas\", monospace;
}

.onecalc-app {
  font-family: var(--oc-font-ui);
  color: var(--oc-color-ink);
  background: var(--oc-color-bg);
}

.onecalc-shell-frame {
  display: grid;
  grid-template-columns: 16rem 1fr;
  min-height: 100vh;
  background: linear-gradient(180deg, #f7f1e6 0%, #f2ebdd 100%);
}

.onecalc-shell-frame__rail {
  padding: var(--oc-space-5);
  background: var(--oc-color-panel);
  border-right: 1px solid var(--oc-color-border);
}

.onecalc-shell-frame__content {
  display: grid;
  grid-template-rows: auto 1fr;
}

.onecalc-shell-frame__context-bar {
  display: flex;
  align-items: center;
  gap: var(--oc-space-3);
  padding: var(--oc-space-4) var(--oc-space-5);
  border-bottom: 1px solid var(--oc-color-border);
  background: rgba(255, 250, 242, 0.9);
}

.onecalc-shell-frame__mode-switch {
  display: flex;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__mode-button {
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-pill);
  background: transparent;
  padding: 0.4rem 0.8rem;
  color: var(--oc-color-ink);
}

.onecalc-shell-frame__mode-button--active {
  background: var(--oc-color-accent);
  color: white;
  border-color: var(--oc-color-accent);
}

.onecalc-shell-frame__mode-body {
  padding: var(--oc-space-5);
}

.onecalc-shell-frame__space-list {
  list-style: none;
  padding: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__space-item {
  padding: var(--oc-space-3);
  border-radius: var(--oc-radius-panel);
  border: 1px solid var(--oc-color-border);
  background: var(--oc-color-surface);
}

.onecalc-shell-frame__space-button {
  width: 100%;
  text-align: left;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  padding: 0;
}

.onecalc-shell-frame__space-item--active {
  border-color: var(--oc-color-accent);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-shell-frame__space-pin {
  margin-left: var(--oc-space-2);
  color: var(--oc-color-warm);
  font-size: 0.85rem;
}

.onecalc-explore-shell__body,
.onecalc-inspect-shell__body,
.onecalc-workbench-shell__body {
  display: grid;
  gap: var(--oc-space-4);
}

.onecalc-explore-shell__body {
  grid-template-columns: minmax(0, 1.7fr) minmax(18rem, 1fr);
  align-items: start;
}

.onecalc-inspect-shell__body,
.onecalc-workbench-shell__body {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.onecalc-explore-shell__editor-panel,
.onecalc-explore-shell__result-panel,
.onecalc-explore-shell__help-panel,
.onecalc-inspect-shell__walk-cluster,
.onecalc-inspect-shell__summary-cluster,
.onecalc-inspect-shell__summary-card,
.onecalc-workbench-shell__outcome-card,
.onecalc-workbench-shell__lineage-card,
.onecalc-workbench-shell__actions-card,
.onecalc-workbench-shell__evidence-card,
.onecalc-workbench-shell__catalog-card,
.onecalc-workbench-shell__compare-card,
.onecalc-workbench-shell__replay-card {
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  padding: var(--oc-space-4);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-explore-shell__panel-header,
.onecalc-explore-shell__context-strip,
.onecalc-explore-shell__result-metric,
.onecalc-explore-shell__array-preview-header,
.onecalc-inspect-shell__meta,
.onecalc-workbench-shell__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  align-items: center;
}

.onecalc-explore-shell__scenario-label,
.onecalc-explore-shell__truth-chip,
.onecalc-explore-shell__context-strip > span,
.onecalc-inspect-shell__meta > span,
.onecalc-workbench-shell__meta > span,
.onecalc-explore-shell__array-preview-badge {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
  font-size: 0.85rem;
}

.onecalc-explore-shell__truth-chip,
.onecalc-explore-shell__array-preview-badge {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-explore-shell__trace-summary,
.onecalc-explore-shell__blocked-reason,
.onecalc-inspect-shell__context-card,
.onecalc-workbench-shell__import-surface,
.onecalc-workbench-shell__catalog-card,
.onecalc-workbench-shell__evidence-card {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__blocked-reason {
  color: #8a5f19;
  background: #f4e4c6;
  border: 1px solid #e1c48c;
  border-radius: 12px;
  padding: var(--oc-space-2) var(--oc-space-3);
}

.onecalc-explore-shell__result-metric {
  justify-content: space-between;
  padding: var(--oc-space-2) var(--oc-space-3);
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: #fbf6ed;
}

.onecalc-explore-shell__array-preview {
  display: grid;
  gap: var(--oc-space-2);
  padding-top: var(--oc-space-3);
  border-top: 1px solid var(--oc-color-border);
}

.onecalc-explore-shell__array-grid {
  display: grid;
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__array-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(3rem, 1fr));
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__array-cell {
  padding: var(--oc-space-2);
  border-radius: 8px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
  font-family: var(--oc-font-mono);
  text-align: center;
}

.onecalc-explore-shell__editor-text,
.onecalc-inspect-shell__source,
.onecalc-workbench-shell__evidence-source {
  font-family: var(--oc-font-mono);
  white-space: pre-wrap;
  background: #fbf6ed;
  border-radius: 10px;
  padding: var(--oc-space-3);
}

.onecalc-explore-shell__syntax-runs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-1);
  font-family: var(--oc-font-mono);
}

.onecalc-formula-editor-surface__body {
  position: relative;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__native-input-layer,
.onecalc-formula-editor-surface__overlay-layer {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__textarea {
  min-height: 10rem;
  width: 100%;
  padding: var(--oc-space-3);
  border-radius: 10px;
  border: 1px solid var(--oc-color-border);
  background: #fffdf8;
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
}

.onecalc-formula-editor-surface__toolbar,
.onecalc-formula-editor-surface__footer,
.onecalc-formula-editor-surface__editor-state,
.onecalc-formula-editor-surface__diagnostic-markers,
.onecalc-formula-editor-surface__inline-diagnostic-spans {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__syntax-layer,
.onecalc-formula-editor-surface__selection-indicator,
.onecalc-formula-editor-surface__caret-indicator,
.onecalc-formula-editor-surface__scroll-indicator,
.onecalc-formula-editor-surface__diagnostic-markers {
  font-family: var(--oc-font-mono);
  font-size: 0.9rem;
}

.onecalc-formula-editor-surface__selection-indicator,
.onecalc-formula-editor-surface__caret-indicator,
.onecalc-formula-editor-surface__scroll-indicator,
.onecalc-formula-editor-surface__diagnostic-markers {
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__diagnostic-marker {
  display: inline-block;
  margin-right: var(--oc-space-2);
  color: var(--oc-color-warm);
}

.onecalc-formula-editor-surface__assist-indicator,
.onecalc-formula-editor-surface__inline-diagnostic {
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: rgba(255, 250, 242, 0.95);
  padding: var(--oc-space-2);
}

.onecalc-formula-editor-surface__completion-popup,
.onecalc-formula-editor-surface__signature-help-popup {
  display: grid;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-2);
  min-width: 16rem;
  padding: var(--oc-space-2);
  border: 1px solid var(--oc-color-border);
  border-radius: 12px;
  background: rgba(255, 250, 242, 0.98);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-formula-editor-surface__popup-container {
  z-index: 4;
}

.onecalc-formula-editor-surface__popup-container[data-focused-assist="completion"] .onecalc-formula-editor-surface__completion-popup,
.onecalc-formula-editor-surface__popup-container[data-focused-assist="signature"] .onecalc-formula-editor-surface__signature-help-popup {
  border-color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__completion-item {
  text-align: left;
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: var(--oc-color-surface);
  padding: var(--oc-space-2) var(--oc-space-3);
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__completion-item[data-selected="true"],
.onecalc-formula-editor-surface__completion-item[data-active-row="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 600;
}

.onecalc-formula-editor-surface__completion-item:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

.onecalc-signature-form {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 0;
}

.onecalc-signature-argument {
  padding: 0 0.1rem;
  border-radius: 6px;
}

.onecalc-signature-argument--active {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-explore-shell__function-help {
  display: grid;
  gap: var(--oc-space-3);
  margin-top: var(--oc-space-3);
  border-top: 1px solid var(--oc-color-border);
  padding-top: var(--oc-space-3);
}

.onecalc-explore-shell__function-help-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__function-help-status {
  padding: 0.2rem 0.5rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-size: 0.85rem;
}

.onecalc-explore-shell__function-help-status--limited {
  background: #f4e4c6;
  color: #8a5f19;
}

.onecalc-explore-shell__function-help-description {
  margin: 0;
  color: var(--oc-color-muted);
}

.onecalc-explore-shell__function-help-signatures,
.onecalc-explore-shell__function-help-arguments,
.onecalc-explore-shell__selected-proposal {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__function-help-signature,
.onecalc-explore-shell__function-help-argument {
  padding: var(--oc-space-2) var(--oc-space-3);
  border-radius: 10px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-explore-shell__function-help-argument--active {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
}

.onecalc-token[data-token-role=\"operator\"] { color: var(--oc-color-warm); }
.onecalc-token[data-token-role=\"function\"] { color: var(--oc-color-accent); font-weight: 600; }
.onecalc-token[data-token-role=\"number\"] { color: var(--oc-color-warning); }
.onecalc-token[data-token-role=\"delimiter\"] { color: var(--oc-color-muted); }
.onecalc-token[data-token-role=\"identifier\"] { color: var(--oc-color-success); }

.onecalc-inspect-shell__walk,
.onecalc-inspect-shell__walk-node-children {
  list-style: none;
  padding-left: var(--oc-space-4);
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__walk-node-header {
  display: flex;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__retained-context,
.onecalc-workbench-shell__import-field-group,
.onecalc-workbench-shell__import-outcome-guide {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__context-card {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-inspect-shell__retained-context {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: #fbf6ed;
}

.onecalc-workbench-shell__import-surface {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__import-label {
  font-weight: 600;
}

.onecalc-workbench-shell__import-help {
  color: var(--oc-color-muted);
  font-size: 0.9rem;
}

.onecalc-workbench-shell__import-outcome-guide {
  margin-top: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 12px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-workbench-shell__import-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

@media (max-width: 980px) {
  .onecalc-shell-frame {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__rail {
    border-right: none;
    border-bottom: 1px solid var(--oc-color-border);
  }

  .onecalc-explore-shell__body,
  .onecalc-inspect-shell__body,
  .onecalc-workbench-shell__body {
    grid-template-columns: 1fr;
  }
}
"#;

#[component]
pub fn ThemeStyleTag() -> impl IntoView {
    view! { <style data-theme="onecalc-theme">{ONECALC_THEME_CSS}</style> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_style_tag_renders_css_tokens() {
        let html = view! { <ThemeStyleTag /> }.to_html();
        assert!(html.contains("--oc-color-bg"));
        assert!(html.contains("onecalc-shell-frame"));
    }
}
