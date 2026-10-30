import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('native window visibility does not depend on a hidden webview animation event', () => {
  const css = readFileSync(new URL('./overlay.css', import.meta.url), 'utf8');
  const rail = css.match(/\.rail\{([^}]+)\}/)[1];
  assert.doesNotMatch(rail, /opacity\s*:\s*0(?:[;.}]|$)/);
  assert.doesNotMatch(rail, /visibility\s*:\s*hidden/);
  const runtime = readFileSync(new URL('../src-tauri/src/runtime.rs', import.meta.url), 'utf8');
  const reveal = runtime.slice(runtime.indexOf('if !visible && started.elapsed()'), runtime.indexOf('fn position_hint'));
  assert.ok(reveal.indexOf('overlay.show()') < reveal.indexOf('"hint-show"'));
  assert.match(runtime, /overlay\.hide\(\)/);
  assert.match(runtime, /overlay\.set_focusable\(false\)/);
});
