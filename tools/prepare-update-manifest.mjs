import fs from 'node:fs';
import assert from 'node:assert/strict';

export function prepareManifest(manifest, assets, version) {
  assert.equal(manifest.version, version);
  for (const target of ['windows-x86_64', 'darwin-aarch64', 'darwin-x86_64']) assert.ok(manifest.platforms[target]?.signature, target);
  for (const platform of Object.values(manifest.platforms)) {
    assert.ok(platform.signature);
    const asset = assets.find(asset => platform.url === asset.url || platform.url === asset.browser_download_url);
    assert.ok(asset, 'Update URL must identify a release asset');
    assert.ok(asset.browser_download_url.startsWith('https://github.com/loyranworks/AppFlash/releases/download/'));
    assert.ok(asset.name && !asset.name.includes('/'));
    platform.url = 'https://github.com/loyranworks/AppFlash/releases/download/v' + version + '/' + encodeURIComponent(asset.name);
  }
  return manifest;
}

if (process.argv[2]) {
  const [manifestPath, assetsPath, version] = process.argv.slice(2);
  const manifest = prepareManifest(JSON.parse(fs.readFileSync(manifestPath)), JSON.parse(fs.readFileSync(assetsPath)), version);
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
}
