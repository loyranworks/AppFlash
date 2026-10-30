import { message } from './i18n.js';

export function selectSection(root, section) {
  const sections = {
    apps: ['应用', '管理应用与字母绑定'],
    hint: ['贴边提示', '需要时出现，不打断你的工作'],
    general: ['通用', '语言、登录启动与版本更新'],
  };
  if (!sections[section]) return;
  for (const panel of root.querySelectorAll('[data-panel]')) panel.hidden = panel.dataset.panel !== section;
  for (const button of root.querySelectorAll('[data-section]')) {
    if (button.dataset.section === section) button.setAttribute('aria-current', 'page');
    else button.removeAttribute('aria-current');
  }
  message(root.getElementById('page-title'), sections[section][0]);
  message(root.getElementById('page-description'), sections[section][1]);
}

export function initializeNavigation(root = document) {
  for (const button of root.querySelectorAll('[data-section]')) button.addEventListener('click', () => selectSection(root, button.dataset.section));
  selectSection(root, 'general');
}
