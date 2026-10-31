import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { hintApps } from './model.js';

test('native window visibility does not depend on a hidden webview animation event', () => {
  const css = readFileSync(new URL('./overlay.css', import.meta.url), 'utf8');
  const rail = css.match(/\.rail\{([^}]+)\}/)[1];
  assert.doesNotMatch(rail, /opacity\s*:\s*0(?:[;.}]|$)/);
  assert.doesNotMatch(rail, /visibility\s*:\s*hidden/);
  const runtime = readFileSync(new URL('../src-tauri/src/runtime.rs', import.meta.url), 'utf8');
  const reveal = runtime.slice(runtime.indexOf('if !visible && started.elapsed()'), runtime.indexOf('fn position_hint'));
  const show = reveal.indexOf('set_hint_visible(&overlay, true)');
  assert.ok(show >= 0 && show < reveal.indexOf('"hint-show"'));
  assert.match(runtime, /set_hint_visible\(&overlay, false\)/);
  assert.match(runtime, /overlay\.set_focusable\(false\)/);
});

test('saved scale reaches the whole rail while custom color and Mac material remain independent', async () => {
  const source = readFileSync(new URL('./overlay.js', import.meta.url), 'utf8').replace(/^import .*;$/gm, '');
  for (const [platform, scale, ratio] of [['MacIntel', 80, '0.8'], ['Win32', 150, '1.5'], ['MacIntel', undefined, '1']]) {
    const styles = new Map();
    const rail = { className: 'rail right', classList: { contains: () => false } };
    const items = { classList: { toggle() {} }, replaceChildren() {} };
    const prefix = {};
    const config = { prefix: ['Alt'], apps: [], hint: { side: 'left', color: '#81958a', order: 'manual', scale } };
    await runInNewContext(`(async () => { ${source} })()`, {
      document: { getElementById: id => ({ rail, items, prefix })[id], documentElement: { style: { setProperty: (key, value) => styles.set(key, value) } } },
      navigator: { platform }, hintApps, setTimeout, clearTimeout,
      window: { __TAURI__: { event: { listen: async () => {} }, core: { invoke: async () => ({ config }) } } },
    });
    assert.equal(styles.get('--overlay-scale'), ratio);
    assert.equal(styles.get('--accent'), '#81958a');
    assert.equal(rail.className.split(/\s+/).includes('native-material'), platform === 'MacIntel');
    assert.equal(prefix.textContent, platform === 'MacIntel' ? '⌥' : 'Alt');
  }
});

test('whole rail scales once inside the resized native viewport and reveals native glass only on Mac', () => {
  const css = readFileSync(new URL('./overlay.css', import.meta.url), 'utf8');
  const rail = css.match(/\.rail\{([^}]+)\}/)[1];
  assert.match(rail, /width:calc\(100% \/ var\(--overlay-scale\)\)/);
  assert.match(rail, /height:calc\(100% \/ var\(--overlay-scale\)\)/);
  assert.match(rail, /transform:scale\(var\(--overlay-scale\)\)/);
  assert.match(rail, /transform-origin:top left/);
  assert.match(rail, /backdrop-filter:blur/);
  const material = css.match(/\.rail\.native-material\{([^}]+)\}/)[1];
  assert.match(material, /background:transparent/);
  assert.match(material, /border(?:-color)?:[^;]*(?:transparent|0|none)/);
  assert.match(material, /backdrop-filter:none/);
});

test('flow keeps the spine signature without animating layout and obeys reduced motion', () => {
  const css = readFileSync(new URL('./overlay.css', import.meta.url), 'utf8');
  const flow = css.slice(css.indexOf('@keyframes flow'), css.indexOf('@keyframes connector'));
  assert.match(flow, /transform:/);
  assert.match(flow, /opacity:/);
  assert.doesNotMatch(flow, /(?:top|bottom|height|width):/);
  assert.match(css, /\.shown \.spine:after\{animation:flow/);
  assert.match(css, /@media\(prefers-reduced-motion:reduce\)[\s\S]*animation:none/);
});
