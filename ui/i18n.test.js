import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { english, setLanguage, t, message, errorMessage, translatePage } from './i18n.js';
import { selectSection } from './navigation.js';

test('all static settings text has English translations', () => {
  const html = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  for (const match of html.matchAll(/data-i18n(?:-placeholder|-label)?="([^"]+)"/g)) assert.ok(english[match[1]], match[1]);
  setLanguage('en');
  assert.equal(t('当前版本 {version}', { version: '0.1.0' }), 'Version 0.1.0');
  assert.throws(() => setLanguage('invalid'));
  setLanguage('zh-CN');
});

test('language switches retranslate status and clear stale error metadata', () => {
  const previous = globalThis.document;
  const node = { dataset: {}, textContent: '' };
  const root = { querySelectorAll: selector => selector === '[data-i18n]' && node.dataset.i18n ? [node] : selector === '[data-error-source]' && node.dataset.errorSource ? [node] : [] };
  globalThis.document = { documentElement: {} };
  try {
    errorMessage(node, '语言必须是 zh-CN 或 en');
    setLanguage('en'); translatePage(root);
    assert.equal(node.textContent, 'Language must be zh-CN or en');
    message(node, '语言已保存');
    setLanguage('zh-CN'); translatePage(root);
    assert.equal(node.textContent, '语言已保存');
    assert.equal(node.dataset.errorSource, undefined);
  } finally { globalThis.document = previous; setLanguage('zh-CN'); }
});

test('sidebar changes only visibility and preserves edits', () => {
  const panels = ['apps', 'shortcuts', 'hint', 'general'].map(panel => ({ dataset: { panel }, hidden: false, value: 'unsaved' }));
  const buttons = panels.map(panel => ({ dataset: { section: panel.dataset.panel }, setAttribute(name, value) { this[name] = value; }, removeAttribute(name) { delete this[name]; } }));
  const title = { dataset: {} }; const description = { dataset: {} };
  const root = { querySelectorAll: selector => selector === '[data-panel]' ? panels : buttons, getElementById: id => id === 'page-title' ? title : description };
  selectSection(root, 'general');
  assert.deepEqual(panels.map(panel => panel.hidden), [true, true, true, false]);
  assert.equal(buttons[3]['aria-current'], 'page');
  selectSection(root, 'invalid');
  assert.equal(panels[3].hidden, false);
  assert.ok(panels.every(panel => panel.value === 'unsaved'));
});
