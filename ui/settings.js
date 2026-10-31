import { addImports, normalizeLetter, shortcutWarnings, moveApp } from './model.js';
import { badge, element, loadIcon } from './shared.js';
import { initializeUpdates } from './updates.js';
import { t, setLanguage, getLanguage, translatePage, message, errorMessage } from './i18n.js';
import { initializeNavigation } from './navigation.js';

const byId = id => document.getElementById(id);
const invoke = (command, args) => window.__TAURI__.core.invoke(command, args);
let state;
let draft;
let dirty = false;
let editingId;
let importRows = [];
let pendingSave;

function showError(error) { errorMessage(byId('error'), error); }
function changed() { dirty = true; message(byId('save-status'), '有待保存的修改'); }
function prefix() { return [...document.querySelectorAll('#prefix-options input:checked')].map(input => input.value); }

function updateTheme() {
  document.documentElement.style.setProperty('--accent', byId('letter-color').value);
  document.documentElement.style.setProperty('--brand-color', state.paused ? '#343840' : byId('letter-color').value);
  byId('color-value').textContent = byId('letter-color').value;
  for (const id of ['hint-delay', 'hint-order']) byId(id).disabled = !byId('hint-enabled').checked;
  for (const input of document.querySelectorAll('input[name=side]')) input.disabled = !byId('hint-enabled').checked;
  byId('shortcut-warning').textContent = shortcutWarnings(prefix(), state.platform).join(' ');
}

function renderApps() {
  const list = byId('app-list');
  list.replaceChildren();
  message(byId('app-count'), '{total} · {bound} 已绑定', { total: draft.apps.length, bound: draft.apps.filter(app => app.letter).length });
  if (!draft.apps.length) { list.append(element('div', 'empty', t('添加常用应用，分配一个字母。\n按住 Prefix，一闪即达。'))); return; }
  for (const app of draft.apps) {
    const row = element('div', 'app-row');
    const main = element('div', 'app-main');
    const image = element('img', 'app-image'); image.alt = '';
    const text = element('div', 'app-text');
    const path = element('p', 'path', app.path); path.title = app.path;
    text.append(element('strong', '', app.name), path); main.append(image, text);
    main.addEventListener('click', () => edit(app));
    const binding = element('button', 'binding'); binding.type = 'button';
    binding.setAttribute('aria-label', t('{name}，{binding}，编辑', { name: app.name, binding: app.letter ? t('字母 {letter}', { letter: app.letter }) : t('未绑定') }));
    binding.append(badge(app.letter)); binding.addEventListener('click', () => edit(app));
    const more = element('button', 'more', '⋯'); more.type = 'button'; more.setAttribute('aria-label', t('编辑 {name}', { name: app.name })); more.addEventListener('click', () => edit(app));
    const reorder = element('div', 'reorder');
    for (const [direction, label] of [[-1, t('上移')], [1, t('下移')]]) {
      const button = element('button', 'quiet', direction === -1 ? '↑' : '↓');
      button.type = 'button'; button.title = label;
      button.setAttribute('aria-label', t('{action} {name}', { action: label, name: app.name }));
      button.dataset.moveId = app.id; button.dataset.direction = direction;
      const index = draft.apps.indexOf(app);
      button.disabled = index + direction < 0 || index + direction >= draft.apps.length;
      button.addEventListener('click', () => {
        const scroll = list.scrollTop;
        draft.apps = moveApp(draft.apps, app.id, direction);
        changed(); renderApps(); list.scrollTop = scroll;
        const moved = [...list.querySelectorAll('[data-move-id]')].find(node => node.dataset.moveId === app.id && Number(node.dataset.direction) === direction);
        (moved?.disabled ? list : moved)?.focus({ preventScroll: true });
      });
      reorder.append(button);
    }
    row.append(main, binding, reorder, more); list.append(row); loadIcon(image, app.path);
  }
}

function renderState() {
  draft = structuredClone(state.config);
  setLanguage(draft.language); byId('language').value = draft.language; translatePage();
  for (const input of document.querySelectorAll('#prefix-options input')) input.checked = draft.prefix.includes(input.value);
  byId('hint-enabled').checked = draft.hint.enabled;
  byId('hint-delay').value = draft.hint.delay;
  byId('hint-order').value = draft.hint.order;
  document.querySelector(`input[name=side][value=${draft.hint.side}]`).checked = true;
  byId('letter-color').value = draft.hint.color;
  byId('super-name').textContent = state.platform === 'macos' ? '⌘' : 'Win';
  byId('access').hidden = state.platform !== 'macos';
  message(byId('pause'), state.paused ? '恢复切换' : '暂停切换');
  message(byId('save-status'), state.paused ? '快捷切换已暂停' : '已加载 · 无需重启');
  byId('config-path').textContent = state.config_path;
  if (state.error) showError(state.error);
  updateTheme(); renderApps(); dirty = false;
}

