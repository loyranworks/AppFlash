import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { message, errorMessage, setLanguage } from './i18n.js';

function exportHarness(invoke) {
  const source = readFileSync(new URL('./settings.js', import.meta.url), 'utf8');
  const start = source.indexOf('async function exportLogs()');
  assert.ok(start >= 0, 'Settings provides a local diagnostic export action');
  const nodes = {
    'export-logs': { disabled: false, addEventListener(event, handler) { this[event] = handler; } },
    'export-status': { dataset: {}, textContent: '' },
    error: { textContent: 'existing validation error' },
  };
  const draft = { apps: [{ name: 'Unsaved application' }], hint: { scale: 80 } };
  const context = { byId: id => nodes[id], invoke, message, errorMessage, draft, dirty: true, Date: class { getTimezoneOffset() { return -480; } } };
  runInNewContext(source.slice(start, source.indexOf('initializeNavigation();', start)), context);
  return { nodes, context, draft };
}

test('export passes the current local time-zone offset to the native snapshot', async () => {
  let received;
  const { nodes } = exportHarness(async (command, args) => { received = { command, args }; return null; });
  await nodes['export-logs'].click();
  assert.equal(received.command, 'export_logs');
  assert.equal(received.args?.timezoneOffsetMinutes, -480);
});

test('export disables duplicate actions, reports the saved path and preserves unsaved settings', async () => {
  let resolve; let calls = 0;
  const { nodes, context, draft } = exportHarness(command => {
    assert.equal(command, 'export_logs'); calls++;
    return new Promise(done => { resolve = done; });
  });
  const pending = nodes['export-logs'].click();
  assert.equal(nodes['export-logs'].disabled, true);
  await nodes['export-logs'].click();
  assert.equal(calls, 1);
  resolve('/tmp/AppFlash 诊断.txt'); await pending;
  assert.equal(nodes['export-logs'].disabled, false);
  assert.match(nodes['export-status'].textContent, /\/tmp\/AppFlash 诊断.txt/);
  assert.equal(nodes.error.textContent, 'existing validation error');
  assert.equal(context.draft, draft);
  assert.equal(context.dirty, true);
  assert.equal(draft.apps[0].name, 'Unsaved application');
});

test('cancel clears pending export feedback without reporting a failure', async () => {
  const { nodes } = exportHarness(async () => null);
  await nodes['export-logs'].click();
  assert.equal(nodes['export-logs'].disabled, false);
  assert.equal(nodes['export-status'].textContent, '');
  assert.equal(nodes.error.textContent, 'existing validation error');
});

test('export failure stays beside the button, is localized and allows retry', async () => {
  setLanguage('en');
  try {
    const { nodes } = exportHarness(async () => { throw '诊断日志暂不可用，请稍后重试。'; });
    await nodes['export-logs'].click();
    assert.equal(nodes['export-logs'].disabled, false);
    assert.equal(nodes['export-status'].textContent, 'Diagnostic logs are temporarily unavailable. Try again shortly.');
    assert.equal(nodes.error.textContent, 'existing validation error');
    assert.equal(nodes['export-status'].className, 'error');
  } finally { setLanguage('zh-CN'); }
});
