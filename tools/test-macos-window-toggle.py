"""Real AX regression: auxiliary CG surfaces must not block known windows.

Requires a GUI login and Accessibility permission for the invoking terminal.
Run sequentially with other tests that change the foreground application.
"""

import json
import pathlib
import plistlib
import select
import subprocess
import sys
import tempfile
import time
import uuid

def stop(process):
    if process is not None and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def request(helper, value):
    helper.stdin.write(json.dumps(value) + "\n")
    helper.stdin.flush()
    assert select.select([helper.stdout], [], [], 5)[0], "Native response timed out"
    line = helper.stdout.readline()
    assert line, "Native helper exited"
    return json.loads(line)


def wait_for(read, check):
    deadline = time.monotonic() + 6
    last = None
    while time.monotonic() < deadline:
        last = read()
        if last is not None and check(last):
            return last
        time.sleep(0.05)
    raise AssertionError("State did not converge: " + str(last))


def main():
    native = pathlib.Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="appflash-toggle-") as temporary:
        root = pathlib.Path(temporary)
        fixture = root / "AppFlash Toggle Test.app"
        executable = fixture / "Contents/MacOS/toggle-test"
        executable.parent.mkdir(parents=True)
        with (fixture / "Contents/Info.plist").open("wb") as output:
            plistlib.dump({"CFBundleIdentifier": "local.appflash.toggle-test-" + uuid.uuid4().hex,
                          "CFBundleName": "AppFlash Toggle Test", "CFBundleExecutable": executable.name,
                          "CFBundlePackageType": "APPL"}, output)
        source = root / "fixture.swift"
        source.write_text(r'''
import AppKit
final class AuxiliarySurface: NSWindow {
    override func accessibilityRole() -> NSAccessibility.Role? { .unknown }
    override func accessibilityChildren() -> [Any]? { [] }
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
UserDefaults.standard.setVolatileDomain(["NSQuitAlwaysKeepsWindows": false], forName: UserDefaults.argumentDomain)
let count = Int(CommandLine.arguments[1])!
let path = CommandLine.arguments[2]
var windows: [NSWindow] = []
for index in 0..<count {
    let window = NSWindow(contentRect: NSRect(x: 130 + index * 70, y: 120, width: 350, height: 220),
                          styleMask: [.titled, .miniaturizable, .closable, .resizable], backing: .buffered, defer: false)
    window.title = "AppFlash toggle test \(index)"
    window.isRestorable = false
    window.isReleasedWhenClosed = false
    window.makeKeyAndOrderFront(nil)
    windows.append(window)
}
// An opaque WindowServer surface with no eligible AXWindow root. Real apps
// use such surfaces for rendering/chrome; the OS cannot prove their absence.
let auxiliary = AuxiliarySurface(contentRect: NSRect(x: -1200, y: -900, width: 260, height: 180),
                                 styleMask: [.borderless], backing: .buffered, defer: false)
auxiliary.isRestorable = false
auxiliary.isReleasedWhenClosed = false
auxiliary.backgroundColor = .blue
auxiliary.isOpaque = true
auxiliary.orderFrontRegardless()
app.activate(ignoringOtherApps: true)
var sample = 0
let timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { _ in
    sample += 1
    let inventory = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let surface = inventory.first { ($0[kCGWindowNumber as String] as? NSNumber)?.intValue == auxiliary.windowNumber }
    let state: [String: Any] = ["sample": sample, "active": app.isActive,
        "windows": windows.enumerated().map { ["index": $0.offset, "key": $0.element.isKeyWindow, "minimized": $0.element.isMiniaturized] as [String: Any] },
        "auxiliary_present": surface != nil, "auxiliary_layer": surface?[kCGWindowLayer as String] ?? -1,
        "auxiliary_alpha": surface?[kCGWindowAlpha as String] ?? -1]
    if let data = try? JSONSerialization.data(withJSONObject: state) {
        try? data.write(to: URL(fileURLWithPath: path), options: .atomic)
    }
}
withExtendedLifetime((timer, auxiliary, windows)) { app.run() }
''')
        subprocess.run(["swiftc", str(source), "-o", str(executable)], check=True)
        latency = "--latency" in sys.argv[2:]
        requested = [argument for argument in sys.argv[2:] if not argument.startswith("--")]
        counts = [int(requested[0])] if requested else ([1] if latency else [1, 3])
        for count in counts:
            helper = application = None
            state_path = root / ("state-" + str(count) + ".json")

            def state():
                try:
                    return json.loads(state_path.read_text())
                except (FileNotFoundError, json.JSONDecodeError):
                    return None

            try:
                application = subprocess.Popen([str(executable), str(count), str(state_path)],
                                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                initial = wait_for(state, lambda s: len(s["windows"]) == count and s["auxiliary_present"])
                assert initial["auxiliary_layer"] == 0 and initial["auxiliary_alpha"] > 0, initial
                helper = subprocess.Popen([str(native)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                          text=True, bufsize=1)

                def snapshot():
                    reply = request(helper, {"command": "snapshot", "path": str(fixture)})
                    assert "data" in reply, reply
                    return reply["data"]

                current = wait_for(snapshot, lambda s: len(s["windows"]) == count)
                ids = [window["id"] for window in current["windows"]]
                if latency:
                    durations = []
                    for _ in range(4):
                        time.sleep(1.05)
                        began = time.monotonic()
                        actual = snapshot()
                        durations.append(time.monotonic() - began)
                        assert [window["id"] for window in actual["windows"]] == ids, actual
                        assert state()["auxiliary_present"], "Unknown surface must remain present"
                    print(json.dumps({"warm_snapshot_ms": [round(value * 1000, 1) for value in durations]}), flush=True)
                    assert max(durations) < 0.10, "Known window operations repeatedly wait for unresolved auxiliary surfaces"
                    continue
                identities = {}
                for round_index in range(3):
                    visited = []
                    for target in ids:
                        reply = request(helper, {"command": "activate", "id": target})
                        assert reply == {"data": True}, reply
                        before = state()["sample"]
                        actual = wait_for(state, lambda s: s["sample"] > before and s["active"] and
                                          sum(w["key"] and not w["minimized"] for w in s["windows"]) == 1)
                        index = next(w["index"] for w in actual["windows"] if w["key"])
                        assert identities.setdefault(target, index) == index, (target, actual)
                        visited.append(index)
                    assert len(set(visited)) == count, visited
                    reply = request(helper, {"command": "collapse", "ids": ids})
                    assert reply == {"data": True}, {"count": count, "round": round_index, "reply": reply}
                    actual = wait_for(state, lambda s: all(w["minimized"] for w in s["windows"]))
                    assert actual["auxiliary_present"], "Auxiliary surface disappeared instead of being tolerated"
                    assert all(w["minimized"] for w in snapshot()["windows"])
                print(json.dumps({"windows": count, "rounds": 3, "auxiliary_surface": True,
                                  "distinct_targets": identities, "minimize_restore": "passed"}), flush=True)
            finally:
                stop(helper)
                stop(application)


if __name__ == "__main__":
    main()
