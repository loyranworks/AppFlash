import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('release configuration is signed and version-matched', () => {
  const env = { ...process.env }; delete env.GITHUB_REF_NAME;
  assert.match(execFileSync(process.execPath, ['tools/check-release.mjs'], { env }).toString(), /configuration valid/);
  assert.throws(() => execFileSync(process.execPath, ['tools/check-release.mjs'], { env: { ...env, GITHUB_REF_NAME: 'v999.0.0' }, stdio: 'pipe' }));
});


test('platform builds share one pre-created release', () => {
  const workflow = readFileSync('.github/workflows/release.yml', 'utf8');
  assert.match(workflow, /build:\s+needs: prepare/);
  assert.match(workflow, /releaseId: \$\{\{ needs\.prepare\.outputs\.release_id \}\}/);
  assert.match(workflow, /outputs:[\s\S]*release_id:/);
});

test('draft preparation returns its ID even before release listings refresh', { skip: process.platform === 'win32' }, () => {
  const workflow = readFileSync('.github/workflows/release.yml', 'utf8');
  const script = workflow.match(/        run: \|\n([\s\S]*?)\n  build:/)[1].replace(/^          /gm, '');
  const dir = mkdtempSync(join(tmpdir(), 'appflash-release-test-'));
  try {
    writeFileSync(join(dir, 'gh'), `#!/usr/bin/env node
const args = process.argv.slice(2);
if (args[0] === 'release' && args[1] === 'view') {
  if (!args.includes('databaseId') || !args.includes('.databaseId')) throw Error('Expected release database ID');
  if (process.env.EXISTING_DRAFT === 'true') console.log('12345');
  else process.exitCode = 1;
} else if (args[0] === 'release' && args[1] === 'create') {
  console.log('https://github.com/loyranworks/AppFlash/releases/untagged-example');
} else if (args[0] === 'api' && args.includes('POST')) {
  for (const field of ['repos/loyranworks/AppFlash/releases', 'tag_name=v0.1.7', 'draft=true', 'generate_release_notes=true']) {
    if (!args.includes(field)) throw Error('Missing release creation field: ' + field);
  }
  console.log('12345');
} else if (args[0] === 'api' && args[1].includes('releases?')) {
  // The newly created draft is not yet visible in the cached release listing.
} else {
  throw Error('Unexpected gh arguments: ' + JSON.stringify(args));
}
`, { mode: 0o755 });
    for (const existing of ['false', 'true']) {
      const output = join(dir, `output-${existing}`);
      execFileSync('bash', ['-eu', '-o', 'pipefail', '-c', script], {
        env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, GH_REPO: 'loyranworks/AppFlash', GITHUB_REF_NAME: 'v0.1.7', GITHUB_OUTPUT: output, EXISTING_DRAFT: existing },
        stdio: 'pipe',
      });
      assert.equal(readFileSync(output, 'utf8'), 'release_id=12345\n');
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
test('release publishing requires all platform builds and complete metadata', () => {
  const workflow = readFileSync('.github/workflows/release.yml', 'utf8');
  assert.match(workflow, /needs: \[prepare, build\]/);
  assert.match(workflow, /releaseDraft: true/);
  assert.match(workflow, /windows-x86_64.*darwin-aarch64.*darwin-x86_64/);
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY/);
});
