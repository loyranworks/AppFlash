import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { setLanguage, t } from './i18n.js';

test('studio signature follows language without replacing the application slogan', () => {
  const html = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  assert.match(html, /APP \/ FLASH[\s\S]*?class="studio-name" data-i18n="落雁工作室"/);
  assert.match(html, /data-i18n="巧思成形，琢而至美。"/);
  setLanguage('en');
  assert.equal(t('落雁工作室'), 'loyran works.');
  assert.equal(t('巧思成形，琢而至美。'), 'Make Ideas Work Beautifully');
  assert.equal(t('你的应用，一闪即达。'), 'Your apps, in a flash.');
  setLanguage('zh-CN');
  assert.equal(t('落雁工作室'), '落雁工作室');
  assert.equal(t('巧思成形，琢而至美。'), '巧思成形，琢而至美。');
});

test('brand fonts are local, compact and retain their licenses', () => {
  const css = readFileSync(new URL('./settings.css', import.meta.url), 'utf8');
  for (const file of ['playfair-display-studio.woff2', 'noto-serif-sc-studio.woff2']) {
    const bytes = readFileSync(new URL('./fonts/' + file, import.meta.url));
    assert.equal(bytes.subarray(0, 4).toString(), 'wOF2');
    assert.ok(bytes.length < 100000);
    assert.ok(css.includes(file));
  }
  for (const file of ['Playfair-OFL.txt', 'NotoSerifSC-OFL.txt']) {
    assert.match(readFileSync(new URL('./fonts/' + file, import.meta.url), 'utf8'), /SIL OPEN FONT LICENSE/);
  }
  assert.match(css, /:lang\(en\) \.studio-name/);
});

test('native About includes localized studio credits and the actual package version', () => {
  const source = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  assert.match(source, /PredefinedMenuItem::about/);
  assert.match(source, /About AppFlash/);
  assert.match(source, /关于 AppFlash/);
  assert.match(source, /loyran works\.\\nMake Ideas Work Beautifully/);
  assert.match(source, /落雁工作室\\n巧思成形，琢而至美。/);
  assert.match(source, /version: Some\(app\.package_info\(\)\.version\.to_string\(\)\)/);
  assert.match(source, /comments: Some\(studio\.into\(\)\)/);
  assert.match(source, /credits: Some\(studio\.into\(\)\)/);
  assert.match(source, /Menu::with_items\(app, &\[&settings, &pause, &separator, &about, &quit\]\)/);
});
