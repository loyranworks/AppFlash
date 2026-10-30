import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';

test('release configuration is signed and version-matched', () => {
  const env = { ...process.env }; delete env.GITHUB_REF_NAME;
  assert.match(execFileSync(process.execPath, ['tools/check-release.mjs'], { env }).toString(), /configuration valid/);
  assert.throws(() => execFileSync(process.execPath, ['tools/check-release.mjs'], { env: { ...env, GITHUB_REF_NAME: 'v999.0.0' }, stdio: 'pipe' }));
});

test('release publishing requires all platform builds and complete metadata', () => {
  const workflow = readFileSync(existsSync('.github/workflows/release.yml') ? '.github/workflows/release.yml' : 'docs/release-workflow.yml', 'utf8');
  assert.match(workflow, /needs: build/);
  assert.match(workflow, /releaseDraft: true/);
  assert.match(workflow, /windows-x86_64.*darwin-aarch64.*darwin-x86_64/);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY/);
});
