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

test('studio signature uses the existing UI font without loading brand fonts', () => {
  const css = readFileSync(new URL('./settings.css', import.meta.url), 'utf8');
  assert.match(css, /\.studio-name\{font-family:inherit;/);
  assert.doesNotMatch(css, /@font-face|Studio Playfair|Studio Noto Serif|\.woff2/);
});

test('studio name matches the footer size and color in both languages, with a smaller motto', () => {
  const css = readFileSync(new URL('./settings.css', import.meta.url), 'utf8');
  const name = css.match(/\.studio-name\{([^}]+)\}/)[1];
  const motto = css.match(/\.studio-motto\{([^}]+)\}/)[1];
  assert.ok(name.includes('font-size:inherit'));
  assert.ok(name.includes('color:inherit'));
  assert.ok(motto.includes('font-size:9px'));
  assert.ok(motto.includes('color:inherit'));
  assert.doesNotMatch(css, /:lang\(en\) \.studio-name/);
  assert.match(css, /\.studio-signature\{margin-top:8px;/);
  assert.match(css, /\.sidebar-tail\{[^}]*font-size:10px;/);
  assert.match(css, /\.sidebar\{padding-bottom:18px\}/);
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

test('studio mark is decorative and uses the approved graphical master', () => {
  const html = readFileSync(new URL('./index.html', import.meta.url), 'utf8');
  const css = readFileSync(new URL('./settings.css', import.meta.url), 'utf8');
  const mark = readFileSync(new URL('./studio-mark.svg', import.meta.url), 'utf8');
  assert.match(html, /class="studio-mark" aria-hidden="true"/);
  assert.match(css, /\.studio-mark\{width:44px;height:52px;grid-row:1 \/ 3;background:currentColor;/);
  assert.match(css, /mask:url\('studio-mark.svg'\)/);
  assert.match(mark, /data-master="approved-v6"/);
  assert.equal((mark.match(/<path/g) || []).length, 15);
  assert.doesNotMatch(mark, /<text/);
});
