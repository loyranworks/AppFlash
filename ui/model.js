import { t } from './i18n.js';

export function prefixKeyLabel(key, platform) {
  const labels = platform === 'macos'
    ? { Control: 'Control', Alt: 'Option', Shift: 'Shift', Super: 'Command' }
    : { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: 'Win' };
  return labels[key] ?? key;
}

export function normalizeLetter(value) {
  const letter = value.trim().toUpperCase();
  if (!/^[A-Z]?$/.test(letter)) throw new Error(t('字母必须是一个 A–Z，留空表示暂不启用。'));
  return letter;
}

export function addImports(apps, selected, platform, createId) {
  const key = path => platform === 'windows' ? path.replaceAll('/', '\\').toLowerCase() : path;
  const result = structuredClone(apps);
  const known = new Set(apps.map(app => key(app.path)));
  for (const app of selected) {
    if (known.has(key(app.path))) continue;
    result.push({ id: createId(), name: app.name, path: app.path, letter: normalizeLetter(app.letter ?? '') });
    known.add(key(app.path));
  }
  return result;
}

export function hintApps(apps, order) {
  const bound = apps.filter(app => app.letter);
  return order === 'alphabetical' ? bound.sort((first, second) => first.letter.localeCompare(second.letter)) : bound;
}

export function moveApp(apps, id, direction) {
  const index = apps.findIndex(app => app.id === id);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= apps.length) return apps;
  const reordered = [...apps];
  [reordered[index], reordered[target]] = [reordered[target], reordered[index]];
  return reordered;
}

export function shortcutWarnings(prefix, platform) {
  const warnings = [];
  if (prefix.includes('Super')) warnings.push(t(platform === 'macos' ? 'Command 参与的组合可能与 macOS 或应用快捷键重叠。' : 'Win 参与的组合可能与系统保留快捷键重叠，部分组合无法注册。'));
  return warnings;
}

export const glyphs = {
  A: 'M5 24L13 4L21 24M8 16H18',
  B: 'M7 4V24M7 4H13C20 4 20 13 13 13H7M13 13C22 13 22 24 13 24H7',
  C: 'M21 7C18 3 10 3 7 7C4 11 4 17 7 21C10 25 18 25 21 21',
  D: 'M7 4V24H12C25 24 25 4 12 4Z',
  E: 'M20 4H7V24H20M7 13H17',
  F: 'M8 24V4H20M8 13H17',
  G: 'M21 7C17 2 8 3 6 10C2 23 15 28 21 20V14H14',
  H: 'M6 4V24M21 4V24M6 14H21',
  I: 'M7 4H19M13 4V24M7 24H19',
  J: 'M8 4H20V18C20 27 6 27 6 19',
  K: 'M6 4V24M21 4L6 17M12 12L22 24',
  L: 'M8 4V24H20',
  M: 'M4 24V4L13 17L22 4V24',
  N: 'M6 24V4L21 24V4',
  O: 'M13 4C2 4 2 24 13 24C24 24 24 4 13 4Z',
  P: 'M7 24V4H13C22 4 22 15 13 15H7',
  Q: 'M13 4C2 4 2 24 13 24C24 24 24 4 13 4ZM16 18L23 26',
  R: 'M7 24V4H13C22 4 22 15 13 15H7M13 15L22 24',
  S: 'M21 7C17 1 5 3 6 10C7 15 22 12 21 19C20 26 8 27 5 21',
  T: 'M4 4H22M13 4V24',
  U: 'M6 4V17C6 27 21 27 21 17V4',
  V: 'M5 4L13 24L22 4',
  W: 'M3 4L7 24L13 10L19 24L24 4',
  X: 'M5 4L22 24M22 4L5 24',
  Y: 'M5 4L13 14L22 4M13 14V24',
  Z: 'M5 4H22L5 24H22',
};
