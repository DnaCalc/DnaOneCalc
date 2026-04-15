use leptos::prelude::*;

pub const ONECALC_THEME_CSS: &str = r#"
:root {
  --oc-color-bg: #f4ede2;
  --oc-color-surface: #fffaf4;
  --oc-color-panel: #eee4d6;
  --oc-color-ink: #1f2a2c;
  --oc-color-muted: #6b695f;
  --oc-color-border: #d9cfbe;
  --oc-color-accent: #245d5a;
  --oc-color-accent-soft: #d7ebe7;
  --oc-color-night: #183132;
  --oc-color-card-edge: rgba(36, 93, 90, 0.18);
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
  --oc-shadow-panel: 0 12px 28px rgba(31, 42, 44, 0.10);
  --oc-shadow-strong: 0 18px 42px rgba(24, 49, 50, 0.14);
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
  grid-template-columns: 18rem 1fr;
  min-height: 100vh;
  background:
    radial-gradient(circle at top left, rgba(255, 255, 255, 0.45), transparent 32rem),
    linear-gradient(180deg, #f8f1e7 0%, #f0e6d7 100%);
}

.onecalc-shell-frame__rail {
  padding: var(--oc-space-5);
  background:
    linear-gradient(180deg, rgba(36, 93, 90, 0.06), transparent 16rem),
    var(--oc-color-panel);
  border-right: 1px solid var(--oc-color-border);
  display: grid;
  align-content: start;
  gap: var(--oc-space-4);
}

.onecalc-shell-frame__content {
  display: grid;
  grid-template-rows: auto 1fr auto;
  min-height: 0;
  min-width: 0;
}

.onecalc-shell-frame__context-bar {
  display: grid;
  grid-template-columns: minmax(13rem, 0.8fr) minmax(0, 1.4fr) auto;
  align-items: start;
  gap: var(--oc-space-3);
  padding: var(--oc-space-4) var(--oc-space-5);
  border-bottom: 1px solid var(--oc-color-border);
  background: rgba(255, 250, 242, 0.92);
  backdrop-filter: blur(10px);
}

.onecalc-shell-frame__brand-block,
.onecalc-shell-frame__active-card,
.onecalc-shell-frame__context-copy {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__brand-copy {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.5;
}

.onecalc-shell-frame__active-card {
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(36, 93, 90, 0.16);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.96));
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-shell-frame__active-meta,
.onecalc-shell-frame__active-capability,
.onecalc-shell-frame__context-copy {
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__workspace-summary,
.onecalc-shell-frame__context-subtitle {
  color: var(--oc-color-muted);
  font-size: 0.9rem;
}

.onecalc-shell-frame__active-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__active-meta > span,
.onecalc-shell-frame__active-capability {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.7);
}

.onecalc-shell-frame__context-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--oc-color-night);
}

.onecalc-shell-frame__context-facts,
.onecalc-shell-frame__footer {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__context-facts {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-shell-frame__context-fact,
.onecalc-shell-frame__footer-fact {
  display: grid;
  gap: 0.2rem;
  padding: 0.65rem 0.8rem;
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.74);
}

.onecalc-shell-frame__context-fact[data-tone="accent"] {
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.12), rgba(255, 255, 255, 0.82));
  border-color: rgba(36, 93, 90, 0.2);
}

.onecalc-shell-frame__footer-fact[data-tone="warning"] {
  background: linear-gradient(180deg, rgba(197, 139, 47, 0.14), rgba(255, 255, 255, 0.9));
  border-color: rgba(197, 139, 47, 0.28);
}

.onecalc-shell-frame__context-fact-label,
.onecalc-shell-frame__footer-fact-label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.68rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-shell-frame__context-fact-value,
.onecalc-shell-frame__footer-fact-value {
  color: var(--oc-color-night);
  font-size: 0.92rem;
}

.onecalc-shell-frame__mode-switch {
  display: flex;
  gap: var(--oc-space-2);
  align-items: center;
}

.onecalc-shell-frame__configure-action {
  margin-left: var(--oc-space-2);
  padding: 0.4rem 0.9rem;
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-radius: var(--oc-radius-pill);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  cursor: pointer;
}