function edit(app) {
  editingId = app?.id;
  message(byId('edit-title'), app ? '编辑应用' : '添加应用');
  byId('edit-name').value = app?.name ?? '';
  byId('edit-path').value = app?.path ?? '';
  byId('edit-letter').value = app?.letter ?? '';
  errorMessage(byId('edit-error'), '');
  byId('delete-app').hidden = !app;
  byId('edit-dialog').showModal();
}

byId('edit-form').addEventListener('submit', event => {
  event.preventDefault();
  try {
    const app = { id: editingId ?? crypto.randomUUID(), name: byId('edit-name').value.trim(), path: byId('edit-path').value.trim(), letter: normalizeLetter(byId('edit-letter').value) };
    if (!app.name || !app.path) throw new Error(t('应用名称和完整地址不能为空。'));
    if (app.letter && draft.apps.some(other => other.id !== app.id && other.letter === app.letter)) throw new Error(t('字母 {letter} 已被其他应用使用。', { letter: app.letter }));
    const index = draft.apps.findIndex(other => other.id === app.id);
    if (index < 0) draft.apps.push(app); else draft.apps[index] = app;
    changed(); renderApps(); byId('edit-dialog').close();
  } catch (error) { errorMessage(byId('edit-error'), error); }
});

byId('delete-app').addEventListener('click', () => {
  draft.apps = draft.apps.filter(app => app.id !== editingId);
  changed(); renderApps(); byId('edit-dialog').close();
});
byId('cancel-edit').addEventListener('click', () => byId('edit-dialog').close());
byId('add').addEventListener('click', () => edit());

async function scanImports() {
  if (byId('refresh-import').disabled) return;
  byId('refresh-import').disabled = true;
  importRows = [];
  errorMessage(byId('import-error'), '');
  byId('import-list').replaceChildren(element('p', 'footnote', t('正在扫描当前有窗口的应用…')));
  byId('confirm-import').disabled = true;
  try {
    const apps = await invoke('list_running');
    importRows = [];
    byId('import-list').replaceChildren();
    for (const app of apps) {
      const row = element('label', 'import-row');
      const checkbox = element('input'); checkbox.type = 'checkbox'; checkbox.setAttribute('aria-label', t('选择 {name}', { name: app.name }));
      const image = element('img', 'app-image'); image.alt = '';
      const text = element('div', 'app-text'); const path = element('p', 'path', app.path); path.title = app.path;
      text.append(element('strong', '', app.name), path);
      const letter = element('input', 'import-letter'); letter.maxLength = 1; letter.placeholder = '—'; letter.setAttribute('aria-label', t('{name} 的字母', { name: app.name }));
      letter.addEventListener('click', event => event.stopPropagation());
      letter.addEventListener('input', () => { checkbox.checked = true; });
      const key = path => state.platform === 'windows' ? path.replaceAll('/', '\\').toLowerCase() : path;
      const existing = draft.apps.some(other => key(other.path) === key(app.path));
      if (existing) { checkbox.disabled = true; letter.disabled = true; letter.title = t('已在列表中'); }
      row.append(checkbox, image, text, letter); byId('import-list').append(row);
      importRows.push({ app, row, checkbox, letter }); loadIcon(image, app.path);
    }
    if (!apps.length) byId('import-list').append(element('p', 'footnote', t('没有可导入的窗口。先打开应用，或使用“添加”填写地址。')));
    filterImports();
  } catch (error) { errorMessage(byId('import-error'), error); }
  finally { byId('confirm-import').disabled = false; byId('refresh-import').disabled = false; }
}

function filterImports() {
  const query = byId('import-search').value.toLowerCase();
  for (const { app, row } of importRows) row.hidden = !`${app.name} ${app.path}`.toLowerCase().includes(query);
}

byId('import').addEventListener('click', () => { byId('import-search').value = ''; byId('import-dialog').showModal(); scanImports(); });
byId('refresh-import').addEventListener('click', scanImports);
byId('import-search').addEventListener('input', filterImports);
byId('cancel-import').addEventListener('click', () => byId('import-dialog').close());
byId('confirm-import').addEventListener('click', () => {
  try {
    const selected = importRows.filter(row => row.checkbox.checked && !row.checkbox.disabled).map(row => ({ ...row.app, letter: row.letter.value }));
    const apps = addImports(draft.apps, selected, state.platform, () => crypto.randomUUID());
    const letters = apps.map(app => app.letter).filter(Boolean);
    if (new Set(letters).size !== letters.length) throw new Error(t('所选字母有重复，请修改后再导入。'));
    draft.apps = apps; changed(); renderApps(); byId('import-dialog').close();
  } catch (error) { errorMessage(byId('import-error'), error); }
});

