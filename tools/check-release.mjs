import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const packageInfo = JSON.parse(readFileSync('package.json', 'utf8'));
const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8');
const version = packageInfo.version;
assert.match(version, /^\d+\.\d+\.\d+$/);
assert.equal(config.version, version, 'Tauri and npm versions must match');
assert.equal(cargo.match(/^version = "([^"]+)"/m)?.[1], version, 'Cargo and npm versions must match');
if (process.env.GITHUB_REF_NAME) assert.equal(process.env.GITHUB_REF_NAME, `v${version}`, 'Release tag must match app version');
assert.equal(config.bundle.createUpdaterArtifacts, true);
assert.equal(config.bundle.macOS.signingIdentity, 'Loyran Works AppFlash Code Signing', 'macOS releases must use the fixed identity');
assert.match(readFileSync('tools/macos-signing-cert.pem', 'utf8'), /-----BEGIN CERTIFICATE-----/);
assert.equal(config.plugins.updater.requireSignedVersion, true);
assert.equal(config.plugins.updater.endpoints[0], 'https://github.com/loyranworks/AppFlash/releases/latest/download/latest.json');
assert.match(Buffer.from(config.plugins.updater.pubkey, 'base64').toString(), /minisign public key/);
console.log(`Release configuration valid: ${version}`);
