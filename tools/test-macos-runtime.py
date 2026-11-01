"""Manual macOS acceptance through the installed AppFlash global shortcut.

Usage: python3 tools/test-macos-runtime.py /Applications/AppFlash.app REPORT.json
Close AppFlash first. Requires an unlocked console and existing Accessibility
authorization. Uses disposable source/anchor apps, restores saved configuration,
and resumes AppFlash in the background. Never sends a message or user text.
The installed runtime chooses its destination normally; the test asserts that
it chose the owned Anchor. It does not inject the native fixture destination filter.
"""
import ast
import datetime
import json
import os
import pathlib
import plistlib
import re
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import uuid


def main():
    app = pathlib.Path(sys.argv[1]).resolve()
    report_path = pathlib.Path(sys.argv[2]).resolve()
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    assert info["CFBundleIdentifier"] == "local.appflash.desktop"
    executable = app / "Contents/MacOS/app-flash"
    for line in subprocess.check_output(["ps", "-axo", "pid=,comm="], text=True).splitlines():
        fields = line.strip().split(None, 1)
        assert len(fields) != 2 or fields[1] != str(executable), "Close AppFlash before this manual test"
    config = pathlib.Path.home() / "Library/Application Support/local.appflash.desktop/config.json"
    original = config.read_bytes()
    report_path.with_suffix(".config-before.json").write_bytes(original)
    log = pathlib.Path.home() / "Library/Logs/local.appflash.desktop" / (
        "appflash-" + datetime.datetime.now(datetime.timezone.utc).date().isoformat() + ".log")
    report = {"version": info["CFBundleShortVersionString"], "cycles": [], "status": "preparing"}

    def save():
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    def stop(process):
        if process and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)

    save()
    with tempfile.TemporaryDirectory(prefix="appflash-runtime-acceptance-") as temporary:
        root = pathlib.Path(temporary)
        tree = ast.parse(pathlib.Path(__file__).with_name("test-macos-native.py").read_text())
        fixtures = [node.args[0].value for node in ast.walk(tree)
                    if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
                    and node.func.attr == "write_text" and isinstance(node.func.value, ast.Name)
                    and node.func.value.id == "source" and isinstance(node.args[0], ast.Constant)]
        assert len(fixtures) == 1
        swift = fixtures[0].replace('["sample": sample,', '["pid": getpid(), "sample": sample,')
        assert '"pid": getpid()' in swift
        (root / "fixture.swift").write_text(swift)
        subprocess.run(["swiftc", "-O", str(root / "fixture.swift"), "-o", str(root / "fixture")], check=True)
        nonce = uuid.uuid4().hex
        apps = {}
        for name in ["Anchor", "Source"]:
            bundle = root / ("AppFlash Runtime " + name + ".app")
            binary = bundle / "Contents/MacOS/fixture"
            binary.parent.mkdir(parents=True)
            shutil.copy2(root / "fixture", binary)
            with (bundle / "Contents/Info.plist").open("wb") as stream:
                plistlib.dump({"CFBundleIdentifier": "local.appflash.runtime-test." + name.lower() + "." + nonce,
                               "CFBundleExecutable": "fixture", "CFBundleName": "AppFlash Runtime " + name,
                               "CFBundlePackageType": "APPL"}, stream)
            apps[name] = bundle

        def state(name):
            try:
                return json.loads((root / (name + ".json")).read_text())
            except (FileNotFoundError, json.JSONDecodeError):
                return {}

        def wait(name, check):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                actual = state(name)
                if actual and check(actual):
                    return actual
                time.sleep(.02)
            raise AssertionError({"fixture": name, "state": state(name),
                                  "stderr": (root / (name + ".stderr")).read_text()})

        runtime = None
        sender = None
        owned_pids = set()
        try:
            for name in ["Anchor", "Source"]:
                arguments = ["open", "-n", "--env", "APPFLASH_FIXTURE_STATE=" + str(root / (name + ".json")),
                             "--env", "APPFLASH_TEXT_FOCUS=1", "--stdout", str(root / (name + ".stdout")),
                             "--stderr", str(root / (name + ".stderr")), "-a", str(apps[name])]
                if name == "Source":
                    arguments.insert(1, "-g")
                subprocess.run(arguments + ["--args", "--windows"], check=True)
                launched = wait(name, lambda s: s["launched"])
                owned_pids.add(launched["pid"])
                report.setdefault("fixture_pids", {})[name] = launched["pid"]
                save()
            source_pid = state("Source")["pid"]
            os.kill(source_pid, signal.SIGUSR1)
            wait("Source", lambda s: s["active"] and s["fullscreen_phase"] == "entered")
            os.kill(source_pid, signal.SIGUSR2)
            wait("Source", lambda s: len(s["windows"]) == 1 and s["windows"][0]["key"])
            candidate = json.loads(original)
            candidate["prefix"] = ["Control", "Alt", "Shift", "Super"]
            candidate["apps"] = [{"id": "runtime-owned-source", "name": "Runtime acceptance fixture",
                                  "path": str(apps["Source"]), "letter": "K"}]
            config.write_text(json.dumps(candidate, indent=2) + "\n")
            runtime = subprocess.Popen([str(executable), "--background"],
                                       stdout=(root / "runtime.log").open("w"), stderr=subprocess.STDOUT)
            time.sleep(1)
            key_source = r'''import AppKit
import ApplicationServices
setbuf(stdout, nil)
let workspace = NSWorkspace.shared
let sourcePID = Int32(CommandLine.arguments[1])!
let anchorPID = Int32(CommandLine.arguments[2])!
let finderPID = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").first?.processIdentifier
let eventSource = CGEventSource(stateID: .hidSystemState)!
func front() -> pid_t { workspace.frontmostApplication?.processIdentifier ?? 0 }
func available() -> Bool {
    guard let session = CGSessionCopyCurrentDictionary() as? [String: Any] else { return false }
    return (session["CGSSessionScreenIsLocked"] as? NSNumber)?.boolValue != true &&
           (session[kCGSessionOnConsoleKey as String] as? NSNumber)?.boolValue == true
}
func key(_ code: CGKeyCode, _ down: Bool, _ flags: CGEventFlags) {
    let event = CGEvent(keyboardEventSource: eventSource, virtualKey: code, keyDown: down)!
    event.flags = flags; event.post(tap: .cghidEventTap); usleep(20000)
}
print("ready")
while let line = readLine() {
    RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.005))
    guard available(), front() == sourcePID || front() == anchorPID || front() == finderPID else { exit(2) }
    if line == "flash" {
        let expected = front()
        let modifiers: [(CGKeyCode, CGEventFlags)] = [(59,.maskControl),(58,.maskAlternate),(56,.maskShift),(55,.maskCommand)]
        var flags = CGEventFlags()
        for (code, flag) in modifiers { flags.insert(flag); key(code, true, flags) }
        usleep(150000)
        if available() && front() == expected { key(40, true, flags); key(40, false, flags) }
        for (code, flag) in modifiers.reversed() { flags.remove(flag); key(code, false, flags) }
    } else { exit(3) }
    print("sent")
}
'''
            (root / "keys.swift").write_text(key_source)
            subprocess.run(["swiftc", "-O", str(root / "keys.swift"), "-o", str(root / "keys")], check=True)
            sender = subprocess.Popen([str(root / "keys"), str(source_pid), str(state("Anchor")["pid"])],
                                      stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)
            assert select.select([sender.stdout], [], [], 5)[0]
            assert sender.stdout.readline().strip() == "ready"

            def flash():
                before = len(log.read_text()) if log.exists() else 0
                sender.stdin.write("flash\n")
                sender.stdin.flush()
                assert select.select([sender.stdout], [], [], 2)[0]
                assert sender.stdout.readline().strip() == "sent", "Fixture lost foreground or session locked"
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    lines = log.read_text()[before:].splitlines()
                    outcomes = [line for line in lines if "[switch.done]" in line or "[switch.error]" in line]
                    if outcomes and any("TRACE focus target_pid=" in line for line in lines):
                        assert len(outcomes) == 1 and "[switch.done]" in outcomes[0], lines
                        assert apps["Source"].name in outcomes[0], lines
                        return lines
                    time.sleep(.02)
                raise AssertionError("Installed runtime shortcut did not complete")

            for cycle in range(3):
                wait("Source", lambda s: s["active"] and not s["hidden"] and s["fullscreen"])
                hide_lines = flash()
                hidden = wait("Source", lambda s: s["hidden"] and s["fullscreen"])
                restore_lines = flash()
                restored = wait("Source", lambda s: s["active"] and not s["hidden"] and s["fullscreen"] and
                                s["windows"][0]["key"] and s["windows"][0]["on_active_space"])
                departure_pids = re.findall(r"TRACE focus target_pid=(\d+)", "\n".join(hide_lines))
                restored_pids = re.findall(r"TRACE focus target_pid=(\d+)", "\n".join(restore_lines))
                assert departure_pids == [str(report["fixture_pids"]["Anchor"])], hide_lines
                assert restored_pids == [str(source_pid)], restore_lines
                report["cycles"].append({"cycle": cycle + 1, "hidden": hidden["hidden"],
                                         "fullscreen_preserved": restored["fullscreen"],
                                         "departure_pids": departure_pids,
                                         "hide_events": hide_lines, "restore_events": restore_lines})
                save()
                print("Installed global shortcut fullscreen hide/restore passed, cycle", cycle + 1, flush=True)
            report["status"] = "completed"
        except BaseException as error:
            report["status"] = "failed"
            report["error"] = str(error)
            raise
        finally:
            try:
                stop(sender)
                stop(runtime)
                for name in ["Source", "Anchor"]:
                    pid = state(name).get("pid")
                    if pid:
                        owned_pids.add(pid)
                for pid in owned_pids:
                    comm = subprocess.run(["ps", "-p", str(pid), "-o", "comm="], capture_output=True, text=True)
                    if str(root) in comm.stdout:
                        try:
                            os.kill(pid, signal.SIGTERM)
                        except ProcessLookupError:
                            pass
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    still_owned = [pid for pid in owned_pids if str(root) in subprocess.run(
                        ["ps", "-p", str(pid), "-o", "comm="], capture_output=True, text=True).stdout]
                    if not still_owned:
                        break
                    time.sleep(.05)
                report["fixtures_stopped"] = not still_owned
                if still_owned:
                    report["status"] = "failed"
                    report["error"] = "Owned fixtures did not exit after cleanup"
                    raise RuntimeError(report["error"])
            finally:
                config.write_bytes(original)
                report["config_restored"] = config.read_bytes() == original
                subprocess.Popen([str(executable), "--background"], stdout=subprocess.DEVNULL,
                                 stderr=subprocess.DEVNULL, start_new_session=True)
                report["original_runtime_restart_requested"] = True
                save()


if __name__ == "__main__":
    main()
