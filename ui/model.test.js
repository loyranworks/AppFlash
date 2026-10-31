import test from 'node:test';
import assert from 'node:assert/strict';
import { addImports, normalizeLetter, shortcutWarnings, glyphs, hintApps, moveApp } from './model.js';

test('hint sorting excludes unbound apps and preserves manual order', () => {
  const apps = [{ id: 'c', letter: 'C' }, { id: 'off', letter: '' }, { id: 'a', letter: 'A' }];
  assert.deepEqual(hintApps(apps, 'manual').map(app => app.id), ['c', 'a']);
  assert.deepEqual(hintApps(apps, 'alphabetical').map(app => app.id), ['a', 'c']);
  assert.deepEqual(apps.map(app => app.id), ['c', 'off', 'a']);
});

test('manual moves retain entries and handle first, last and missing IDs', () => {
  const apps = [{ id: 'a', path: '/one' }, { id: 'b', path: '/two' }, { id: 'c', path: '/three' }];
  assert.deepEqual(moveApp(apps, 'b', -1).map(app => app.id), ['b', 'a', 'c']);
  assert.deepEqual(moveApp(apps, 'b', 1).map(app => app.id), ['a', 'c', 'b']);
  assert.deepEqual(moveApp(apps, 'a', -1), apps);
  assert.deepEqual(moveApp(apps, 'c', 1), apps);
  assert.deepEqual(moveApp(apps, 'missing', 1), apps);
  assert.deepEqual(apps.map(app => app.id), ['a', 'b', 'c']);
});

test('paths rather than names identify imported apps', () => {
  const apps = [{ id: 'old', name: 'Chrome', path: 'C:\\one\\chrome.exe', letter: 'B' }];
  const imported = addImports(apps, [
    { name: 'Chrome', path: 'c:/one/chrome.exe', letter: 'A' },
    { name: 'Chrome', path: 'C:\\two\\chrome.exe', letter: 'C' },
  ], 'windows', () => 'new');
  assert.equal(imported.length, 2);
  assert.equal(imported[0].letter, 'B');
  assert.equal(Object.hasOwn(imported[0], 'args'), false);
  assert.equal(imported[1].letter, 'C');
});

test('unassigned imports and more than 26 apps are retained', () => {
  const source = Array.from({ length: 30 }, (_, index) => ({ name: 'app', path: `/Applications/${index}.app`, letter: '' }));
  let counter = 0;
  const imported = addImports([], source, 'macos', () => `id-${counter++}`);
  assert.equal(imported.length, 30);
  assert(imported.every(app => app.letter === '' && !Object.hasOwn(app, 'args')));
});

test('letters are normalized, invalid input rejected, all glyphs available', () => {
  assert.equal(normalizeLetter('a'), 'A');
  assert.equal(normalizeLetter(''), '');
  assert.throws(() => normalizeLetter('ab'));
  assert.throws(() => normalizeLetter('<'));
  assert.equal(Object.keys(glyphs).length, 26);
});

test('single prefixes save without generic warnings; system-key warnings remain', () => {
  for (const platform of ['windows', 'macos']) {
    for (const modifier of ['Alt', 'Control', 'Shift']) assert.deepEqual(shortcutWarnings([modifier], platform), []);
  }
  assert(shortcutWarnings(['Super'], 'macos').length > 0);
  assert(shortcutWarnings(['Super'], 'windows').length > 0);
  assert.equal(shortcutWarnings(['Control', 'Alt'], 'windows').length, 0);
});
