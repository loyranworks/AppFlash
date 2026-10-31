"""Verify strict integrity and the stable certificate identity of macOS code."""

import argparse
import hashlib
import pathlib
import re
import subprocess
import tempfile


def checked(arguments):
    result = subprocess.run(arguments, capture_output=True)
    if result.returncode:
        raise ValueError(result.stderr.decode(errors="replace").strip())
    return result.stdout


def verify(path, certificate, identifier):
    expected = checked(["openssl", "x509", "-in", str(certificate), "-outform", "DER"])
    sha1 = hashlib.sha1(expected).hexdigest()
    requirement = 'identifier "' + identifier + '" and certificate leaf = H"' + sha1 + '"'
    checked(["codesign", "--verify", "--deep", "--strict", "-R", "=" + requirement, str(path)])
    with tempfile.TemporaryDirectory(prefix="appflash-verify-cert-") as temporary:
        prefix = str(pathlib.Path(temporary) / "cert")
        checked(["codesign", "-d", "--extract-certificates=" + prefix, str(path)])
        actual = pathlib.Path(prefix + "0").read_bytes()
        if actual != expected:
            raise ValueError("Signing certificate does not match the pinned certificate")
    result = subprocess.run(["codesign", "-d", "-r-", str(path)], capture_output=True, text=True)
    if result.returncode:
        raise ValueError(result.stderr.strip())
    designated = result.stdout + result.stderr
    if re.search(r"\bcdhash\s", designated) or sha1 not in designated.lower():
        raise ValueError("Designated requirement must use the fixed certificate, not a build hash")
    print("Stable signature verified:", identifier, "certificate SHA-256", hashlib.sha256(actual).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=pathlib.Path)
    parser.add_argument("--certificate", type=pathlib.Path, default=pathlib.Path(__file__).parent / "macos-signing-cert.pem")
    parser.add_argument("--identifier", default="local.appflash.desktop")
    args = parser.parse_args()
    try:
        verify(args.path, args.certificate, args.identifier)
    except (ValueError, OSError) as error:
        parser.exit(1, "macOS signing verification failed: " + str(error) + "\n")


if __name__ == "__main__":
    main()
