import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import * as model from './model.js';
import { setLanguage, getLanguage, message, errorMessage, t } from './i18n.js';

// Exercise the complete settings controller. Only browser/IPC boundaries are
// replaced; change, dialog, save and language handlers all come from production.
class Node {
  get value() { return this._value ?? ''; }
  set value(value) { this._value = this.type === 'color' ? String(value).toLowerCase() : String(value); }
  constructor() {
    this.value = ''; this.checked = false; this.disabled = false; this.open = false;
    this.dataset = {}; this.children = []; this.handlers = new Map(); this.scrollTop = 0;
    const classes = new Set();
    this.classList = { toggle(name, enabled) { if (enabled) classes.add(name); else classes.delete(name); }, contains: name => classes.has(name) };
    this.style = { setProperty() {} };
  }
  addEventListener(event, callback) { const handlers = this.handlers.get(event) ?? []; handlers.push(callback); this.handlers.set(event, handlers); }
  async dispatch(event) { for (const callback of this.handlers.get(event) ?? []) await callback({ preventDefault() {}, stopPropagation() {} }); }
  setAttribute(name, value) { this[name] = value; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = children; }
  showModal() { this.open = true; }
  close() { this.open = false; void this.dispatch('close'); }
  querySelectorAll() { return []; }
}

async function until(predicate) {
  for (let attempt = 0; attempt < 20 && !predicate(); attempt++) await new Promise(resolve => setImmediate(resolve));
  assert.ok(predicate(), 'Expected the settings controller to reach the IPC boundary');
}

async function harness(initial = {}, platform = 'macos') {
  setLanguage('zh-CN');
  const nodes = new Map();
  const node = id => { if (!nodes.has(id)) nodes.set(id, new Node()); return nodes.get(id); };
  const prefixes = ['Control', 'Alt', 'Shift', 'Super'].map(value => Object.assign(new Node(), { value }));
  const sides = ['left', 'right'].map(value => Object.assign(new Node(), { value }));
  const config = { prefix: ['Control'], apps: [], hint: { enabled: true, delay: 0.2, side: 'right', color: '#244b70', scale: 100, order: 'manual' }, language: 'zh-CN', ...initial };
  node('letter-color').type = 'color';
  let saved = structuredClone(config);
  let paused = false;
  const controls = { save: null, report: null };
  const calls = [];
  const document = {
    documentElement: new Node(), getElementById: node,
    querySelector(selector) { return sides.find(side => selector.includes(`[value=${side.value}]`) || selector.includes(':checked') && side.checked); },
    querySelectorAll(selector) {
      if (selector === '#prefix-options input:checked') return prefixes.filter(input => input.checked);
      if (selector === '#prefix-options input') return prefixes;
      if (selector === 'input[name=side]') return sides;
      if (selector === '[data-prefix-key]') return [];
      return selector.split(',').flatMap(part => part === '#prefix-options input' ? prefixes : part === 'input[name=side]' ? sides : part.startsWith('#') ? [node(part.slice(1))] : []);
    },
  };
  const status = () => ({ config: structuredClone(saved), platform, paused, config_path: '/test/config.json' });
  async function invoke(command, args) {
    calls.push({ command, args: structuredClone(args) });
    if (command === 'get_state') return status();
    if (command === 'set_settings_dirty') { if (controls.report) await controls.report(args); return; }
    if (command === 'save_config') { if (controls.save) await controls.save(args); saved = structuredClone(args.config); return status(); }
    if (command === 'set_language') { saved.language = args.language; return; }
    if (command === 'set_paused') { paused = args.paused; return status(); }
    if (command === 'get_autostart') return false;
    if (command === 'list_running') return [{ id: 'safari', name: 'Safari', path: '/Applications/Safari.app', letter: '' }];
    throw new Error(`Unexpected IPC command ${command}`);
  }
  const context = {
    ...model, document, structuredClone, crypto: { randomUUID: () => 'new-app' },
    t, setLanguage, getLanguage, message, errorMessage,
    element: () => new Node(), badge: () => new Node(), loadIcon() {}, translatePage() {}, initializeNavigation() {}, initializeUpdates: async () => {},
    window: { __TAURI__: { core: { invoke }, event: { listen: async () => {} }, app: { getVersion: async () => 'test' } } },
  };
  const source = readFileSync(new URL('./settings.js', import.meta.url), 'utf8').replace(/^import .*;\n/gm, '');
  await runInNewContext(`(async () => { ${source}\n globalThis.settings = { edit, save, changed, get dirty() { return dirty; }, get draft() { return draft; }, get state() { return state; }, flush: async () => { if (typeof settingsSync !== 'undefined') await settingsSync; } }; })()`, context);
  const settings = context.settings;
  await settings.flush();
  const reports = () => calls.filter(call => call.command === 'set_settings_dirty').map(call => call.args.dirty);
  return { node, settings, calls, controls, reports, config, async input(id, value) { node(id).value = String(value); await node(id).dispatch('input'); await settings.flush(); } };
}

test('clean settings explicitly enable shortcuts without closing the window', async () => {
  const h = await harness();
  assert.equal(h.settings.dirty, false);
  assert.deepEqual(h.reports(), [false]);
});

