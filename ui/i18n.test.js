import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { english, setLanguage, t, message, errorMessage, errorText, translatePage } from './i18n.js';
import { selectSection, initializeNavigation } from './navigation.js';

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
  const panels = ['general', 'apps', 'hint'].map(panel => ({ dataset: { panel }, hidden: false, value: 'unsaved' }));
  const buttons = panels.map(panel => ({ dataset: { section: panel.dataset.panel }, setAttribute(name, value) { this[name] = value; }, removeAttribute(name) { delete this[name]; }, addEventListener() {} }));
  const title = { dataset: {} }; const description = { dataset: {} };
  const root = { querySelectorAll: selector => selector === '[data-panel]' ? panels : buttons, getElementById: id => id === 'page-title' ? title : description };
  initializeNavigation(root);
  assert.deepEqual(panels.map(panel => panel.hidden), [false, true, true]);
  assert.equal(buttons[0]['aria-current'], 'page');
  selectSection(root, 'invalid');
  assert.equal(panels[0].hidden, false);
  assert.ok(panels.every(panel => panel.value === 'unsaved'));
});

test('General comes first and Prefix controls share the application panel', () => {
  const html = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  assert.equal([...html.matchAll(/data-section="([^"]+)"/g)][0][1], 'general');
  const apps = html.slice(html.indexOf('id="panel-apps"'), html.indexOf('id="panel-hint"'));
  assert.match(apps, /id="prefix-options"/);
  assert.match(apps, /id="app-list"/);
  assert.doesNotMatch(html, /data-panel="shortcuts"/);
  assert.match(html, /界面语言 \/ Language/);
});

// These errors originate in the native switch path, including full-screen departure.
test('native switch recovery guidance is translated for the active interface language', () => {
  setLanguage('en');
  try {
    assert.equal(errorText('未能切离目标全屏桌面；请保持另一个应用窗口可用后重试，目标应用尚未收起。'), 'Could not leave the target fullscreen desktop. Keep another app window available and try again. The target app is still open.');
    assert.equal(errorText('目标应用暂时没有响应，请在桌面切换完成后重试。'), 'The target app is temporarily unresponsive. Try again after the desktop switch finishes.');
  } finally { setLanguage('zh-CN'); }
});

test('native log export failures translate the action while preserving operating-system details', () => {
  setLanguage('en');
  try {
    for (const [source, expected] of [
      ['此窗口不能导出日志。', 'This window cannot export logs.'],
      ['无法读取导出位置：invalid path', 'Cannot read export location: invalid path'],
      ['导出任务未完成：cancelled worker', 'Log export did not complete: cancelled worker'],
      ['导出日志失败：Permission denied (os error 13)', 'Log export failed: Permission denied (os error 13)'],
    ]) assert.equal(errorText(source), expected);
  } finally { setLanguage('zh-CN'); }
});


test('partial registration warning translates guidance while preserving key and working count', () => {
  setLanguage('en');
  try {
    const result=errorText('部分快捷键未能注册；2 个可用快捷键仍可切换。请关闭占用程序或修改 Prefix / 字母后保存。快捷键 Alt+KeyF 注册失败：HotKey already registered');
    assert.match(result, /2 working shortcuts remain available/);
    assert.match(result, /Alt\+KeyF/);
    assert.match(result, /registration failed/);
    assert.doesNotMatch(result, /[\u4e00-\u9fff]/);
    const blocked=errorText('所有已配置的快捷键均注册失败，快捷切换已暂停。请关闭占用程序或修改 Prefix / 字母后保存。');
    assert.match(blocked, /Switching is paused/);
    assert.doesNotMatch(blocked, /[\u4e00-\u9fff]/);
  } finally { setLanguage('zh-CN'); }
});


test('OS conflict reports retain the target and key without inventing the external owner', () => {
  setLanguage('en');
  try {
    const result=errorText('AppFlash → Firefox [Alt+F] 与系统中已注册的同组合键冲突；占用程序未知（系统未提供名称）。系统详情：HotKey already registered: Alt+KeyF');
    assert.match(result,/Firefox \[Alt\+F\]/);
    assert.match(result,/owning program is unknown/);
    assert.match(result,/HotKey already registered/);
    assert.doesNotMatch(result,/[\u4e00-\u9fff]/);
  } finally { setLanguage('zh-CN'); }
});
