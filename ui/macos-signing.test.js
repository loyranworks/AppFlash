import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

test('fixed certificate accepts changed builds but rejects ad-hoc, foreign and tampered code', { skip: process.platform !== 'darwin' }, () => {
  const root = mkdtempSync(join(tmpdir(), 'appflash-signing-test-'));
  const keychain = join(root, 'test.keychain-db');
  const run = (program, args) => execFileSync(program, args, { stdio: 'pipe' }).toString();
  try {
    run('security', ['create-keychain', '-p', 'ephemeral-test-only', keychain]);
    run('security', ['unlock-keychain', '-p', 'ephemeral-test-only', keychain]);
    const identities = [];
    for (const name of ['expected', 'foreign']) {
      const config = join(root, `${name}.cnf`);
      const key = join(root, `${name}.key`), cert = join(root, `${name}.pem`), p12 = join(root, `${name}.p12`);
      writeFileSync(config, `[req]\ndistinguished_name=dn\nx509_extensions=ext\nprompt=no\n[dn]\nCN=AppFlash ephemeral ${name}\n[ext]\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=critical,codeSigning\n`);
      run('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-days', '1', '-config', config, '-keyout', key, '-out', cert]);
      run('openssl', ['pkcs12', '-export', '-inkey', key, '-in', cert, '-out', p12, '-passout', 'pass:ephemeral-test-only', '-keypbe', 'PBE-SHA1-3DES', '-certpbe', 'PBE-SHA1-3DES', '-macalg', 'sha1']);
      run('security', ['import', p12, '-f', 'pkcs12', '-k', keychain, '-P', 'ephemeral-test-only', '-T', '/usr/bin/codesign']);
      identities.push({ cert, p12, identity: run('openssl', ['x509', '-in', cert, '-noout', '-fingerprint', '-sha1']).split('=')[1].trim().replaceAll(':', '') });
    }
    run('security', ['set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', 'ephemeral-test-only', keychain]);
    const paths = ['a', 'b', 'adhoc', 'foreign', 'tampered'].map(name => join(root, name));
    for (const [i, path] of paths.entries()) {
      const source = `${path}.c`;
      writeFileSync(source, `int main(void) { return ${i}; }\n`);
      run('cc', [source, '-o', path]);
      const identity = path.endsWith('adhoc') ? '-' : identities[path.endsWith('foreign') ? 1 : 0].identity;
      run('codesign', ['--force', '--sign', identity, '--keychain', keychain, '--identifier', 'local.appflash.signing-test', '--timestamp=none', path]);
    }
    writeFileSync(paths[4], 'modified bytes');
    for (const [i, path] of paths.entries()) {
      const result = spawnSync('python3', ['tools/verify-macos-signing.py', path, '--certificate', identities[0].cert, '--identifier', 'local.appflash.signing-test'], { encoding: 'utf8' });
      if (i < 2) assert.equal(result.status, 0, result.stdout + result.stderr);
      else assert.notEqual(result.status, 0, `${path} must be rejected`);
    }
    const before = run('security', ['list-keychains', '-d', 'user']);
    const state = join(root, 'import-state.json'), output = join(root, 'github-env');
    const env = { ...process.env, MACOS_SIGN_CERTIFICATE: readFileSync(identities[0].p12).toString('base64'), MACOS_SIGN_PASSWORD: 'ephemeral-test-only', GITHUB_ENV: output };
    const imported = spawnSync('python3', ['tools/macos-signing-keychain.py', 'import', '--state', state, '--certificate', identities[0].cert], { env, encoding: 'utf8' });
    assert.equal(imported.status, 0, imported.stdout + imported.stderr);
    const importedState = JSON.parse(readFileSync(state, 'utf8'));
    try {
      assert.ok(existsSync(importedState.keychain));
      assert.ok(readFileSync(output, 'utf8').includes(`APPLE_SIGNING_IDENTITY=${identities[0].identity}`));
      assert.ok(run('security', ['list-keychains', '-d', 'user']).includes(importedState.keychain));
    } finally {
      run('python3', ['tools/macos-signing-keychain.py', 'cleanup', '--state', state]);
    }
    assert.equal(run('security', ['list-keychains', '-d', 'user']), before);
    assert.ok(!existsSync(importedState.keychain));
    const foreign = spawnSync('python3', ['tools/macos-signing-keychain.py', 'import', '--state', state, '--certificate', identities[1].cert], { env, encoding: 'utf8' });
    assert.notEqual(foreign.status, 0);
    assert.ok(!existsSync(state));
  } finally {
    spawnSync('security', ['delete-keychain', keychain]);
    rmSync(root, { recursive: true, force: true });
  }
});
