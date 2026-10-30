import test from 'node:test';
import assert from 'node:assert/strict';
import { prepareManifest } from '../tools/prepare-update-manifest.mjs';

test('release assets become public download URLs; incomplete or foreign feeds fail', () => {
  const asset = { name: 'update.exe', url: 'https://api.github.com/repos/loyranworks/AppFlash/releases/assets/1', browser_download_url: 'https://github.com/loyranworks/AppFlash/releases/download/v0.1.1/update.exe' };
  const manifest = { version: '0.1.1', platforms: Object.fromEntries(['windows-x86_64', 'darwin-aarch64', 'darwin-x86_64'].map(target => [target, { signature: 'signed', url: asset.url }])) };
  const result = prepareManifest(structuredClone(manifest), [asset], '0.1.1');
  assert.ok(Object.values(result.platforms).every(platform => platform.url === asset.browser_download_url));
  assert.throws(() => prepareManifest(manifest, [], '0.1.1'));
  assert.throws(() => prepareManifest(manifest, [asset], '0.1.2'));
  assert.throws(() => prepareManifest(manifest, [{ ...asset, browser_download_url: 'https://evil.example/update.exe' }], '0.1.1'));
  assert.throws(() => prepareManifest({ ...manifest, platforms: {} }, [asset], '0.1.1'));
});