test('valid modifier order and color normalization do not create phantom edits', async () => {
  const h = await harness({ prefix: ['Shift', 'Control'], hint: { enabled: true, delay: 0.2, side: 'right', color: '#244B70', scale: 100, order: 'manual' } });
  assert.equal(h.settings.dirty, false);
  assert.deepEqual(h.reports(), [false]);
});

test('Windows settings use the same clean, edit and revert admission rules', async () => {
  const h = await harness({}, 'windows');
  assert.deepEqual(h.reports(), [false]);
  await h.input('hint-scale', 80);
  assert.equal(h.reports().at(-1), true);
  await h.input('hint-scale', 100);
  assert.equal(h.reports().at(-1), false);
});

test('edits block shortcuts with a prominent explanation and reverting resumes them', async () => {
  const h = await harness();
  await h.input('hint-scale', 80);
  assert.equal(h.settings.dirty, true);
  assert.equal(h.reports().at(-1), true);
  assert.match(h.node('save-status').textContent, /未保存.*暂停.*保存/);
  assert.equal(h.node('save-bar').classList.contains('save-pending'), true);
  await h.input('hint-scale', 100);
  assert.equal(h.settings.dirty, false);
  assert.equal(h.reports().at(-1), false);
  assert.equal(h.node('save-bar').classList.contains('save-pending'), false);
});

test('invalid edits stay blocked until restored to the saved value', async () => {
  const h = await harness();
  await h.input('hint-delay', '');
  assert.equal(h.reports().at(-1), true);
  await h.input('hint-delay', '0.2');
  assert.equal(h.settings.dirty, false);
  assert.equal(h.reports().at(-1), false);
});

test('uncommitted dialog input blocks shortcuts and cancelling clears only those changes', async () => {
  const h = await harness();
  h.settings.edit();
  await h.input('edit-name', 'New app');
  assert.equal(h.reports().at(-1), true);
  await h.node('cancel-edit').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, false);
  assert.equal(h.reports().at(-1), false);
  await h.input('hint-scale', 80);
  h.settings.edit();
  await h.input('edit-name', 'Another app');
  await h.node('cancel-edit').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, true);
});

test('successful save resumes shortcuts while failed save preserves the paused draft', async () => {
  const h = await harness();
  await h.input('hint-scale', 80);
  h.controls.save = async () => { throw new Error('Disk full'); };
  await h.node('save').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, true);
  assert.equal(h.reports().at(-1), true);
  assert.equal(h.node('hint-scale').value, '80');
  assert.match(h.node('save-status').textContent, /暂停/);
  h.controls.save = null;
  await h.node('save').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, false);
  assert.equal(h.reports().at(-1), false);
});

test('input made during an asynchronous save is retained and remains unsaved', async () => {
  const h = await harness();
  await h.input('hint-scale', 80);
  let finish; h.controls.save = () => new Promise(resolve => { finish = resolve; });
  const pending = h.node('save').dispatch('click');
  await until(() => finish);
  await h.input('hint-scale', 90);
  finish(); await pending; await h.settings.flush();
  assert.equal(h.settings.state.config.hint.scale, 80);
  assert.equal(h.node('hint-scale').value, '90');
  assert.equal(h.settings.dirty, true);
  assert.equal(h.reports().at(-1), true);
});

test('dirty notifications cannot complete out of order and leave shortcuts blocked', async () => {
  const h = await harness();
  let finish;
  h.controls.report = ({ dirty }) => dirty ? new Promise(resolve => { finish = resolve; }) : undefined;
  h.node('hint-scale').value = '80'; await h.node('hint-scale').dispatch('input');
  await until(() => finish);
  h.node('hint-scale').value = '100'; await h.node('hint-scale').dispatch('input');
  assert.deepEqual(h.reports(), [false, true]);
  finish(); await h.settings.flush();
  assert.deepEqual(h.reports(), [false, true, false]);
});

test('immediately saved language does not create unsaved edits or clear existing ones', async () => {
  const h = await harness();
  h.node('language').value = 'en'; await h.node('language').dispatch('change'); await h.settings.flush();
  assert.equal(h.settings.dirty, false);
  await h.input('hint-scale', 80);
  h.node('language').value = 'zh-CN'; await h.node('language').dispatch('change'); await h.settings.flush();
  assert.equal(h.settings.dirty, true);
  assert.equal(h.reports().at(-1), true);
});

test('import selections count as drafts and cancel restores the clean state', async () => {
  const h = await harness();
  await h.node('import').dispatch('click');
  await until(() => h.node('import-list').children[0]?.children.length === 4);
  const checkbox = h.node('import-list').children[0].children[0];
  checkbox.checked = true; await checkbox.dispatch('change'); await h.settings.flush();
  assert.equal(h.settings.dirty, true);
  assert.equal(h.reports().at(-1), true);
  await h.node('cancel-import').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, false);
  assert.equal(h.reports().at(-1), false);
});

test('saving the draft keeps an explicit manual pause active', async () => {
  const h = await harness();
  await h.node('pause').dispatch('click');
  await h.input('hint-scale', 80);
  await h.node('save').dispatch('click'); await h.settings.flush();
  assert.equal(h.settings.dirty, false);
  assert.equal(h.settings.state.paused, true);
  assert.match(h.node('save-status').textContent, /切换仍暂停/);
  assert.equal(h.calls.filter(call => call.command === 'set_paused').length, 1);
});
