import { t, message, errorMessage } from './i18n.js';

export function downloadProgress(received, total) {
  if (typeof total === 'number' && total > 0) return t('正在下载 {percent}%', { percent: Math.min(100, Math.round(received / total * 100)) });
  return t('正在下载 {size} MB', { size: (received / 1024 / 1024).toFixed(1) });
}

export async function initializeUpdates({ invoke, listen, hasUnsavedChanges, version }) {
  const byId = id => document.getElementById(id);
  let available;
  let busy = false;
  message(byId('app-version'), '当前版本 {version}', { version });

  async function check() {
    if (busy || byId('update-dialog').open) return;
    busy = true; byId('check-update').disabled = true;
    byId('install-update').disabled = true;
    message(byId('update-status'), '正在检查更新…');
    try {
      available = await invoke('check_update');
      message(byId('update-status'), available ? '发现新版本 {version}' : '已是最新版本', { version: available?.version });
      byId('install-update').hidden = !available;
      if (available?.notes) { delete byId('update-notes').dataset.i18n; byId('update-notes').textContent = available.notes; }
      else message(byId('update-notes'), '暂无更新说明。');
    } catch (error) {
      errorMessage(byId('update-status'), error);
    } finally {
      busy = false; byId('check-update').disabled = false;
      byId('install-update').disabled = !available;
    }
  }

  byId('check-update').addEventListener('click', check);
  byId('install-update').addEventListener('click', () => {
    if (!available || busy) return;
    if (hasUnsavedChanges()) { message(byId('update-status'), '请先保存或处理未保存的设置，再升级。'); return; }
    message(byId('update-confirm-text'), '将升级到 {version}。下载后会校验更新包签名与版本，安装后 AppFlash 会退出并重新启动；已保存的配置会保留并备份。', { version: available.version });
    byId('update-dialog').showModal();
  });
  byId('cancel-update').addEventListener('click', () => byId('update-dialog').close());
  byId('confirm-update').addEventListener('click', async () => {
    if (!available || busy) return;
    if (hasUnsavedChanges()) { byId('update-dialog').close(); message(byId('update-status'), '请先保存设置再升级。'); return; }
    busy = true; byId('update-dialog').close();
    byId('check-update').disabled = true; byId('install-update').disabled = true;
    document.querySelector('main').inert = true;
    message(byId('update-status'), '正在下载更新…');
    try { await invoke('install_update', { version: available.version }); }
    catch (error) { errorMessage(byId('update-status'), error); }
    finally {
      busy = false; document.querySelector('main').inert = false;
      byId('check-update').disabled = false; byId('install-update').disabled = !available;
    }
  });
  await listen('update-progress', event => { const { received, total } = event.payload;
    message(byId('update-status'), typeof total === 'number' && total > 0 ? '正在下载 {percent}%' : '正在下载 {size} MB', { percent: Math.min(100, Math.round(received / total * 100)), size: (received / 1024 / 1024).toFixed(1) }); });
  await listen('update-installing', () => { message(byId('update-status'), '下载并校验完成，正在安装…'); });
  setTimeout(check, 3000);
  setInterval(check, 6 * 60 * 60 * 1000);
}
