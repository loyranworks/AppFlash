import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

test('brand previews selected color, pauses in gray and restores without altering accent', () => {
  const source = readFileSync(new URL('./settings.js', import.meta.url), 'utf8');
  const start = source.indexOf('function updateTheme()');
  const end = source.indexOf('function renderApps()', start);
  assert.ok(start >= 0 && end > start);
  const nodes = { 'letter-color': { value: '#81958a' }, 'hint-enabled': { checked: false } };
  const styles = new Map();
  const state = { paused: false, platform: 'windows' };
  const context = {
    state, byId: id => nodes[id] ??= {}, prefix: () => [], shortcutWarnings: () => [],
    document: { documentElement: { style: { setProperty: (key, value) => styles.set(key, value) } }, querySelectorAll: () => [] },
  };
  runInNewContext(source.slice(start, end) + '\nupdateTheme();', context);
  assert.equal(styles.get('--brand-color'), '#81958a');
  assert.notEqual(nodes['letter-color'].disabled, true);
  state.paused = true;
  context.updateTheme();
  assert.equal(styles.get('--brand-color'), '#343840');
  assert.equal(styles.get('--accent'), '#81958a');
  state.paused = false;
  context.updateTheme();
  assert.equal(styles.get('--brand-color'), '#81958a');
  const icon = readFileSync(new URL('./app-icon.svg', import.meta.url), 'utf8');
  assert.match(icon, /M70 26 43 68H62L54 103 89 56H69L78 26Z/);
  assert.doesNotMatch(icon, /<circle/);
});
