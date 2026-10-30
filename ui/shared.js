import { glyphs } from './model.js';
import { t } from './i18n.js';
const namespace = 'http://www.w3.org/2000/svg';
let badgeCounter = 0;
const icons = new Map();

export function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function badge(letter) {
  const svg = document.createElementNS(namespace, 'svg');
  svg.setAttribute('viewBox', '0 0 36 36');
  svg.setAttribute('aria-label', letter || t('未绑定'));
  svg.setAttribute('role', 'img');
  svg.classList.add('badge');
  const circle = document.createElementNS(namespace, 'circle');
  circle.setAttribute('cx', '18'); circle.setAttribute('cy', '18'); circle.setAttribute('r', '17');
  circle.setAttribute('fill', 'currentColor');
  if (glyphs[letter]) {
    const defs = document.createElementNS(namespace, 'defs');
    const mask = document.createElementNS(namespace, 'mask');
    const id = `badge-${badgeCounter++}`;
    mask.setAttribute('id', id);
    const white = document.createElementNS(namespace, 'rect');
    white.setAttribute('width', '36'); white.setAttribute('height', '36'); white.setAttribute('fill', 'white');
    const path = document.createElementNS(namespace, 'path');
    path.setAttribute('d', glyphs[letter]); path.setAttribute('transform', 'translate(6.8 6.8) scale(.8)');
    path.setAttribute('fill', 'none'); path.setAttribute('stroke', 'black'); path.setAttribute('stroke-width', '3.2');
    path.setAttribute('stroke-linecap', 'round'); path.setAttribute('stroke-linejoin', 'round');
    mask.append(white, path); defs.append(mask); svg.append(defs);
    circle.setAttribute('mask', `url(#${id})`);
  } else {
    circle.setAttribute('fill', 'none'); circle.setAttribute('stroke', 'currentColor'); circle.setAttribute('stroke-opacity', '.35');
  }
  svg.append(circle);
  return svg;
}

export async function loadIcon(image, path) {
  if (!window.__TAURI__) return;
  if (!icons.has(path)) icons.set(path, window.__TAURI__.core.invoke('app_icon', { path }).catch(() => null));
  const source = await icons.get(path);
  if (source && image.isConnected) { image.src = source; image.classList.add('loaded'); }
}
