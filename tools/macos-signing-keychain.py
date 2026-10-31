"""Import the pinned signing identity into a disposable build keychain."""

import argparse
import base64
import hashlib
import json
import os
import pathlib
import re
import secrets
import subprocess


def checked(args, env=None):
    result = subprocess.run(args, capture_output=True, env=env)
    if result.returncode:
        # Never echo command arguments: they may contain a keychain password.
        raise ValueError(result.stderr.decode(errors="replace").strip())
    return result.stdout


def search_list():
    return re.findall(r'"([^"\n]+)"', checked(["security", "list-keychains", "-d", "user"]).decode())


def cleanup(state):
    if not state.exists():
        return
    value = json.loads(state.read_text())
    keychain = pathlib.Path(value["keychain"])
    if value.get("format") != "appflash-signing-v1" or keychain.parent.resolve() != state.parent.resolve() or not keychain.name.startswith("appflash-signing-"):
        raise ValueError("Refusing to clean an unrelated keychain")
    current = [path for path in search_list() if pathlib.Path(path).resolve() != keychain.resolve()]
    checked(["security", "list-keychains", "-d", "user", "-s", *current])
    if keychain.exists():
        checked(["security", "delete-keychain", str(keychain)])
    state.unlink()
    print("Temporary signing keychain cleaned up")


def import_identity(state, certificate):
    if state.exists():
        raise ValueError("A signing state already exists; clean it up before importing")
    encoded = os.environ.get("MACOS_SIGN_CERTIFICATE")
    password = os.environ.get("MACOS_SIGN_PASSWORD")
    if not encoded or not password:
        raise ValueError("Configure MACOS_SIGN_CERTIFICATE and MACOS_SIGN_PASSWORD in Actions secrets")
    data = base64.b64decode(encoded, validate=True)
    state.parent.mkdir(parents=True, exist_ok=True)
    suffix = secrets.token_hex(10)
    archive = state.parent / ("appflash-signing-" + suffix + ".p12")
    keychain = state.parent / ("appflash-signing-" + suffix + ".keychain-db")
    archive.write_bytes(data)
    archive.chmod(0o600)
    try:
        pem = checked(["openssl", "pkcs12", "-in", str(archive), "-clcerts", "-nokeys", "-passin", "env:MACOS_SIGN_PASSWORD"])
        certificate_file = state.parent / ("appflash-signing-" + suffix + ".pem")
        try:
            certificate_file.write_bytes(pem)
            actual = checked(["openssl", "x509", "-in", str(certificate_file), "-outform", "DER"])
        finally:
            if certificate_file.exists():
                certificate_file.unlink()
        expected = checked(["openssl", "x509", "-in", str(certificate), "-outform", "DER"])
        if actual != expected:
            raise ValueError("Actions certificate does not match the pinned public certificate")
        identity = hashlib.sha1(actual).hexdigest().upper()
        original = search_list()
        state.write_text(json.dumps({"format": "appflash-signing-v1", "keychain": str(keychain), "original_search_list": original}) + "\n")
        state.chmod(0o600)
        keychain_password = secrets.token_urlsafe(32)
        try:
            checked(["security", "create-keychain", "-p", keychain_password, str(keychain)])
            checked(["security", "unlock-keychain", "-p", keychain_password, str(keychain)])
            checked(["security", "set-keychain-settings", "-lut", "21600", str(keychain)])
            checked(["security", "import", str(archive), "-f", "pkcs12", "-k", str(keychain), "-P", password, "-T", "/usr/bin/codesign"])
            checked(["security", "set-key-partition-list", "-S", "apple-tool:,apple:,codesign:", "-s", "-k", keychain_password, str(keychain)])
            checked(["security", "list-keychains", "-d", "user", "-s", *original, str(keychain)])
            exported = "APPLE_SIGNING_IDENTITY=" + identity + "\nAPP_FLASH_SIGNING_KEYCHAIN=" + str(keychain) + "\nAPP_FLASH_REQUIRE_STABLE_SIGNING=1\n"
            github_env = os.environ.get("GITHUB_ENV")
            if github_env:
                with open(github_env, "a") as stream:
                    stream.write(exported)
            print("Imported fixed signing certificate SHA-256:", hashlib.sha256(actual).hexdigest())
        except Exception:
            cleanup(state)
            raise
    finally:
        archive.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["import", "cleanup"])
    parser.add_argument("--state", type=pathlib.Path, required=True)
    parser.add_argument("--certificate", type=pathlib.Path, default=pathlib.Path(__file__).parent / "macos-signing-cert.pem")
    args = parser.parse_args()
    try:
        if args.command == "import":
            import_identity(args.state.resolve(), args.certificate)
        else:
            cleanup(args.state.resolve())
    except (ValueError, OSError) as error:
        parser.exit(1, "Signing keychain operation failed: " + str(error) + "\n")


if __name__ == "__main__":
    main()
