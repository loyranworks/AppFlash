import { badge, element, loadIcon } from './shared.js';
import { hintApps } from './model.js';
import { setLanguage, t, errorText } from './i18n.js';
const rail = document.getElementById('rail');
const items = document.getElementById('items');
const nodes = new Map();
const timers = new Map();
const notice = document.getElementById('switch-notice');
const noticeTitle = document.getElementById('notice-title');
const noticeMessage = document.getElementById('notice-message');
let activeNotice = null;

function renderNotice() {
  rail.classList.toggle('has-notice', Boolean(activeNotice));
  notice.hidden = !activeNotice;
  noticeTitle.textContent = t('暂时无法切换');
  const text = activeNotice ? errorText(activeNotice.message) : '';
  noticeMessage.textContent = text;
  noticeMessage.title = text;
}

function render(config) {
  setLanguage(config.language ?? 'zh-CN');
  document.documentElement.lang = config.language ?? 'zh-CN';
  rail.className = `rail ${config.hint.side}${navigator.platform.includes('Mac') ? ' native-material' : ''}${rail.classList.contains('shown') ? ' shown' : ''}`;
  document.documentElement.style.setProperty('--overlay-scale', String((config.hint.scale ?? 100) / 100));
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
  const mac = navigator.platform.includes('Mac');
  const display = mac ? { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' } : { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: 'Win' };
  document.getElementById('prefix').textContent = config.prefix.map(key => display[key]).join(mac ? ' ' : ' + ');
  renderNotice();
}

await window.__TAURI__.event.listen('config-changed', event => render(event.payload));
await window.__TAURI__.event.listen('hint-show', () => { rail.classList.remove('shown'); void rail.offsetWidth; rail.classList.add('shown'); });
await window.__TAURI__.event.listen('hint-hide', () => { if (!activeNotice) rail.classList.remove('shown'); });
await window.__TAURI__.event.listen('switch-notice', event => {
  const changed = event.payload?.id !== activeNotice?.id;
  activeNotice = event.payload;
  renderNotice();
  if (activeNotice && changed) {
    notice.classList.remove('reveal'); void notice.offsetWidth; notice.classList.add('reveal');
  }
});
await window.__TAURI__.event.listen('app-hit', event => {
  const node = nodes.get(event.payload);
  if (!node) return;
  if (timers.has(event.payload)) clearTimeout(timers.get(event.payload));
  node.classList.remove('hit'); void node.offsetWidth; node.classList.add('hit');
  timers.set(event.payload, setTimeout(() => { node.classList.remove('hit'); timers.delete(event.payload); }, 460));
});
const initialState = await window.__TAURI__.core.invoke('get_state');
activeNotice = initialState.notice ?? null;
render(initialState.config);
