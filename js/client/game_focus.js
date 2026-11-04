import { PHASE } from "./load_gate.js";

const CHAT_INPUT = 'input[data-ui="chat"]';
const TEXT_INPUT_TYPES = new Set(["text", "search", "email", "url", "tel", "password", "number"]);
const FOCUSABLE = "a[href], button, input, select, textarea, iframe, [contenteditable], [tabindex]";

const isLive = () => document.body.dataset.phase === PHASE.LIVE;

function isTextEntry(node) {
  if (node instanceof HTMLTextAreaElement || node?.isContentEditable) return true;
  return node instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(node.type);
}

function removeKeyboardFocus() {
  for (const node of document.querySelectorAll(FOCUSABLE)) {
    if (node.tabIndex >= 0 && !node.matches(CHAT_INPUT)) node.tabIndex = -1;
  }
}

export function installGameFocusGuard({ onArrow }) {
  const sweep = () => {
    if (isLive()) removeKeyboardFocus();
  };
  new MutationObserver(sweep).observe(document.body, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: ["tabindex", "data-phase"],
  });
  sweep();

  window.addEventListener("keydown", (e) => {
    if (!isLive() || !e.key.startsWith("Arrow") || isTextEntry(e.target)) return;
    e.preventDefault();
    e.stopPropagation();
    onArrow(e.key);
  }, true);

  window.addEventListener("mousedown", (e) => {
    if (isLive() && !isTextEntry(e.target)) e.preventDefault();
  }, true);

  window.addEventListener("pointerdown", () => {
    const active = document.activeElement;
    if (isLive() && active && active !== document.body && !isTextEntry(active)) active.blur();
  }, true);
}