function collectConfig() {
  const config = structuredClone(draft);
  config.language = getLanguage();
  config.prefix = prefix();
  if (!config.prefix.length) throw new Error(t('至少选择一个 Prefix 修饰键。'));
  const delay = Number(byId('hint-delay').value);
  if (byId('hint-delay').value === '' || !Number.isFinite(delay) || delay < 0 || delay > 10) throw new Error(t('出现延迟必须在 0–10 秒之间。'));
  config.hint = { enabled: byId('hint-enabled').checked, delay, side: document.querySelector('input[name=side]:checked').value, color: byId('letter-color').value, order: byId('hint-order').value };
  return config;
}

async function save(config) {
  byId('save').disabled = true; byId('language').disabled = true; message(byId('save-status'), '正在检查绑定并保存…'); showError('');
  try { state = await invoke('save_config', { config }); renderState(); message(byId('save-status'), state.paused ? '已保存 · 切换仍暂停' : '已保存并生效 · 无需重启'); }
  catch (error) { showError(error); message(byId('save-status'), String(error).includes('回滚失败') ? '绑定恢复失败，请退出重开' : '未保存，原绑定保持不变'); }
  finally { byId('save').disabled = false; byId('language').disabled = false; }
}

byId('save').addEventListener('click', () => {
  try {
    const config = collectConfig();
    const warnings = shortcutWarnings(config.prefix, state.platform);
    if (warnings.length) { pendingSave = config; byId('confirm-text').textContent = warnings.join(' '); byId('confirm-dialog').showModal(); }
    else save(config);
  } catch (error) { showError(error); }
});
byId('cancel-confirm').addEventListener('click', () => { pendingSave = undefined; byId('confirm-dialog').close(); });
byId('accept-confirm').addEventListener('click', () => { const config = pendingSave; pendingSave = undefined; byId('confirm-dialog').close(); if (config) save(config); });
byId('pause').addEventListener('click', async () => {
  byId('pause').disabled = true;
  try { const next = await invoke('set_paused', { paused: !state.paused }); state.paused = next.paused; message(byId('pause'), state.paused ? '恢复切换' : '暂停切换'); updateTheme(); if (!dirty) message(byId('save-status'), state.paused ? '快捷切换已暂停' : '快捷切换已恢复'); }
  catch (error) { showError(error); }
  finally { byId('pause').disabled = false; }
});
byId('access').addEventListener('click', async () => { try { await invoke('request_access'); } catch (error) { showError(error); } });
for (const input of document.querySelectorAll('#prefix-options input,#hint-enabled,#hint-delay,#hint-order,input[name=side],#letter-color')) input.addEventListener('input', () => { changed(); updateTheme(); });
byId('reset-color').addEventListener('click', () => { byId('letter-color').value = '#587a6b'; changed(); updateTheme(); });

byId('autostart').addEventListener('change', async () => {
  const input = byId('autostart');
  const previous = !input.checked;
  input.disabled = true;
  message(byId('autostart-status'), '正在更新系统启动项…');
  try {
    input.checked = await invoke('set_autostart', { enabled: input.checked });
    message(byId('autostart-status'), input.checked ? '已启用 · 登录后静默驻留' : '已关闭');
  } catch (error) {
    input.checked = previous;
    errorMessage(byId('autostart-status'), error);
    try { input.checked = await invoke('get_autostart'); } catch { input.disabled = true; return; }
  }
  input.disabled = false;
});

initializeNavigation(); translatePage();
byId('language').addEventListener('change', async () => {
  const previous = getLanguage();
  const selected = byId('language').value;
  byId('language').disabled = true; byId('save').disabled = true;
  message(byId('language-status'), '正在保存语言…');
  try {
    await invoke('set_language', { language: selected });
    setLanguage(selected); state.config.language = selected; draft.language = selected;
    translatePage();
    const scroll = byId('app-list').scrollTop; renderApps(); byId('app-list').scrollTop = scroll; updateTheme();
    message(byId('language-status'), '语言已保存');
  } catch (error) {
    byId('language').value = previous; errorMessage(byId('language-status'), error);
  } finally { byId('language').disabled = false; byId('save').disabled = false; }
});

try {
  if (!window.__TAURI__) throw new Error(t('请通过 AppFlash 桌面应用打开设置；设计网页不具有系统快捷键权限。'));
  await window.__TAURI__.event.listen('app-error', event => showError(event.payload));
  await window.__TAURI__.event.listen('paused-changed', event => {
    if (!state) return;
    state.paused = event.payload; message(byId('pause'), state.paused ? '恢复切换' : '暂停切换'); updateTheme();
  });
  state = await invoke('get_state'); renderState(); byId('language').disabled = false;
  await initializeUpdates({ invoke, listen: window.__TAURI__.event.listen, hasUnsavedChanges: () => dirty, version: await window.__TAURI__.app.getVersion() });
  try {
    byId('autostart').checked = await invoke('get_autostart');
    byId('autostart').disabled = false;
    message(byId('autostart-status'), byId('autostart').checked ? '已启用 · 登录后静默驻留' : '已关闭');
  } catch (error) { errorMessage(byId('autostart-status'), error); }
} catch (error) { showError(error); byId('save').disabled = true; message(byId('save-status'), '初始化失败'); }
