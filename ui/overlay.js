import { badge, element, loadIcon } from './shared.js';
import { hintApps } from './model.js';
const rail = document.getElementById('rail');
const items = document.getElementById('items');
const nodes = new Map();
const timers = new Map();

function render(config) {
  rail.className = `rail ${config.hint.side}`;
  document.documentElement.style.setProperty('--accent', config.hint.color);
  for (const timer of timers.values()) clearTimeout(timer);
  timers.clear(); nodes.clear(); items.replaceChildren();
  const apps = hintApps(config.apps, config.hint.order);
  items.classList.toggle('dense', apps.length > 12);
  for (const app of apps) {
    const row = element('div', 'item');
    const frame = element('span', 'icon-frame');
    const image = element('img'); image.alt = app.name; frame.append(image);
    row.append(frame, badge(app.letter), element('span', 'tick'));
    items.append(row); nodes.set(app.id, row); loadIcon(image, app.path);
  }
  const display = { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: navigator.platform.includes('Mac') ? '⌘' : 'Win' };
  document.getElementById('prefix').textContent = config.prefix.map(key => display[key]).join(' + ');
}

await window.__TAURI__.event.listen('config-changed', event => render(event.payload));
await window.__TAURI__.event.listen('hint-show', () => { rail.classList.remove('shown'); void rail.offsetWidth; rail.classList.add('shown'); });
await window.__TAURI__.event.listen('hint-hide', () => { rail.classList.remove('shown'); });
await window.__TAURI__.event.listen('app-hit', event => {
  const node = nodes.get(event.payload);
  if (!node) return;
  if (timers.has(event.payload)) clearTimeout(timers.get(event.payload));
  node.classList.remove('hit'); void node.offsetWidth; node.classList.add('hit');
  timers.set(event.payload, setTimeout(() => { node.classList.remove('hit'); timers.delete(event.payload); }, 460));
});
render((await window.__TAURI__.core.invoke('get_state')).config);
