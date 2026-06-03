export type TranslationMessages = Record<string, string>;

const ATTRIBUTES_TO_LOCALIZE = ['aria-label', 'placeholder', 'title', 'alt'];
const TEXT_SKIP_SELECTOR = 'script, style, code, pre, textarea, input, [data-i18n-skip]';
const ATTRIBUTE_SKIP_SELECTOR = 'script, style, code, pre, [data-i18n-skip]';

interface LocalizedValueState {
  original: string;
  translated: string;
}

const textNodeOriginals = new WeakMap<Text, LocalizedValueState>();
const attrOriginals = new WeakMap<Element, Map<string, LocalizedValueState>>();

function shouldSkip(node: Node): boolean {
  const element = node.nodeType === Node.ELEMENT_NODE ? node as Element : node.parentElement;
  return Boolean(element?.closest(TEXT_SKIP_SELECTOR));
}

function applyMessages(value: string, messages: TranslationMessages): string {
  if (!value.trim()) return value;
  const exact = messages[value];
  if (exact) return exact;

  let translated = value;
  const keys = Object.keys(messages).sort((left, right) => right.length - left.length);
  for (const key of keys) {
    if (key && translated.includes(key)) {
      translated = translated.split(key).join(messages[key]);
    }
  }
  return translated;
}

function resolveOriginal(current: string, state: LocalizedValueState | undefined): string {
  if (!state) return current;
  return current === state.translated ? state.original : current;
}

function localizeTextNode(node: Text, messages: TranslationMessages, enabled: boolean) {
  if (shouldSkip(node)) return;
  const current = node.nodeValue ?? '';
  const state = textNodeOriginals.get(node);
  const original = resolveOriginal(current, state);
  const translated = enabled ? applyMessages(original, messages) : original;
  textNodeOriginals.set(node, { original, translated });
  if (current !== translated) node.nodeValue = translated;
}

function localizeElementAttributes(element: Element, messages: TranslationMessages, enabled: boolean) {
  if (element.closest(ATTRIBUTE_SKIP_SELECTOR)) return;
  let originals = attrOriginals.get(element);
  if (!originals) {
    originals = new Map<string, LocalizedValueState>();
    attrOriginals.set(element, originals);
  }

  for (const attr of ATTRIBUTES_TO_LOCALIZE) {
    if (!element.hasAttribute(attr)) continue;
    const current = element.getAttribute(attr) ?? '';
    const state = originals.get(attr);
    const original = resolveOriginal(current, state);
    const translated = enabled ? applyMessages(original, messages) : original;
    originals.set(attr, { original, translated });
    if (current !== translated) element.setAttribute(attr, translated);
  }
}

function walk(root: Node, visit: (node: Node) => void) {
  visit(root);
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT);
  let next = walker.nextNode();
  while (next) {
    visit(next);
    next = walker.nextNode();
  }
}

export function localizeDom(root: ParentNode, messages: TranslationMessages, enabled: boolean) {
  walk(root as Node, (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      localizeTextNode(node as Text, messages, enabled);
    } else if (node.nodeType === Node.ELEMENT_NODE) {
      localizeElementAttributes(node as Element, messages, enabled);
    }
  });
}

const OBSERVER_OPTIONS: MutationObserverInit = {
  childList: true,
  subtree: true,
  characterData: true,
  attributes: true,
  attributeFilter: ATTRIBUTES_TO_LOCALIZE,
};

function scheduleOnce(callback: () => void): () => void {
  let scheduled = false;
  return () => {
    if (scheduled) return;
    scheduled = true;
    const run = () => {
      scheduled = false;
      callback();
    };
    if (typeof requestAnimationFrame === 'function') {
      requestAnimationFrame(run);
    } else {
      setTimeout(run, 0);
    }
  };
}

export function observeLocalization(root: HTMLElement, messages: TranslationMessages, enabled: boolean): MutationObserver {
  const observer = new MutationObserver(() => scheduleApply());
  const applySafely = () => {
    observer.disconnect();
    localizeDom(root, messages, enabled);
    observer.observe(root, OBSERVER_OPTIONS);
  };
  const scheduleApply = scheduleOnce(applySafely);
  observer.observe(root, OBSERVER_OPTIONS);
  return observer;
}

export function translateString(value: string, messages: TranslationMessages, enabled: boolean): string {
  return enabled ? applyMessages(value, messages) : value;
}
