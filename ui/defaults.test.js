import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('default green is consistent across UI, reset control and packaged icons', () => {
  const color = '#587a6b';
  for (const file of ['settings.css', 'overlay.css', 'app-icon.svg', 'settings.js', '../src-tauri/src/config.rs', '../tools/generate-icons.ps1']) {
    const source = readFileSync(new URL(file, import.meta.url), 'utf8');
    assert.ok(source.includes(color), file);
    assert.ok(!source.includes('#71899a'), file);
  }
  const rgb = color.slice(1).match(/../g).map(hex => parseInt(hex, 16) / 255).map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  const luminance = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
  assert.ok(1.05 / (luminance + 0.05) >= 4.5);
});
