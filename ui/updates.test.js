import test from 'node:test';
import assert from 'node:assert/strict';
import { downloadProgress, initializeUpdates } from './updates.js';

test('updates require confirmation, block dirty settings and recover from install failure', async context => {
  const previousDocument = globalThis.document;
  const nodes = new Map();
  const main = { inert: false };
  const events = new Map();
  const timers = [];
  const calls = [];
  let dirty = true;
  function node(id) {
    if (!nodes.has(id)) nodes.set(id, {
      dataset: {}, textContent: '', disabled: false, hidden: false, open: false,
      handlers: new Map(),
      addEventListener(name, handler) { this.handlers.set(name, handler); },
      showModal() { this.open = true; }, close() { this.open = false; },
    });
    return nodes.get(id);
  }
  globalThis.document = { getElementById: node, querySelector: () => main };
  context.after(() => { globalThis.document = previousDocument; });
  context.mock.method(globalThis, 'setTimeout', (handler, delay) => { timers.push({ handler, delay }); });
  context.mock.method(globalThis, 'setInterval', (handler, delay) => { timers.push({ handler, delay }); });
  await initializeUpdates({
    version: '0.1.0', hasUnsavedChanges: () => dirty,
    listen: async (name, handler) => { events.set(name, handler); },
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === 'check_update') return { version: '0.1.1', notes: '<script>untrusted notes</script>' };
      assert.equal(main.inert, true);
      throw new Error('signature rejected');
    },
  });
  assert.deepEqual(timers.map(timer => timer.delay), [3000, 21600000]);
  await timers[0].handler();
  assert.equal(calls.length, 1);
  assert.equal(node('install-update').hidden, false);
  assert.equal(node('update-notes').textContent, '<script>untrusted notes</script>');
  node('install-update').handlers.get('click')();
  assert.equal(node('update-dialog').open, false);
  assert.match(node('update-status').textContent, /保存/);
  dirty = false;
  node('install-update').handlers.get('click')();
  assert.equal(node('update-dialog').open, true);
  await timers[1].handler();
  assert.equal(calls.length, 1);
  events.get('update-progress')({ payload: { received: 10, total: 100 } });
  assert.equal(node('update-status').textContent, '正在下载 10%');
  await node('confirm-update').handlers.get('click')();
  assert.deepEqual(calls[1], { command: 'install_update', args: { version: '0.1.1' } });
  assert.equal(main.inert, false);
  assert.equal(node('check-update').disabled, false);
  assert.match(node('update-status').textContent, /signature rejected/);
});

test('download progress supports known and missing content lengths', () => {
  assert.equal(downloadProgress(50, 100), '正在下载 50%');
  assert.equal(downloadProgress(120, 100), '正在下载 100%');
  assert.equal(downloadProgress(1048576, null), '正在下载 1.0 MB');
  assert.equal(downloadProgress(0, 0), '正在下载 0.0 MB');
});
