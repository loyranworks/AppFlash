"""Exercise the real long-lived helper without requiring Accessibility access."""

import json
import pathlib
import plistlib
import select
import subprocess
import sys
import tempfile
import time
import uuid


def request(helper, value):
    helper.stdin.write(json.dumps(value) + "\n")
    helper.stdin.flush()
    assert select.select([helper.stdout], [], [], 5)[0], "Native response timed out"
    line = helper.stdout.readline()
    assert line, "Native helper exited before responding"
    return json.loads(line)


def listed(helper, path):
    reply = request(helper, {"command": "list"})
    assert "data" in reply, reply
    return any(pathlib.Path(app["path"]).resolve() == path for app in reply["data"])


def wait_listed(helper, path, expected):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if listed(helper, path) == expected:
            return
        time.sleep(0.05)
    raise AssertionError("Running application list did not refresh: expected present=" + str(expected))


def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def window_controls(helper, path):
    def snapshot():
        return request(helper, {"command": "snapshot", "path": str(path)})

    def wait_state(check):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            reply = snapshot()
            if "data" in reply and check(reply["data"]):
                return reply["data"]
            time.sleep(0.05)
        raise AssertionError("Window state did not converge: " + str(reply))

    state = wait_state(lambda state: len(state["windows"]) == 2)
    ids = [window["id"] for window in state["windows"]]
    for target in ids + [ids[0]]:
        assert request(helper, {"command": "activate", "id": target}) == {"data": True}
        wait_state(lambda state: state["active"] == target)
    for target in ids:
        assert request(helper, {"command": "minimize", "id": target}) == {"data": True}
    wait_state(lambda state: len(state["windows"]) == 2 and all(w["minimized"] for w in state["windows"]))
    for target in ids:
        assert request(helper, {"command": "activate", "id": target}) == {"data": True}
        wait_state(lambda state: state["active"] == target and any(w["id"] == target and not w["minimized"] for w in state["windows"]))


def main():
    assert sys.platform == "darwin", "This test requires macOS"
    native = pathlib.Path(sys.argv[1]).resolve()
    control_windows = "--window-control" in sys.argv[2:]
    with tempfile.TemporaryDirectory(prefix="appflash-native-test-") as temporary:
        root = pathlib.Path(temporary)
        fixture = root / "AppFlash Native Fixture.app"
        executable = fixture / "Contents/MacOS/fixture"
        executable.parent.mkdir(parents=True)
        with (fixture / "Contents/Info.plist").open("wb") as stream:
            plistlib.dump({
                "CFBundleIdentifier": "local.appflash.native-test." + uuid.uuid4().hex,
                "CFBundleExecutable": "fixture",
                "CFBundleName": "AppFlash Native Fixture",
                "CFBundlePackageType": "APPL",
            }, stream)
        source = root / "fixture.swift"
        source.write_text("""import AppKit
let app = NSApplication.shared
app.setActivationPolicy(.regular)
var windows: [NSWindow] = []
if CommandLine.arguments.contains("--windows") {
    for index in 0..<2 {
        let window = NSWindow(contentRect: NSRect(x: 100 + index * 80, y: 100, width: 360, height: 200), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "AppFlash native test \\(index + 1)"
        window.identifier = NSUserInterfaceItemIdentifier("appflash-test-\\(index)")
        window.makeKeyAndOrderFront(nil)
        windows.append(window)
    }
}
app.run()
""")
        subprocess.run(["swiftc", str(source), "-o", str(executable)], check=True)
        helper = None
        application = None
        try:
            helper = subprocess.Popen([str(native)], stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, text=True, bufsize=1)
            assert not listed(helper, fixture.resolve())
            arguments = [str(executable)] + (["--windows"] if control_windows else [])
            application = subprocess.Popen(arguments, stdout=subprocess.DEVNULL,
                                           stderr=subprocess.DEVNULL)
            wait_listed(helper, fixture.resolve(), True)
            if control_windows:
                window_controls(helper, fixture.resolve())
            stop(application)
            wait_listed(helper, fixture.resolve(), False)
            assert "error" in request(helper, {"command": "not-a-command"})
            helper.stdin.close()
            assert helper.wait(timeout=5) == 0, "Helper did not exit cleanly on stdin EOF"
            print("Native IPC, live application launch/exit updates and EOF shutdown passed")
            if control_windows:
                print("Two-window activation, minimization and restoration passed")
        finally:
            stop(application)
            stop(helper)


if __name__ == "__main__":
    main()
