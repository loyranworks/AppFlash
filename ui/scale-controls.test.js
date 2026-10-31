import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import * as model from './model.js';

test('range, number and reset keep one saved percentage; invalid edits cannot be saved', () => {
  const source = readFileSync(new URL('./settings.js', import.meta.url), 'utf8');
  const nodes = Object.fromEntries(['hint-scale', 'hint-scale-range', 'reset-scale'].map(id => [id, { value: '100', listeners: {}, addEventListener(event, listener) { this.listeners[event] = listener; } }]));
  const start = source.indexOf('function updateScale(');
  const end = source.indexOf("byId('autostart')", start);
  assert.ok(start >= 0 && end > start, 'scale controls are initialized with settings');
  let changes = 0;
  runInNewContext(source.slice(start, end), { byId: id => nodes[id], normalizeHintScale: model.normalizeHintScale, changed: () => changes++ });
  nodes['hint-scale-range'].value = '80';
  nodes['hint-scale-range'].listeners.input();
  assert.equal(nodes['hint-scale'].value, '80');
  nodes['hint-scale'].value = '150';
  nodes['hint-scale'].listeners.input();
  assert.equal(nodes['hint-scale-range'].value, '150');
  nodes['hint-scale'].value = '80.5';
  nodes['hint-scale'].listeners.input();
  assert.equal(nodes['hint-scale-range'].value, '150');
  assert.throws(() => model.normalizeHintScale(nodes['hint-scale'].value));
  nodes['reset-scale'].listeners.click();
  assert.equal(nodes['hint-scale'].value, '100');
  assert.equal(nodes['hint-scale-range'].value, '100');
  assert.equal(changes, 4);
});

test('saving carries scale with existing bindings and custom color, and rejects invalid number edits', () => {
  const source = readFileSync(new URL('./settings.js', import.meta.url), 'utf8');
  const start = source.indexOf('function collectConfig()');
  const end = source.indexOf('async function save(', start);
  const nodes = { 'hint-scale': { value: '80' }, 'hint-delay': { value: '0.5' }, 'hint-enabled': { checked: true }, 'letter-color': { value: '#81958a' }, 'hint-order': { value: 'manual' } };
  const context = {
    byId: id => nodes[id], draft: { theme_revision: 1, hint: {}, apps: [{ id: 'one', letter: 'A', path: '/Applications/Example.app' }] },
    structuredClone, getLanguage: () => 'en', prefix: () => ['Alt'], normalizeHintScale: model.normalizeHintScale, t: value => value,
    document: { querySelector: selector => { assert.equal(selector, 'input[name=side]:checked'); return { value: 'left' }; } },
  };
  runInNewContext(source.slice(start, end), context);
  const config = context.collectConfig();
  assert.equal(config.hint.scale, 80);
  assert.equal(config.hint.color, '#81958a');
  assert.equal(config.apps[0].letter, 'A');
  assert.equal(config.prefix[0], 'Alt');
  assert.equal(config.language, 'en');
  assert.equal(config.theme_revision, 1);
  for (const value of ['', '49', '150.5', '151']) {
    nodes['hint-scale'].value = value;
    assert.throws(() => context.collectConfig());
  }
});