.onecalc-shell-frame__configure-action[data-open="true"] {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-shell-frame {
  --oc-shell-mode-accent: var(--oc-color-accent);
  --oc-shell-mode-accent-soft: var(--oc-color-accent-soft);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="explore"] {
  --oc-shell-mode-accent: #1e4d4a;
  --oc-shell-mode-accent-soft: rgba(30, 77, 74, 0.12);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="inspect"] {
  --oc-shell-mode-accent: #3e5238;
  --oc-shell-mode-accent-soft: rgba(62, 82, 56, 0.14);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="workbench"] {
  --oc-shell-mode-accent: #b84532;
  --oc-shell-mode-accent-soft: rgba(184, 69, 50, 0.14);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame__mode-button {
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-pill);
  background: transparent;
  padding: 0.4rem 0.8rem;
  color: var(--oc-color-ink);
}

.onecalc-shell-frame__mode-button--active {
  background: var(--oc-shell-mode-accent);
  color: var(--oc-shell-mode-accent-ink);
  border-color: var(--oc-shell-mode-accent);
  box-shadow: 0 8px 18px rgba(31, 28, 23, 0.18);
}

.onecalc-shell-frame__mode-body {
  padding: var(--oc-space-4) var(--oc-space-5);
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.onecalc-shell-frame__mode-body > * {
  flex: 1 1 auto;
  min-height: 0;
  min-width: 0;
}

.onecalc-shell-frame__footer {
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  padding: var(--oc-space-3) var(--oc-space-5) var(--oc-space-4);
  border-top: 1px solid var(--oc-color-border);
  background: rgba(255, 251, 244, 0.92);
}

.onecalc-shell-frame__rail h1,
.onecalc-inspect-shell__header h1,
.onecalc-workbench-shell__header h1 {
  margin: 0;
  font-size: 1.7rem;
  letter-spacing: -0.03em;
}

.onecalc-inspect-shell__eyebrow,
.onecalc-workbench-shell__eyebrow,
.onecalc-shell-frame__eyebrow {
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-inspect-shell__lead,
.onecalc-workbench-shell__lead {
  margin: 0;
  max-width: 58rem;
  color: var(--oc-color-muted);
  line-height: 1.65;
  font-size: 1rem;
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
  display: grid;
  gap: 0.2rem;
}

.onecalc-shell-frame__space-button-label {
  font-weight: 700;
}

.onecalc-shell-frame__space-button-meta,
.onecalc-shell-frame__space-button-packet {
  color: var(--oc-color-muted);
  font-size: 0.82rem;
}

.onecalc-shell-frame__space-item--active {
  border-left: 4px solid var(--oc-shell-mode-accent);
  border-top: 1px solid var(--oc-shell-mode-accent-soft);
  border-right: 1px solid var(--oc-shell-mode-accent-soft);
  border-bottom: 1px solid var(--oc-shell-mode-accent-soft);
  background: linear-gradient(90deg, var(--oc-shell-mode-accent-soft), var(--oc-color-surface) 60%);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-shell-frame__rail-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-3);
}

.onecalc-shell-frame__rail-action-buttons {
  display: flex;
  gap: 0.3rem;
}

.onecalc-shell-frame__rail-action-button {
  width: 1.8rem;
  height: 1.8rem;
  border-radius: 999px;
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 1.1rem;
  font-weight: 700;
  cursor: pointer;
  line-height: 1;
}

.onecalc-shell-frame__rail-action-button:hover {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-shell-frame__rail-section {
  display: grid;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-2);
}

.onecalc-shell-frame__rail-section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-shell-frame__rail-section-count {
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  background: rgba(31, 28, 23, 0.08);
  color: var(--oc-color-muted);
  font-size: 0.66rem;
}

.onecalc-shell-frame__rail-section-empty {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-radius: 10px;
  border: 1px dashed rgba(31, 28, 23, 0.14);
  background: rgba(255, 252, 246, 0.6);
  color: var(--oc-color-muted);
  font-size: 0.78rem;
  font-style: italic;
}

.onecalc-shell-frame__space-button-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.onecalc-shell-frame__space-dirty-dot {
  display: inline-block;
  width: 0.55rem;
  height: 0.55rem;
  border-radius: 999px;
  background: var(--oc-color-warning, #c88d2e);
  box-shadow: 0 0 0 2px rgba(200, 141, 46, 0.18);
}

.onecalc-shell-frame__space-verdicts {
  display: flex;
  gap: 0.25rem;
  align-items: center;
  margin-top: 0.35rem;
}

.onecalc-shell-frame__space-verdict {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.2rem;
  height: 1.2rem;
  border-radius: 999px;
  font-family: var(--oc-font-mono);
  font-size: 0.6rem;
  font-weight: 800;
  border: 1px solid rgba(31, 28, 23, 0.15);
  background: rgba(255, 252, 246, 0.9);
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__space-verdict[data-verdict="pass"] {
  border-color: rgba(79, 123, 87, 0.55);
  background: rgba(79, 123, 87, 0.18);
  color: #33532c;
}

.onecalc-shell-frame__space-verdict[data-verdict="fail"] {
  border-color: rgba(184, 69, 50, 0.55);
  background: rgba(184, 69, 50, 0.14);
  color: #a75842;
}

.onecalc-shell-frame__space-verdict[data-verdict="unobserved"] {
  border-style: dashed;
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__space-verdict-lane {
  margin-left: 0.25rem;
  font-size: 0.64rem;
  color: var(--oc-color-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.onecalc-shell-frame__space-affordances {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.35rem;
  opacity: 0;
  transition: opacity 120ms ease-out;
}

.onecalc-shell-frame__space-item:hover .onecalc-shell-frame__space-affordances,
.onecalc-shell-frame__space-item--active .onecalc-shell-frame__space-affordances {
  opacity: 1;
}

.onecalc-shell-frame__space-affordance {
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.14);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-muted);
  font-size: 0.66rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  cursor: pointer;
}

.onecalc-shell-frame__space-affordance:hover {
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-color: var(--oc-shell-mode-accent, var(--oc-color-accent));
}

.onecalc-shell-frame[data-active-mode] .onecalc-shell-frame__context-title {
  color: var(--oc-shell-mode-accent);
}

.onecalc-shell-frame[data-active-mode] .onecalc-shell-frame__active-card {
  border-left: 3px solid var(--oc-shell-mode-accent);
  padding-left: calc(var(--oc-space-3) - 2px);
}

.onecalc-shell-frame__context-bar {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
  padding: 0.85rem 1.25rem 0.9rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
  background: linear-gradient(180deg, rgba(255, 252, 246, 0.95), rgba(246, 239, 225, 0.9));
  backdrop-filter: blur(8px);
}

.onecalc-shell-frame__breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
  font-size: 0.8rem;
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__breadcrumb-segment {
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
  color: var(--oc-color-ink);
  font-weight: 600;
}

.onecalc-shell-frame__breadcrumb-segment--space {
  color: var(--oc-color-ink);
  font-weight: 700;
}

.onecalc-shell-frame__breadcrumb-segment--mode {
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-size: 0.72rem;
  padding: 0.2rem 0.55rem;
  background: var(--oc-shell-mode-accent-soft, rgba(36, 93, 90, 0.12));
  border-radius: 999px;
}

.onecalc-shell-frame__breadcrumb-separator {
  color: var(--oc-color-muted);
  opacity: 0.6;
}

.onecalc-shell-frame__scope-strip {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.onecalc-shell-frame__scope-segment {
  display: inline-flex;
  align-items: baseline;
  gap: 0.35rem;
  padding: 0.25rem 0.55rem;
  border-radius: 8px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 252, 246, 0.92);
  font-size: 0.78rem;
  color: var(--oc-color-ink);
}

.onecalc-shell-frame__scope-segment[data-status="live"] {
  border-color: rgba(79, 123, 87, 0.35);
  background: rgba(79, 123, 87, 0.08);
}

.onecalc-shell-frame__scope-segment[data-status="not-implemented"] {
  border-style: dashed;
  border-color: rgba(183, 101, 69, 0.4);
  background: rgba(183, 101, 69, 0.06);
  color: #a75842;
}

.onecalc-shell-frame__scope-segment[data-status="not-implemented"] .onecalc-shell-frame__scope-segment-value {
  text-decoration: underline dotted rgba(183, 101, 69, 0.55);
  text-underline-offset: 3px;
}

.onecalc-shell-frame__scope-segment-label {
  color: var(--oc-color-muted);
  text-transform: uppercase;
  font-size: 0.64rem;
  letter-spacing: 0.06em;
  font-weight: 700;
}

.onecalc-shell-frame__scope-segment-value {
  font-family: var(--oc-font-mono);
  font-weight: 700;
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
  grid-template-columns: minmax(0, 2fr) minmax(18rem, 1.5fr) minmax(16rem, 1fr);
  align-items: stretch;
  min-height: 0;
  flex: 1 1 auto;
}

.onecalc-explore-shell {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
}

.onecalc-explore-shell__body-column {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-3);
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.onecalc-explore-shell__body-column > * {
  min-height: 0;
}

.onecalc-inspect-shell__column,
.onecalc-workbench-shell__column {
  display: grid;
  gap: var(--oc-space-4);
  align-content: start;
}

.onecalc-inspect-shell__body {
  grid-template-columns: minmax(16rem, 0.72fr) minmax(0, 1.28fr) minmax(18rem, 0.9fr);
}

.onecalc-workbench-shell__body {
  grid-template-columns: minmax(0, 1.18fr) minmax(18rem, 0.86fr) minmax(18rem, 0.86fr);
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

.onecalc-explore-shell__editor-panel,
.onecalc-inspect-shell__walk-cluster {
  border-color: var(--oc-color-card-edge);
}

.onecalc-explore-shell__panel-header,
.onecalc-explore-shell__editor-summary-row,
.onecalc-explore-shell__result-metric,
.onecalc-explore-shell__array-preview-header,
.onecalc-inspect-shell__meta,
.onecalc-workbench-shell__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  align-items: center;
}

.onecalc-inspect-shell__header,
.onecalc-workbench-shell__header {
  display: grid;
  gap: var(--oc-space-3);
  margin-bottom: var(--oc-space-5);
}

.onecalc-inspect-shell__overview-deck,
.onecalc-workbench-shell__overview-deck {
  display: grid;
  gap: var(--oc-space-3);
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-inspect-shell__overview-card,
.onecalc-workbench-shell__overview-card {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(244, 236, 223, 0.96));
  box-shadow: 0 16px 34px rgba(31, 28, 23, 0.06);
}

.onecalc-inspect-shell__overview-card strong,
.onecalc-workbench-shell__overview-card strong {
  font-size: 1.08rem;
  color: var(--oc-color-night);
}

.onecalc-inspect-shell__overview-card p,
.onecalc-workbench-shell__overview-card p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.55;
}

.onecalc-explore-shell__panel-intro {
  display: grid;
  gap: var(--oc-space-1);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__panel-intro p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.6;
}

.onecalc-explore-shell__editor-summary-row,
.onecalc-explore-shell__assist-meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: var(--oc-space-2);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__status-card,
.onecalc-explore-shell__assist-metric,
.onecalc-inspect-shell__source-card {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffdf8, #fbf4e7);
}

.onecalc-explore-shell__status-label,
.onecalc-explore-shell__metric-label,
.onecalc-inspect-shell__source-label,
.onecalc-inspect-shell__walk-node-value-label,
.onecalc-explore-shell__hero-result-label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-explore-shell__scenario-label,
.onecalc-explore-shell__truth-chip,
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

.onecalc-explore-shell__editor-note {
  display: grid;
  gap: 0.2rem;
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(36, 93, 90, 0.16);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.92));
}

.onecalc-explore-shell__editor-note-detail {
  color: var(--oc-color-muted);
  font-size: 0.88rem;
  line-height: 1.5;
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

.onecalc-explore-shell__hero-result {
  display: grid;
  gap: var(--oc-space-1);
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-4);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.08), rgba(255, 255, 255, 0.92));
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-explore-shell__hero-result-value {
  font-family: var(--oc-font-mono);
  font-size: 1.5rem;
  line-height: 1.2;
  color: var(--oc-color-night);
}

.onecalc-explore-shell__result-state-chip {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
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
  background: linear-gradient(180deg, #fffefb, #fef8ef);
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
  box-shadow: inset 0 1px 2px rgba(31, 42, 44, 0.06);
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

.onecalc-formula-editor-surface__completion-popup {
  max-height: calc(1.9rem * 8 + 1rem);
  overflow-y: auto;
  overscroll-behavior: contain;
}

.onecalc-formula-editor-surface__popup-container {
  z-index: 4;
  pointer-events: auto;
}

.onecalc-formula-editor-surface__popup-container--signature {
  pointer-events: none;
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-formula-editor-surface__signature-help-popup {
  min-width: 12rem;
  max-width: 28rem;
  margin-top: 0;
  padding: 0.5rem 0.75rem;
  border-color: rgba(255, 255, 255, 0.15);
  background: var(--oc-color-night);
  color: #faf7f1;
  box-shadow: 0 14px 28px rgba(24, 49, 50, 0.32);
  font-family: var(--oc-font-mono);
  font-size: 0.82rem;
}

.onecalc-formula-editor-surface__popup-container--signature [data-role="signature-help-callee"] {
  color: var(--oc-color-amber, #c88d2e);
  font-weight: 700;
  margin-right: 0.35rem;
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-signature-argument {
  color: rgba(250, 247, 241, 0.72);
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-signature-argument--active {
  color: #fff;
  font-weight: 700;
  border-bottom: 1px solid #c88d2e;
}

.onecalc-formula-editor-surface__popup-container[data-focused-assist="completion"] .onecalc-formula-editor-surface__completion-popup {
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
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(180deg, #fffdf9, #f8efe3);
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

.onecalc-explore-shell__selected-proposal {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(180deg, #fffef9, #f4ecdf);
}

.onecalc-explore-shell__selected-proposal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
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

.onecalc-inspect-shell__walk-node {
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-inspect-shell__walk-node-state {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-inspect-shell__walk-node-value {
  display: grid;
  gap: var(--oc-space-1);
  margin-top: var(--oc-space-2);
}

.onecalc-inspect-shell__retained-context,
.onecalc-workbench-shell__import-field-group,
.onecalc-workbench-shell__import-outcome-guide {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__source-stack,
.onecalc-inspect-shell__gap-board {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__retained-context-header,
.onecalc-workbench-shell__catalog-item-header,
.onecalc-inspect-shell__comparison-record-header,
.onecalc-workbench-shell__comparison-record-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__retained-context-badges,
.onecalc-inspect-shell__comparison-record-badges,
.onecalc-workbench-shell__comparison-record-badges,
.onecalc-workbench-shell__catalog-item-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__retained-context-badges > span,
.onecalc-inspect-shell__comparison-record-badges > span,
.onecalc-workbench-shell__comparison-record-badges > span,
.onecalc-workbench-shell__catalog-item-header > span,
.onecalc-workbench-shell__outcome-chip {
  padding: 0.3rem 0.75rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-color-border);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
  font-size: 0.82rem;
}

.onecalc-inspect-shell__comparison-board,
.onecalc-inspect-shell__explain-board,
.onecalc-workbench-shell__compare-card,
.onecalc-workbench-shell__replay-card,
.onecalc-workbench-shell__actions-card {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__summary-grid,
.onecalc-inspect-shell__comparison-grid,
.onecalc-inspect-shell__explain-grid,
.onecalc-workbench-shell__comparison-grid,
.onecalc-workbench-shell__score-grid {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__comparison-grid,
.onecalc-inspect-shell__summary-grid,
.onecalc-workbench-shell__comparison-grid,
.onecalc-workbench-shell__score-grid {
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
}

.onecalc-inspect-shell__comparison-record,
.onecalc-inspect-shell__explain-record,
.onecalc-workbench-shell__comparison-record,
.onecalc-workbench-shell__explain-record,
.onecalc-workbench-shell__score-card,
.onecalc-workbench-shell__catalog-item {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-inspect-shell__comparison-record[data-projection-gap="true"],
.onecalc-workbench-shell__comparison-record[data-projection-gap="true"] {
  border-color: #d59b5b;
  background: linear-gradient(180deg, #fff9ed, #f7ead3);
}

.onecalc-inspect-shell__comparison-lane,
.onecalc-workbench-shell__comparison-lane {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__comparison-lane-card,
.onecalc-workbench-shell__comparison-lane-card {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-2);
  border-radius: 10px;
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.72);
}

.onecalc-inspect-shell__comparison-detail,
.onecalc-workbench-shell__comparison-label,
.onecalc-workbench-shell__catalog-item,
.onecalc-inspect-shell__retained-context {
  color: var(--oc-color-muted);
}

.onecalc-workbench-shell__hero-outcome {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-4);
  border-radius: 14px;
  border: 1px solid rgba(36, 93, 90, 0.18);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.09), rgba(255, 255, 255, 0.94));
}

.onecalc-workbench-shell__catalog-item-actions button,
.onecalc-shell-frame__mode-button,
.onecalc-workbench-shell__import-buttons button,
.onecalc-workbench-shell__catalog-card button,
.onecalc-workbench-shell__bundle-import-surface button {
  cursor: pointer;
}

.onecalc-workbench-shell__catalog-item-actions,
.onecalc-workbench-shell__import-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__catalog-item-actions button,
.onecalc-workbench-shell__import-buttons button,
.onecalc-workbench-shell__bundle-import-surface button {
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: #fffdf8;
  color: var(--oc-color-ink);
  padding: 0.55rem 0.9rem;
  font: inherit;
}

.onecalc-inspect-shell__context-card {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-inspect-shell__walk-intro,
.onecalc-workbench-shell__outcome-ledger {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(36, 93, 90, 0.15);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.9));
  color: var(--oc-color-muted);
}

.onecalc-inspect-shell__summary-cluster {
  align-content: start;
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

.onecalc-inspect-shell__column--left .onecalc-inspect-shell__source-stack,
.onecalc-inspect-shell__column--summary .onecalc-inspect-shell__summary-cluster,
.onecalc-workbench-shell__column--support .onecalc-workbench-shell__replay-card,
.onecalc-workbench-shell__column--actions .onecalc-workbench-shell__actions-card {
  position: sticky;
  top: 0;
}

.onecalc-workbench-shell__bundle-import-textarea {
  min-height: 12rem;
  width: 100%;
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(46, 62, 82, 0.18);
  background: rgba(255, 250, 243, 0.92);
  color: var(--oc-color-ink);
  font: 0.9rem/1.45 "Consolas", "SFMono-Regular", monospace;
  resize: vertical;
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

[data-role="retained-import-upstream-gap-summary"],
[data-role="inspect-retained-upstream-gap-summary"] {
  display: grid;
  gap: var(--oc-space-2);
  margin: 0;
  padding-left: 1.25rem;
  color: #8d4f2f;
}

.onecalc-explore-shell__header-copy,
.onecalc-workbench-shell__header-copy,
.onecalc-inspect-shell__header-copy {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-4);
}

.onecalc-explore-shell__hero-badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__hero-badges > span,
.onecalc-workbench-shell__meta > span,
.onecalc-inspect-shell__meta > span {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 251, 245, 0.92);
  box-shadow: 0 6px 18px rgba(31, 28, 23, 0.05);
}

.onecalc-workbench-shell__overview-card[data-role="workbench-overview-outcome"],
.onecalc-inspect-shell__overview-card[data-role="inspect-overview-source"] {
  background: linear-gradient(155deg, rgba(36, 93, 90, 0.11), rgba(255, 252, 246, 0.96));
}

.onecalc-explore-shell__section-accent,
.onecalc-workbench-shell__section-accent,
.onecalc-inspect-shell__section-accent {
  width: 0.3rem;
  height: 2rem;
  border-radius: 999px;
  background: linear-gradient(180deg, var(--oc-color-warm), var(--oc-color-warning));
  box-shadow: 0 6px 14px rgba(183, 101, 69, 0.25);
  margin-bottom: var(--oc-space-2);
}

.onecalc-explore-shell__panel-header,
.onecalc-workbench-shell__panel-header,
.onecalc-inspect-shell__panel-header {
  align-items: flex-start;
  justify-content: space-between;
  padding-bottom: var(--oc-space-3);
  margin-bottom: var(--oc-space-3);
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-explore-shell__editor-panel,
.onecalc-explore-shell__result-panel,
.onecalc-explore-shell__help-panel {
  overflow: hidden;
  background:
    linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(248, 239, 227, 0.96)),
    var(--oc-color-surface);
  border: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: 0 22px 48px rgba(31, 28, 23, 0.08);
}

.onecalc-explore-shell__editor-panel {
  position: relative;
}

.onecalc-explore-shell__editor-panel::before,
.onecalc-explore-shell__result-panel::before,
.onecalc-explore-shell__help-panel::before,
.onecalc-workbench-shell__outcome-card::before,
.onecalc-workbench-shell__evidence-card::before,
.onecalc-workbench-shell__compare-card::before,
.onecalc-workbench-shell__replay-card::before,
.onecalc-workbench-shell__lineage-card::before,
.onecalc-workbench-shell__actions-card::before,
.onecalc-workbench-shell__catalog-card::before,
.onecalc-inspect-shell__source-stack::before,
.onecalc-inspect-shell__walk-cluster::before,
.onecalc-inspect-shell__summary-cluster::before {
  content: "";
  display: block;
  height: 0.24rem;
  margin: calc(-1 * var(--oc-space-4)) calc(-1 * var(--oc-space-4)) var(--oc-space-4);
  background: linear-gradient(90deg, var(--oc-color-accent), rgba(200, 141, 46, 0.7), rgba(184, 69, 50, 0.6));
}

.onecalc-explore-shell__context-strip {
  padding: var(--oc-space-2) var(--oc-space-3);
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.07), rgba(255, 255, 255, 0.7));
  border: 1px solid rgba(36, 93, 90, 0.14);
}

.onecalc-explore-shell__status-card,
.onecalc-explore-shell__assist-metric,
.onecalc-inspect-shell__source-card,
.onecalc-workbench-shell__score-card,
.onecalc-workbench-shell__observation-card {
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(248, 239, 227, 0.96));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8);
}

.onecalc-explore-shell__assist-intro {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 251, 245, 0.88);
  color: var(--oc-color-muted);
  line-height: 1.5;
}

.onecalc-explore-shell__overview-deck {
  display: grid;
  grid-template-columns: 1.15fr 0.85fr 0.95fr;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__overview-card {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(244, 236, 223, 0.96));
  box-shadow: 0 16px 34px rgba(31, 28, 23, 0.06);
}

.onecalc-explore-shell__overview-card strong {
  font-size: 1.08rem;
  color: var(--oc-color-night);
}

.onecalc-explore-shell__overview-card p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.55;
}

.onecalc-explore-shell__overview-card[data-role="explore-overview-primary"] {
  background: linear-gradient(155deg, rgba(36, 93, 90, 0.11), rgba(255, 252, 246, 0.96));
}

.onecalc-explore-shell__overview-card[data-role="explore-overview-result"] strong {
  font-family: var(--oc-font-mono);
  font-size: 1.45rem;
}

.onecalc-explore-shell__body-column--result .onecalc-explore-shell__result-panel,
.onecalc-explore-shell__body-column--help .onecalc-explore-shell__help-panel {
  position: sticky;
  top: 0;
}

.onecalc-explore-shell__hero-result {
  grid-template-columns: minmax(0, 1.4fr) minmax(14rem, 0.9fr);
  align-items: stretch;
  padding: var(--oc-space-5);
}

.onecalc-explore-shell__hero-result-copy,
.onecalc-explore-shell__hero-result-sidecar {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__hero-result-sidecar {
  align-content: start;
}

.onecalc-explore-shell__hero-result-value {
  font-size: clamp(1.8rem, 3vw, 2.6rem);
}

.onecalc-explore-shell__hero-result-caption {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.45;
}

.onecalc-explore-shell__hero-pill {
  display: grid;
  gap: 0.2rem;
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 255, 255, 0.78);
}

.onecalc-explore-shell__hero-pill > span {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-explore-shell__result-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--oc-space-2);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__assist-callout {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 16px;
  border: 1px solid rgba(36, 93, 90, 0.15);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 255, 255, 0.78));
}

.onecalc-explore-shell__assist-callout > div {
  display: grid;
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__assist-callout-state {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  background: rgba(36, 93, 90, 0.14);
  color: var(--oc-color-accent);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-explore-shell__selected-proposal,
.onecalc-explore-shell__function-help {
  box-shadow: 0 14px 28px rgba(31, 28, 23, 0.06);
}

.onecalc-formula-editor-surface {
  display: grid;
  position: relative;
  gap: 0;
  border-radius: 18px;
  overflow: hidden;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: linear-gradient(180deg, #fffdf9, #f8f1e4);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85);
}

.onecalc-formula-editor-surface__toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--oc-space-3);
  padding: 1rem 1.25rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, #f7f3ea, #eee4d6);
}

.onecalc-formula-editor-surface__toolbar-copy {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-4);
  min-width: 0;
  flex: 1;
}

.onecalc-formula-editor-surface__toolbar-meta {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--oc-space-2);
  min-width: 0;
}

.onecalc-formula-editor-surface__toolbar-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: #1f1c17;
}

.onecalc-formula-editor-surface__toolbar-subtitle {
  color: var(--oc-color-muted);
  font-size: 0.82rem;
  margin-top: 0.1rem;
}

.onecalc-formula-editor-surface__toolbar-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  color: var(--oc-color-muted);
  font-size: 0.8rem;
}

.onecalc-formula-editor-surface__toolbar-state {
  white-space: nowrap;
  padding: 0.45rem 0.9rem;
  border-radius: 12px;
  background: var(--oc-color-accent);
  color: #fff;
  font-size: 0.82rem;
  font-weight: 700;
  box-shadow: 0 8px 18px rgba(36, 93, 90, 0.22);
}

.onecalc-formula-editor-surface__toolbar-pills {
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  flex-wrap: nowrap;
  min-width: 0;
}

.onecalc-formula-editor-surface__toolbar-actions {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  flex: 0 0 auto;
}

.onecalc-formula-editor-surface__entry-mode-pill,
.onecalc-formula-editor-surface__result-class-pill,
.onecalc-formula-editor-surface__live-state-pill {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.7rem;
  white-space: nowrap;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="formula"] {
  border-color: rgba(36, 93, 90, 0.35);
  background: rgba(36, 93, 90, 0.1);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="value"] {
  border-color: rgba(200, 141, 46, 0.4);
  background: rgba(200, 141, 46, 0.12);
  color: #8d5f1e;
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="text"] {
  border-color: rgba(62, 82, 56, 0.4);
  background: rgba(62, 82, 56, 0.12);
  color: #3e5238;
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="empty"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-formula-editor-surface__result-class-pill[data-has-result="false"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="idle"] {
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="editing-live"] {
  border-color: rgba(200, 141, 46, 0.45);
  background: rgba(200, 141, 46, 0.14);
  color: #8d5f1e;
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="proofed-scratch"] {
  border-color: rgba(36, 93, 90, 0.45);
  background: rgba(36, 93, 90, 0.12);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="committed"] {
  border-color: rgba(79, 123, 87, 0.5);
  background: rgba(79, 123, 87, 0.16);
  color: #33532c;
}

.onecalc-formula-editor-surface[data-expanded-editor="true"] .onecalc-formula-editor-surface__textarea {
  min-height: 32rem;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__overlay-layer {
  display: none;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__bracket-pair-layer,
.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__popup-container,
.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__toolbar-pills {
  display: none;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__toolbar::after {
  content: "Plain mode";
  margin-left: var(--oc-space-2);
  padding: 0.3rem 0.7rem;
  border-radius: 999px;
  background: rgba(31, 28, 23, 0.06);
  color: var(--oc-color-muted);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__textarea {
  font: 0.95rem/1.6rem var(--oc-font-mono);
  padding: 0.85rem 1rem;
  background: #fff;
}

.onecalc-formula-editor-surface__inline-diagnostic {
  position: relative;
  color: var(--oc-color-warm);
  text-decoration-line: underline;
  text-decoration-style: wavy;
  text-decoration-color: var(--oc-color-warm);
  text-underline-offset: 3px;
}

.onecalc-formula-editor-surface__bracket-pair-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.onecalc-formula-editor-surface__bracket-pair {
  box-sizing: border-box;
  border: 1px solid rgba(36, 93, 90, 0.6);
  background: rgba(36, 93, 90, 0.1);
  border-radius: 3px;
}

.onecalc-formula-editor-surface__settings-gear {
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-ink);
  font-size: 1rem;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-gear[data-open="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__settings-popover {
  position: absolute;
  top: calc(100% + 0.65rem);
  right: 0;
  width: min(36rem, calc(100vw - 4rem));
  max-width: min(36rem, calc(100vw - 4rem));
  border: 1px solid rgba(31, 28, 23, 0.12);
  border-radius: 16px;
  background: linear-gradient(180deg, #fffdf9, #f8f1e4);
  box-shadow: 0 18px 34px rgba(31, 28, 23, 0.12);
  padding: 0.9rem 1rem;
  z-index: 5;
}

.onecalc-formula-editor-surface__settings-popover-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__settings-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 0.55rem 0.8rem;
  border-radius: 10px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.9);
  color: var(--oc-color-ink);
  font-size: 0.82rem;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-toggle[data-checked="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__settings-choice {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.55rem 0.8rem;
  border-radius: 10px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.9);
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__settings-choice-buttons {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.onecalc-formula-editor-surface__settings-choice-button {
  padding: 0.3rem 0.55rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.85);
  color: var(--oc-color-ink);
  font-size: 0.72rem;
  font-weight: 700;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-choice-button[data-active="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__body {
  display: grid;
  grid-template-columns: 4.2rem minmax(0, 1fr);
  min-height: 20rem;
  background: #fff;
}

.onecalc-formula-editor-surface__line-rail {
  display: grid;
  align-content: start;
  gap: 0;
  padding: 1rem 0;
  background: linear-gradient(180deg, #faf7f1, #f0e8dc);
  border-right: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-formula-editor-surface__line-number {
  height: 1.9rem;
  line-height: 1.9rem;
  padding: 0 0.85rem;
  text-align: right;
  font-family: var(--oc-font-mono);
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__line-number--active {
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__editor-stage {
  position: relative;
  min-width: 0;
}

.onecalc-formula-editor-surface__native-input-layer,
.onecalc-formula-editor-surface__overlay-layer {
  inset: 0;
  padding: 1rem 1.1rem;
}

.onecalc-formula-editor-surface__textarea {
  width: 100%;
  min-height: 20rem;
  padding: 0;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  color: #1f1c17;
  font: 0.95rem/1.9rem var(--oc-font-mono);
}

.onecalc-formula-editor-surface__overlay-layer {
  pointer-events: none;
}

.onecalc-formula-editor-surface__completion-popup,
.onecalc-formula-editor-surface__signature-help-popup {
  min-width: 15rem;
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.97);
  box-shadow: 0 18px 36px rgba(31, 28, 23, 0.14);
  backdrop-filter: blur(10px);
}

.onecalc-formula-editor-surface__completion-item {
  width: 100%;
  text-align: left;
  padding: 0.7rem 0.85rem;
  border: none;
  background: transparent;
  color: var(--oc-color-ink);
  font: inherit;
}

.onecalc-formula-editor-surface__completion-item[data-active-row="true"] {
  background: linear-gradient(90deg, rgba(36, 93, 90, 0.12), rgba(255, 255, 255, 0.65));
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__diagnostic-band {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-3);
  padding: 0.9rem 1.25rem;
  border-top: 1px solid rgba(31, 28, 23, 0.08);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.06), rgba(255, 255, 255, 0.84));
}

.onecalc-formula-editor-surface__diagnostic-band-state {
  display: flex;
  align-items: flex-start;
  gap: var(--oc-space-3);
}

.onecalc-formula-editor-surface__diagnostic-band-state > div {
  display: grid;
  gap: 0.2rem;
  color: var(--oc-color-muted);
  font-size: 0.84rem;
}

.onecalc-formula-editor-surface__diagnostic-band-state strong {
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__diagnostic-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 999px;
  background: var(--oc-color-accent);
  color: white;
  font-size: 0.72rem;
  font-weight: 700;
}

.onecalc-formula-editor-surface__diagnostic-band-action {
  padding: 0.35rem 0.75rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 255, 255, 0.75);
  color: var(--oc-color-ink);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-formula-editor-surface__footer {
  border-top: 1px solid rgba(31, 28, 23, 0.08);
  background: #fcf8f1;
}

.onecalc-workbench-shell__hero-outcome {
  padding: var(--oc-space-5);
  border: 1px solid rgba(184, 69, 50, 0.18);
  background: linear-gradient(145deg, rgba(184, 69, 50, 0.06), rgba(255, 255, 255, 0.94));
}

.onecalc-workbench-shell__score-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-workbench-shell__score-card {
  border: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: 0 12px 26px rgba(31, 28, 23, 0.06);
}

.onecalc-workbench-shell__observation-envelope {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-workbench-shell__observation-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__observation-card {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  color: var(--oc-color-muted);
}

.onecalc-workbench-shell__timeline {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__timeline-item {
  display: grid;
  grid-template-columns: 1.25rem minmax(0, 1fr);
  gap: var(--oc-space-3);
  align-items: start;
}

.onecalc-workbench-shell__timeline-dot {
  width: 0.9rem;
  height: 0.9rem;
  margin-top: 0.55rem;
  border-radius: 999px;
  background: linear-gradient(180deg, var(--oc-color-accent), var(--oc-color-night));
  box-shadow: 0 0 0 0.22rem rgba(36, 93, 90, 0.12);
}

.onecalc-workbench-shell__timeline-card,
.onecalc-workbench-shell__action-item {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-workbench-shell__action-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__walk-node {
  position: relative;
  overflow: hidden;
}

.onecalc-inspect-shell__walk-node::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 0.3rem;
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.7), rgba(184, 69, 50, 0.55));
}

.onecalc-inspect-shell__summary-card,
.onecalc-inspect-shell__comparison-record,
.onecalc-inspect-shell__explain-record {
  box-shadow: 0 14px 28px rgba(31, 28, 23, 0.06);
}

@media (max-width: 980px) {
  .onecalc-shell-frame {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__rail {
    border-right: none;
    border-bottom: 1px solid var(--oc-color-border);
  }

  .onecalc-shell-frame__context-bar {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__context-facts {
    grid-template-columns: 1fr;
  }

  .onecalc-explore-shell__body,
  .onecalc-inspect-shell__body,
  .onecalc-workbench-shell__body {
    grid-template-columns: 1fr;
  }

  .onecalc-explore-shell__hero-result,
  .onecalc-explore-shell__overview-deck,
  .onecalc-inspect-shell__overview-deck,
  .onecalc-workbench-shell__overview-deck,
  .onecalc-workbench-shell__score-grid,
  .onecalc-explore-shell__result-grid {
    grid-template-columns: 1fr;
  }

  .onecalc-explore-shell__header-copy,
  .onecalc-workbench-shell__header-copy,
  .onecalc-inspect-shell__header-copy,
  .onecalc-formula-editor-surface__toolbar,
  .onecalc-formula-editor-surface__toolbar-copy,
  .onecalc-formula-editor-surface__diagnostic-band {
    grid-template-columns: 1fr;
    display: grid;
  }

  .onecalc-formula-editor-surface__toolbar-meta {
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .onecalc-formula-editor-surface__toolbar-pills {
    flex-wrap: wrap;
  }

  .onecalc-formula-editor-surface__settings-popover {
    position: static;
    width: auto;
    max-width: none;
    margin-top: var(--oc-space-2);
  }

  .onecalc-formula-editor-surface__body {
    grid-template-columns: 3.2rem minmax(0, 1fr);
  }

  .onecalc-inspect-shell__column--left .onecalc-inspect-shell__source-stack,
  .onecalc-inspect-shell__column--summary .onecalc-inspect-shell__summary-cluster,
  .onecalc-workbench-shell__column--support .onecalc-workbench-shell__replay-card,
  .onecalc-workbench-shell__column--actions .onecalc-workbench-shell__actions-card {
    position: static;
  }
}

.onecalc-explore-shell__configure-button {
  padding: 0.45rem 0.9rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  background: rgba(255, 252, 246, 0.9);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  cursor: pointer;
  margin-left: var(--oc-space-2);
}

.onecalc-explore-shell__configure-button[data-open="true"] {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-explore-shell__body-column[data-drawer-open="true"] {
  padding: 0;
}

.onecalc-explore-shell__configure-placeholder {
  display: grid;
  gap: var(--oc-space-2);
  color: var(--oc-color-ink);
  font-size: 0.85rem;
  line-height: 1.55;
}

.onecalc-explore-shell__configure-placeholder p {
  margin: 0;
  color: var(--oc-color-muted);
}

.onecalc-explore-shell__configure-placeholder ul {
  margin: 0;
  padding-left: 1.1rem;
}

.onecalc-shell-drawer {
  display: grid;
  grid-template-rows: auto 1fr;
  min-width: 20rem;
  max-width: 32rem;
  background: linear-gradient(180deg, #fffdf9, #f5efe1);
  border-left: 4px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-top: 1px solid rgba(31, 28, 23, 0.1);
  border-bottom: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: -18px 0 36px rgba(31, 28, 23, 0.14);
  color: var(--oc-color-ink);
  animation: onecalc-shell-drawer-slide-in 180ms ease-out both;
}

@keyframes onecalc-shell-drawer-slide-in {
  from {
    transform: translateX(12px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.onecalc-shell-drawer__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--oc-space-2);
  padding: 1rem 1.25rem 0.85rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 252, 246, 0.75);
}

.onecalc-shell-drawer__titles {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
}

.onecalc-shell-drawer__eyebrow {
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-weight: 700;
}

.onecalc-shell-drawer__title {
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--oc-color-ink);
  line-height: 1.25;
}

.onecalc-shell-drawer__subtitle {
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-shell-drawer__close {
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-muted);
  font-size: 1.2rem;
  line-height: 1;
  cursor: pointer;
  flex-shrink: 0;
}

.onecalc-shell-drawer__close:hover {
  color: var(--oc-color-warm);
  border-color: var(--oc-color-warm);
}

.onecalc-shell-drawer__body {
  padding: 1rem 1.25rem 1.25rem;
  overflow-y: auto;
  min-height: 0;
}

.onecalc-value-panel {
  display: grid;
  gap: var(--oc-space-2);
  padding: 1rem 1.25rem;
  border-radius: 16px;
  border: 2px solid rgba(36, 93, 90, 0.14);
  background: linear-gradient(180deg, #fffdf9, #f7f3ea);
  color: var(--oc-color-ink);
}

.onecalc-explore-shell__result-panel {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__result-panel > .onecalc-value-panel {
  padding: 0;
  border: none;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}

.onecalc-value-panel__header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__header-label {
  font-size: 1rem;
  font-weight: 700;
  color: var(--oc-color-ink);
}

.onecalc-value-panel__header-kind-tag {
  font-family: var(--oc-font-mono);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__primary {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__number-main,
.onecalc-value-panel__logical,
.onecalc-value-panel__error-code {
  font-family: var(--oc-font-mono);
  font-size: 2rem;
  font-weight: 700;
  color: var(--oc-color-accent);
}

.onecalc-value-panel__error-code {
  color: var(--oc-color-warm);
}

.onecalc-value-panel__text-content {
  font-family: var(--oc-font-mono);
  font-size: 1.1rem;
  padding: 0.6rem 0.8rem;
  background: rgba(255, 255, 255, 0.8);
  border-radius: 8px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  white-space: pre-wrap;
  word-break: break-word;
}

.onecalc-value-panel__text-meta,
.onecalc-value-panel__reference code,
.onecalc-value-panel__lambda dd code {
  font-family: var(--oc-font-mono);
  font-size: 0.8rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__text-warning,
.onecalc-value-panel__reference-warning {
  color: var(--oc-color-warm);
  font-weight: 700;
}

.onecalc-value-panel__error-surface {
  display: inline-block;
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  background: rgba(200, 141, 46, 0.16);
  color: #8d5f1e;
  font-size: 0.72rem;
  font-weight: 700;
}

.onecalc-value-panel__array-header {
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__array-grid {
  border-collapse: collapse;
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
}

.onecalc-value-panel__array-cell {
  padding: 0.25rem 0.5rem;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 255, 255, 0.8);
}

.onecalc-value-panel__kv-list,
.onecalc-value-panel__lambda {
  display: grid;
  gap: 0.25rem;
  margin: 0;
}

.onecalc-value-panel__kv-row {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--oc-space-2);
  font-size: 0.82rem;
}

.onecalc-value-panel__kv-row dt {
  color: var(--oc-color-muted);
  font-weight: 700;
  text-transform: uppercase;
  font-size: 0.7rem;
  letter-spacing: 0.04em;
}

.onecalc-value-panel__kv-row dd {
  margin: 0;
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
}

.onecalc-value-panel__rich-kvp-flag {
  margin-left: 0.35rem;
  padding: 0.1rem 0.35rem;
  border-radius: 999px;
  background: rgba(183, 101, 69, 0.12);
  color: #a75842;
  font-family: var(--oc-font-ui);
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
}

.onecalc-value-panel__presentation {
  display: flex;
  gap: 0.35rem;
  align-items: center;
  flex-wrap: wrap;
}

.onecalc-value-panel__presentation-badge {
  padding: 0.25rem 0.55rem;
  border-radius: 999px;
  background: rgba(36, 93, 90, 0.1);
  color: var(--oc-color-accent);
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.onecalc-value-panel__presentation-format-code {
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__effective-display {
  display: grid;
  gap: 0.35rem;
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-value-panel__effective-display-line {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__effective-display-label {
  color: var(--oc-color-muted);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.onecalc-value-panel__effective-display-value {
  font-family: var(--oc-font-mono);
  font-size: 1rem;
}

.onecalc-value-panel__effective-display-value[data-has-display="false"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-value-panel__effective-display-pipeline {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  min-width: 0;
}

.onecalc-value-panel__pipeline-chip {
  max-width: 100%;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.85);
  color: var(--oc-color-muted);
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  white-space: normal;
  overflow-wrap: anywhere;
  text-align: center;
}

.onecalc-value-panel__pipeline-chip[data-status="live"] {
  border-color: rgba(79, 123, 87, 0.4);
  background: rgba(79, 123, 87, 0.14);
  color: #33532c;
}

.onecalc-value-panel__pipeline-chip[data-status="not-implemented"] {
  border-color: rgba(183, 101, 69, 0.35);
  background: rgba(183, 101, 69, 0.1);
  color: #a75842;
  text-decoration: line-through;
}

.onecalc-value-panel__provenance {
  display: flex;
  gap: var(--oc-space-2);
  font-family: var(--oc-font-mono);
  font-size: 0.7rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__unevaluated {
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  border: 1px dashed rgba(31, 28, 23, 0.18);
  background: rgba(255, 255, 255, 0.6);
  color: var(--oc-color-muted);
  font-style: italic;
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
