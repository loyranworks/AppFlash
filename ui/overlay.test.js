import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { hintApps } from './model.js';
import * as i18n from './i18n.js';

test('native window visibility does not depend on a hidden webview animation event', () => {
  const css = readFileSync(new URL('./overlay.css', import.meta.url), 'utf8');
  const rail = css.match(/\.rail\{([^}]+)\}/)[1];
  assert.doesNotMatch(rail, /opacity\s*:\s*0(?:[;.}]|$)/);
  assert.doesNotMatch(rail, /visibility\s*:\s*hidden/);
  const runtime = readFileSync(new URL('../src-tauri/src/runtime.rs', import.meta.url), 'utf8');
  const presentation = runtime.indexOf('let presentation =');
  assert.ok(presentation >= 0, 'overlay presentation determines when the native window is revealed');
  const reveal = runtime.slice(presentation, runtime.indexOf('fn position_hint'));
  const show = reveal.indexOf('set_hint_visible(&overlay, true)');
  assert.ok(show >= 0 && show < reveal.indexOf('"hint-show"'));
  assert.match(runtime, /set_hint_visible\(&overlay, false\)/);
  assert.match(runtime, /overlay\.set_focusable\(false\)/);
});

function fakeNode() {
  const node = { className: '', children: [], dataset: {}, hidden: false, textContent: '', title: '', attributes: {},
    append(...children) { this.children.push(...children); }, replaceChildren(...children) { this.children = children; },
    setAttribute(name, value) { this.attributes[name] = value; },
  };
  node.classList = {
    contains: name => node.className.split(/\s+/).includes(name),
    toggle(name, force) { const enabled = force ?? !this.contains(name); this[enabled ? 'add' : 'remove'](name); return enabled; },
    add(...names) { node.className = [...new Set([...node.className.split(/\s+/).filter(Boolean), ...names])].join(' '); },
    remove(...names) { node.className = node.className.split(/\s+/).filter(name => !names.includes(name)).join(' '); },
  };
  Object.defineProperty(node, 'innerHTML', { set() { throw new Error('Untrusted text must not be parsed as HTML'); } });
  return node;
}

async function overlayHarness({ platform = 'MacIntel', scale = 100, initialNotice = null } = {}) {
  const source = readFileSync(new URL('./overlay.js', import.meta.url), 'utf8').replace(/^import .*;$/gm, '');
  const styles = new Map(); const events = new Map(); const timeouts = [];
  const elements = Object.fromEntries(['rail', 'items', 'prefix', 'switch-notice', 'notice-title', 'notice-message'].map(id => [id, fakeNode()]));
  elements.rail.className = 'rail right'; elements['switch-notice'].hidden = true;
  const config = { language: 'zh-CN', prefix: ['Alt'], apps: [{ id: 'one', name: 'Example', path: '/Example.app', letter: 'A' }], hint: { side: 'left', color: '#81958a', order: 'manual', scale } };
  await runInNewContext(`(async () => { ${source} })()`, {
    document: { getElementById: id => elements[id], documentElement: { style: { setProperty: (key, value) => styles.set(key, value) } } },
    navigator: { platform }, hintApps, ...i18n,
    element: (tag, className, text) => Object.assign(fakeNode(), { className: className ?? '', textContent: text ?? '' }), badge: () => fakeNode(), loadIcon() {},
    setTimeout: (...args) => { timeouts.push(args); }, clearTimeout() {},
    window: { __TAURI__: { event: { listen: async (name, handler) => events.set(name, handler) }, core: { invoke: async command => {
      assert.equal(command, 'get_state');
      for (const name of ['config-changed', 'hint-show', 'hint-hide', 'switch-notice', 'app-hit']) assert.ok(events.has(name), `${name} is ready before reading initial state`);
      return { config, notice: initialNotice };
    } } } },
  });
  return { elements, styles, events, config, timeouts };
}

test('saved scale reaches the whole rail while custom color and Mac material remain independent', async () => {
  for (const [platform, scale, ratio] of [['MacIntel', 80, '0.8'], ['Win32', 150, '1.5'], ['MacIntel', undefined, '1']]) {
    const { elements, styles } = await overlayHarness({ platform, scale });
    assert.equal(styles.get('--overlay-scale'), ratio);
    assert.equal(styles.get('--accent'), '#81958a');
    assert.equal(elements.rail.classList.contains('native-material'), platform === 'MacIntel');
    assert.equal(elements.prefix.textContent, platform === 'MacIntel' ? '⌥' : 'Alt');
  }
});

test('switch notices replace safely, preserve application rows and rely on backend clearing', async () => {
  const { elements, events, timeouts } = await overlayHarness();
  assert.ok(events.has('switch-notice'), 'overlay subscribes to switch failure feedback');
  const row = elements.items.children[0];
  events.get('switch-notice')({ payload: { id: 1, message: '<img src=x onerror=alert(1)> 目标应用没有响应' } });
  assert.equal(elements['switch-notice'].hidden, false);
  assert.equal(elements['notice-title'].textContent, '暂时无法切换');
  assert.equal(elements['notice-message'].textContent, '<img src=x onerror=alert(1)> 目标应用没有响应');
  assert.equal(elements['notice-message'].title, elements['notice-message'].textContent);
  assert.equal(elements.rail.classList.contains('has-notice'), true);
  assert.equal(elements.items.children[0], row);
  events.get('hint-hide')();
  assert.equal(elements['switch-notice'].hidden, false);
  events.get('switch-notice')({ payload: { id: 2, message: '新的错误' } });
  assert.equal(elements['notice-message'].textContent, '新的错误');
  assert.equal(timeouts.length, 0, 'frontend must not race the native notice TTL');
  events.get('switch-notice')({ payload: null });
  assert.equal(elements['switch-notice'].hidden, true);
  assert.equal(elements.rail.classList.contains('has-notice'), false);
  assert.equal(elements['notice-message'].textContent, '');
  assert.equal(elements['notice-message'].title, '');
});

test('configuration and language changes retain the current notice and update its translation', async () => {
  const { elements, events, config } = await overlayHarness();
  assert.ok(events.has('switch-notice'), 'overlay subscribes to switch failure feedback');
  events.get('switch-notice')({ payload: { id: 3, message: '目标应用暂时没有响应，请在桌面切换完成后重试。' } });
  events.get('config-changed')({ payload: { ...config, language: 'en', hint: { ...config.hint, scale: 150, side: 'right' } } });
  assert.equal(elements['switch-notice'].hidden, false);
  assert.equal(elements.rail.classList.contains('has-notice'), true);
  assert.equal(elements['notice-title'].textContent, 'Unable to switch');
  assert.equal(elements['notice-message'].textContent, 'The target app is temporarily unresponsive. Try again after the desktop switch finishes.');
  assert.equal(elements.items.children.length, 1);
  i18n.setLanguage('zh-CN');
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

test('the initial state restores a notice raised before overlay listeners were ready', async () => {
  const { elements, events, timeouts } = await overlayHarness({ initialNotice: { id: 42, message: '启动时切换失败 <script>unsafe</script>' } });
  assert.ok(events.has('switch-notice'));
  assert.equal(elements['switch-notice'].hidden, false);
  assert.equal(elements.rail.classList.contains('has-notice'), true);
  assert.equal(elements['notice-title'].textContent, '暂时无法切换');
  assert.equal(elements['notice-message'].textContent, '启动时切换失败 <script>unsafe</script>');
  assert.equal(timeouts.length, 0);
  events.get('switch-notice')({ payload: null });
  assert.equal(elements['switch-notice'].hidden, true);
});
