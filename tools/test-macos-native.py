"""Exercise the real long-lived helper without requiring Accessibility access."""

import json
import os
import pathlib
import plistlib
import select
import signal
import shutil
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


def busy_target_controls(helper, path, application):
    startup_deadline = time.monotonic() + 5
    while True:
        initial = request(helper, {"command": "snapshot", "path": str(path)})
        if len(initial.get("data", {}).get("windows", [])) == 2 or time.monotonic() >= startup_deadline:
            break
        time.sleep(0.05)
    assert len(initial.get("data", {}).get("windows", [])) == 2, initial
    application.send_signal(signal.SIGSTOP)
    try:
        began = time.monotonic()
        reply = request(helper, {"command": "snapshot", "path": str(path)})
        elapsed = time.monotonic() - began
        assert elapsed < 0.75, {"elapsed": elapsed, "reply": reply}
        assert "响应" in reply.get("error", ""), reply
        assert "授权" not in reply["error"], "Busy is not an Accessibility authorization failure"
    finally:
        application.send_signal(signal.SIGCONT)
    restored = request(helper, {"command": "snapshot", "path": str(path)})
    assert restored.get("data", {}).get("windows") == initial["data"]["windows"], restored
    print("Paused target answered with busy in %.3fs; no false permission error; window identities retained after resume" % elapsed, flush=True)


def cold_fullscreen_controls(helper, path, application, root):
    """The target app must enter full screen BEFORE its first AX query."""
    state_file = root / "fixture-state.json"

    def fixture_state(check):
        deadline = time.monotonic() + 8
        last = None
        while time.monotonic() < deadline:
            try:
                last = json.loads(state_file.read_text())
                if check(last):
                    return last
            except (FileNotFoundError, json.JSONDecodeError):
                pass
            time.sleep(0.05)
        raise AssertionError("Cold fixture state did not converge: " + str(last))

    fixture_state(lambda state: state["launched"])
    application.send_signal(signal.SIGUSR1)
    fixture_state(lambda state: state["fullscreen_phase"] == "entered" and any(
        w["identifier"] == "appflash-test-0" and w["key"] and w["on_active_space"] for w in state["windows"]))
    first = request(helper, {"command": "snapshot", "path": str(path)})
    assert "data" in first, first
    active = first["data"]["active"]
    assert active is not None, first
    ids = [w["id"] for w in first["data"]["windows"]]
    # Before a fullscreen collapse, visiting the ordinary desktop exposes AX
    # roots that the app has never published. Do not hide or require another key.
    prepared = request(helper, {"command": "prepare-collapse", "path": str(path), "ids": ids})
    assert "data" in prepared and prepared["data"] is not None, prepared
    refreshed = prepared["data"]
    assert len(refreshed["windows"]) == 2, {"first": first, "prepared": prepared}
    target = next(w["id"] for w in refreshed["windows"] if w["id"] != active)
    assert request(helper, {"command": "activate", "id": target}) == {"data": True}
    fixture_state(lambda state: state["active"] and not state["hidden"] and state["fullscreen"] and any(
        w["identifier"] == "appflash-test-1" and w["key"] and w["on_active_space"] for w in state["windows"]))
    ids = [w["id"] for w in refreshed["windows"]]
    assert request(helper, {"command": "collapse", "ids": ids}) == {"data": True}
    fixture_state(lambda state: state["hidden"] and state["fullscreen"])
    print("Cold target AX graph: first snapshot count=" + str(len(first["data"]["windows"])) +
          "; pre-collapse refresh count=2; other exact window activated; fullscreen collapse preserved", flush=True)


def fullscreen_keyboard_controls(helper, path, application, destination_path, destination, root, rebuild_root=False):
    """Two real fullscreen apps must receive keys, including after hide/restore."""
    paths = [path, destination_path]
    processes = [application, destination]
    state_paths = [root / "fixture-state.json", root / "departure-state.json"]

    def state(index):
        try:
            return json.loads(state_paths[index].read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def wait(index, check):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            actual = state(index)
            if actual and check(actual):
                return actual
            time.sleep(0.02)
        raise AssertionError({"fixture": index, "state": state(index)})

    key_source = root / "key.swift"
    key_binary = root / "send-key"
    key_source.write_text('''import ApplicationServices
let source = CGEventSource(stateID: .hidSystemState)!
for down in [true, false] {
    let event = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: down)!
    event.flags = []
    event.post(tap: .cghidEventTap)
    usleep(20000)
}
''')
    subprocess.run(["swiftc", str(key_source), "-o", str(key_binary)], check=True)
    for index, process in enumerate(processes):
        wait(index, lambda s: s["launched"])
        process.send_signal(signal.SIGUSR1)
        wait(index, lambda s: s["fullscreen_phase"] == "entered" and s["active"])
        process.send_signal(signal.SIGUSR2)
        wait(index, lambda s: len(s["windows"]) == 1)

    def target(index):
        reply = request(helper, {"command": "snapshot", "path": str(paths[index])})
        assert "data" in reply and len(reply["data"]["windows"]) == 1, reply
        return reply["data"]["windows"][0]["id"]

    ids = [target(0), target(1)]
    for round_index in range(4):
        for index in range(2):
            if rebuild_root and index == 0:
                previous_generation = state(index)["inventory_requests"]
                previous_number = state(index)["windows"][0]["window_number"]
                processes[index].send_signal(signal.SIGINFO)
                wait(index, lambda s: s["inventory_requests"] > previous_generation)
                assert state(index)["windows"][0]["window_number"] == previous_number
            reply = request(helper, {"command": "activate", "id": ids[index]})
            assert reply == {"data": True}, {"round": round_index, "fixture": index, "reply": reply, "state": state(index)}
            actual = wait(index, lambda s: s["active"] and not s["hidden"] and s["windows"][0]["key"] and s["windows"][0]["on_active_space"])
            count = actual["test_key_count"]
            # Never send test input unless the foreground is our own fixture.
            subprocess.run([str(key_binary)], check=True)
            wait(index, lambda s: s["test_key_count"] > count)
            prepared = request(helper, {"command": "prepare-collapse", "path": str(paths[index]), "ids": [ids[index]]})
            assert "data" in prepared, prepared
            reply = request(helper, {"command": "collapse", "ids": [ids[index]]})
            assert reply == {"data": True}, reply
            wait(index, lambda s: s["hidden"] and s["fullscreen"])
        print("Two fullscreen applications received real keyboard input and hid/restored, round", round_index + 1, flush=True)


def window_controls(helper, path, application, root, fullscreen=False, check_space=False, self_hide_probe=False):
    state_file = root / "fixture-state.json"
    identities = {}
    normal_closed = False

    def snapshot():
        return request(helper, {"command": "snapshot", "path": str(path)})

    def latest_fixture():
        try:
            return json.loads(state_file.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            return None

    def fixture_state(check, label):
        deadline = time.monotonic() + 8
        last = None
        while time.monotonic() < deadline:
            last = latest_fixture()
            if last is not None and check(last):
                return last
            time.sleep(0.05)
        raise AssertionError("Fixture state did not converge (" + label + "): " + str(last))

    def wait_state(check):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            reply = snapshot()
            if "data" in reply and check(reply["data"]):
                return reply["data"]
            time.sleep(0.05)
        raise AssertionError("Window state did not converge: " + str(reply))

    def activate(target, phase, index):
        context = {
            "phase": phase, "index": index, "target": target,
            "expected_identifier": identities.get(target),
            "before_snapshot": snapshot(), "before_fixture": latest_fixture(),
        }
        try:
            context["reply"] = request(helper, {"command": "activate", "id": target})
            assert context["reply"] == {"data": True}, "Native activate rejected the request"
            wait_state(lambda state: state["active"] == target)
            # A previous timer sample can still describe the old key window.
            # Require one fixture observation after the helper response first.
            previous_sample = (latest_fixture() or {}).get("sample", -1)
            fixture_state(lambda state: state["sample"] > previous_sample, "fresh post-activation sample")
            if target not in identities:
                state = fixture_state(
                    lambda state: state["active"] and sum(window["key"] for window in state["windows"]) == 1,
                    "identify key window for native ID " + str(target),
                )
                identities[target] = next(window["identifier"] for window in state["windows"] if window["key"])
                context["expected_identifier"] = identities[target]
            if check_space:
                expected = identities[target]
                fixture_state(
                    lambda state: state["active"] and not state["hidden"] and any(
                        window["identifier"] == expected and window["key"] and window["on_active_space"]
                        for window in state["windows"]
                    ),
                    "exact target key window on active Space: " + expected,
                )
        except Exception as error:
            context["failure"] = str(error)
            context["after_snapshot"] = snapshot()
            context["after_fixture"] = latest_fixture()
            raise AssertionError("Native activation failed: " + json.dumps(context, ensure_ascii=False, sort_keys=True)) from error
        context["after_snapshot"] = snapshot()
        context["after_fixture"] = latest_fixture()
        print("Native activation: " + json.dumps(context, ensure_ascii=False, sort_keys=True), flush=True)

    state = wait_state(lambda state: len(state["windows"]) == 2)
    initial = fixture_state(lambda state: state["launched"] and len(state["windows"]) == 2, "fixture startup after applicationDidFinishLaunching")
    assert not initial["fullscreen"] and initial["fullscreen_phase"] == "windowed" and all(not window["fullscreen"] for window in initial["windows"]), "Fixture restored an unexpected full-screen window: " + json.dumps(initial, ensure_ascii=False, sort_keys=True)
    assert initial["app_window_count"] == 2, "Fixture restored extra untracked windows: " + json.dumps(initial, ensure_ascii=False, sort_keys=True)
    print("Native initial fixture: " + json.dumps(initial, ensure_ascii=False, sort_keys=True), flush=True)
    ids = [window["id"] for window in state["windows"]]
    for index, target in enumerate(ids + [ids[0]]):
        activate(target, "ordinary", index)
    assert len(set(identities.values())) == len(ids), "Native IDs did not identify distinct fixture windows: " + str(identities)
    reply = request(helper, {"command": "collapse", "ids": ids})
    assert reply == {"data": True}, {"phase": "ordinary-collapse", "reply": reply, "fixture": latest_fixture()}
    wait_state(lambda state: len(state["windows"]) == 2 and all(w["minimized"] for w in state["windows"]))
    for index, target in enumerate(ids):
        activate(target, "ordinary-restore", index)
        wait_state(lambda state: any(w["id"] == target and not w["minimized"] for w in state["windows"]))

    if fullscreen:
        application.send_signal(signal.SIGUSR1)
        entered = fixture_state(lambda state: state["fullscreen_phase"] == "entered" and state["fullscreen"], "windowDidEnterFullScreen")
        # A second fresh helper has no cached AX references. Probe before any
        # warm-helper action changes Space, and report this independent result
        # even if the subsequent activation check fails first.
        cold = subprocess.Popen(helper.args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)
        try:
            cold_reply = None
            first_snapshot_window_count = None
            cold_snapshots = 0
            cold_deadline = time.monotonic() + 3
            while time.monotonic() < cold_deadline:
                cold_reply = request(cold, {"command": "snapshot", "path": str(path)})
                cold_snapshots += 1
                if first_snapshot_window_count is None:
                    first_snapshot_window_count = len(cold_reply.get("data", {}).get("windows", []))
                if len(cold_reply.get("data", {}).get("windows", [])) == len(entered["windows"]):
                    break
                time.sleep(0.1)
            cold_context = {"phase": "cold-start-fullscreen", "first_snapshot_window_count": first_snapshot_window_count,
                            "snapshot_count": cold_snapshots, "reply": cold_reply, "fixture": latest_fixture(),
                            "expected_window_count": len(entered["windows"])}
            print("Native cold-start enumeration: " + json.dumps(cold_context, ensure_ascii=False, sort_keys=True), flush=True)
        finally:
            stop(cold)
        for index, target in enumerate(ids + [ids[0]]):
            activate(target, "fullscreen-cross-space", index)
            fixture_state(lambda state: state["fullscreen_phase"] == "entered" and state["fullscreen"], "full-screen preservation")
        fullscreen_id = next(id for id, identifier in identities.items() if identifier == "appflash-test-0")
        activate(fullscreen_id, "fullscreen-before-collapse", 0)
        if self_hide_probe:
            before_probe = latest_fixture()
            expected_requests = before_probe["self_hide_requests"] + 1
            application.send_signal(signal.SIGWINCH)
            probe_deadline = time.monotonic() + 2
            after_probe = before_probe
            while time.monotonic() < probe_deadline:
                after_probe = latest_fixture()
                if after_probe is not None and after_probe["self_hide_requests"] >= expected_requests and after_probe["hidden"]:
                    break
                time.sleep(0.05)
            probe_context = {"phase": "fixture-self-hide-from-fullscreen", "before_fixture": before_probe, "after_fixture": after_probe, "snapshot": snapshot()}
            print("Native fixture self-hide probe: " + json.dumps(probe_context, ensure_ascii=False, sort_keys=True), flush=True)
            assert after_probe is not None and after_probe["self_hide_requests"] >= expected_requests and after_probe["hidden"] and after_probe["fullscreen"], "Fixture NSApp.hide(nil) did not hide while preserving full screen: " + json.dumps(probe_context, ensure_ascii=False, sort_keys=True)
            activate(fullscreen_id, "fixture-self-hide-restore", 0)
            fixture_state(lambda state: not state["hidden"] and state["fullscreen"], "restore after fixture self-hide")
        reply = request(helper, {"command": "collapse", "ids": ids})
        assert reply == {"data": True}, {"phase": "fullscreen-collapse", "reply": reply, "fixture": latest_fixture()}
        fixture_state(lambda state: state["hidden"] and state["fullscreen"], "full-screen application hide")
        for index, target in enumerate(ids):
            activate(target, "fullscreen-unhide", index)
            fixture_state(lambda state: not state["hidden"] and state["fullscreen"], "full-screen application unhide")
        # Close and release the ordinary window while its sibling remains full
        # screen, then exercise the already-running Finder route for all-FS apps.
        normal_id = next(id for id, identifier in identities.items() if identifier == "appflash-test-1")
        application.send_signal(signal.SIGUSR2)
        fixture_state(lambda state: len(state["windows"]) == 1 and state["fullscreen"], "ordinary window closed while sibling stays full screen")
        wait_state(lambda state: len(state["windows"]) == 1 and all(window["id"] != normal_id for window in state["windows"]))
        normal_closed = True
        activate(fullscreen_id, "fullscreen-only-before-collapse", 0)
        collapse_deadline = time.monotonic() + 3
        collapse_attempts = []
        while True:
            refreshed = snapshot()
            reply = request(helper, {"command": "collapse", "ids": [fullscreen_id]})
            collapse_attempts.append({"snapshot": refreshed, "reply": reply})
            if reply == {"data": True}:
                break
            pending = str(reply.get("error", "")).startswith("仍在确认目标应用其他桌面的窗口")
            if not pending or time.monotonic() >= collapse_deadline:
                break
            time.sleep(0.1)
        if reply != {"data": True}:
            expected_inventory = (latest_fixture() or {}).get("inventory_requests", 0) + 1
            application.send_signal(signal.SIGINFO)
            fixture_state(lambda state: state["inventory_requests"] >= expected_inventory, "capture exact own-PID CG inventory after pending retry")
        print("Native full-screen-only collapse attempts: " + json.dumps({"attempts": collapse_attempts, "fixture": latest_fixture()}, ensure_ascii=False, sort_keys=True), flush=True)
        assert reply == {"data": True}, {"phase": "fullscreen-only-collapse", "attempts": collapse_attempts, "fixture": latest_fixture()}
        fixture_state(lambda state: state["hidden"] and state["fullscreen"], "full-screen-only application hide")
        activate(fullscreen_id, "fullscreen-only-unhide", 0)
        fixture_state(lambda state: not state["hidden"] and state["fullscreen"], "full-screen-only application unhide")
        application.send_signal(signal.SIGUSR1)
        fixture_state(lambda state: state["fullscreen_phase"] == "exited" and not state["fullscreen"], "windowDidExitFullScreen")

    # A window omitted by AXWindows is kept while valid; closing and releasing
    # the fixture's second NSWindow must still invalidate and remove its ID.
    closed_id = next(id for id, identifier in identities.items() if identifier == "appflash-test-1")
    if not normal_closed:
        application.send_signal(signal.SIGUSR2)
    closed_state = fixture_state(lambda state: len(state["windows"]) == 1, "closed fixture window released")
    remaining = wait_state(lambda state: len(state["windows"]) == 1 and all(window["id"] != closed_id for window in state["windows"]))
    closed_reply = request(helper, {"command": "activate", "id": closed_id})
    assert "error" in closed_reply, {"phase": "closed-window", "target": closed_id, "reply": closed_reply, "snapshot": remaining, "fixture": closed_state}
    print("Native closed-window cleanup: " + json.dumps({"closed_id": closed_id, "snapshot": remaining, "reply": closed_reply, "fixture": closed_state}, ensure_ascii=False, sort_keys=True), flush=True)
    if fullscreen:
        # Do not turn partial cold-start discovery into a passing full-screen
        # test. Keep this after the warm-cache checks to expose both boundaries.
        assert len(cold_reply.get("data", {}).get("windows", [])) == cold_context["expected_window_count"], "Cold-start full-screen enumeration incomplete: " + json.dumps(cold_context, ensure_ascii=False, sort_keys=True)


def main():
    assert sys.platform == "darwin", "This test requires macOS"
    native = pathlib.Path(sys.argv[1]).resolve()
    self_hide_probe = "--self-hide-probe" in sys.argv[2:]
    cold_fullscreen = "--cold-fullscreen" in sys.argv[2:]
    busy_target = "--busy-target" in sys.argv[2:]
    rebuild_focus_root = "--refresh-focus-root" in sys.argv[2:]
    fullscreen_keyboard = "--fullscreen-keyboard" in sys.argv[2:] or rebuild_focus_root
    fullscreen = "--fullscreen" in sys.argv[2:] or self_hide_probe or cold_fullscreen or fullscreen_keyboard
    check_space = "--check-space" in sys.argv[2:]
    control_windows = "--window-control" in sys.argv[2:] or fullscreen or check_space or busy_target
    with tempfile.TemporaryDirectory(prefix="appflash-native-test-") as temporary:
        root = pathlib.Path(temporary)
        fixture_id = uuid.uuid4().hex
        fixture_name = "AppFlash Native Fixture " + fixture_id
        executable_name = "fixture-" + fixture_id
        fixture = root / (fixture_name + ".app")
        executable = fixture / "Contents/MacOS" / executable_name
        executable.parent.mkdir(parents=True)
        with (fixture / "Contents/Info.plist").open("wb") as stream:
            plistlib.dump({
                "CFBundleIdentifier": "local.appflash.native-test." + fixture_id,
                "CFBundleExecutable": executable_name,
                "CFBundleName": fixture_name,
                "CFBundleDisplayName": fixture_name,
                "CFBundlePackageType": "APPL",
            }, stream)
        source = root / "fixture.swift"
        source.write_text("""import AppKit
// Process-local defaults only: never inherit or write a user's restore setting.
UserDefaults.standard.setVolatileDomain(["NSQuitAlwaysKeepsWindows": false], forName: UserDefaults.argumentDomain)
final class FixtureDelegate: NSObject, NSApplicationDelegate {
    var launched = false
    func applicationDidFinishLaunching(_ notification: Notification) { launched = true }
    func applicationShouldSaveSecureApplicationState(_ app: NSApplication) -> Bool { false }
    func applicationShouldRestoreSecureApplicationState(_ app: NSApplication) -> Bool { false }
}
let app = NSApplication.shared
let fixtureDelegate = FixtureDelegate()
app.delegate = fixtureDelegate
app.setActivationPolicy(.regular)
let applicationName = ProcessInfo.processInfo.processName
let mainMenu = NSMenu()
let applicationMenuItem = NSMenuItem()
let applicationMenu = NSMenu(title: applicationName)
let hideItem = NSMenuItem(title: "Hide " + applicationName, action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
hideItem.target = app
applicationMenu.addItem(hideItem)
applicationMenu.addItem(NSMenuItem.separator())
let quitItem = NSMenuItem(title: "Quit " + applicationName, action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
quitItem.target = app
applicationMenu.addItem(quitItem)
applicationMenuItem.submenu = applicationMenu
mainMenu.addItem(applicationMenuItem)
app.mainMenu = mainMenu
var windows: [NSWindow] = []
var testKeyCount = 0
let testKeyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
    if event.keyCode == 0 && event.modifierFlags.intersection(.deviceIndependentFlagsMask).isEmpty { testKeyCount += 1 }
    return event
}
if CommandLine.arguments.contains("--windows") {
    for index in 0..<2 {
        let window = NSWindow(contentRect: NSRect(x: 100 + index * 80, y: 100, width: 360, height: 200), styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "AppFlash native test \\(index + 1)"
        window.identifier = NSUserInterfaceItemIdentifier("appflash-test-\\(index)")
        window.collectionBehavior = [.fullScreenPrimary]
        window.isReleasedWhenClosed = false
        window.isRestorable = false
        window.restorationClass = nil
        window.makeKeyAndOrderFront(nil)
        windows.append(window)
    }
}
let statePath = ProcessInfo.processInfo.environment["APPFLASH_FIXTURE_STATE"]
if let statePath = statePath {
    signal(SIGUSR1, SIG_IGN)
    let signalSource = DispatchSource.makeSignalSource(signal: SIGUSR1, queue: .main)
    signalSource.setEventHandler {
        if let window = windows.first { window.makeKeyAndOrderFront(nil); window.toggleFullScreen(nil) }
    }
    signalSource.resume()
    signal(SIGUSR2, SIG_IGN)
    let closeSource = DispatchSource.makeSignalSource(signal: SIGUSR2, queue: .main)
    closeSource.setEventHandler {
        if windows.count > 1 {
            let closed = windows.removeLast()
            closed.close()
        }
    }
    closeSource.resume()
    // SIGUSR3 does not exist on macOS. SIGWINCH is private to this fixture's
    // test protocol and is sent only to its exact subprocess PID.
    signal(SIGWINCH, SIG_IGN)
    var selfHideRequests = 0
    let selfHideSource = DispatchSource.makeSignalSource(signal: SIGWINCH, queue: .main)
    selfHideSource.setEventHandler {
        selfHideRequests += 1
        app.hide(nil)
    }
    selfHideSource.resume()
    // SIGINFO requests a WindowServer inventory; the root-refresh mode also
    // rebuilds only this fixture window's AX identity.
    signal(SIGINFO, SIG_IGN)
    var inventoryRequests = 0
    var cgInventory: [[String: Any]] = []
    let inventorySource = DispatchSource.makeSignalSource(signal: SIGINFO, queue: .main)
    inventorySource.setEventHandler {
        if ProcessInfo.processInfo.environment["APPFLASH_REBUILD_FOCUS_ROOT"] != nil, let window = windows.first {
            window.orderOut(nil)
            window.orderFront(nil)
        }
        inventoryRequests += 1
        let all = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
        cgInventory = all.filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == ProcessInfo.processInfo.processIdentifier }
    }
    inventorySource.resume()
    var fullscreenPhase = "windowed"
    var observers: [NSObjectProtocol] = []
    if ProcessInfo.processInfo.environment["APPFLASH_REBUILD_FOCUS_ROOT"] != nil {
        observers.append(NotificationCenter.default.addObserver(forName: NSApplication.didUnhideNotification, object: app, queue: .main) { _ in
            if let window = windows.first { window.orderOut(nil); window.orderFront(nil) }
        })
    }
    for (name, phase) in [(NSWindow.willEnterFullScreenNotification, "entering"), (NSWindow.didEnterFullScreenNotification, "entered"), (NSWindow.willExitFullScreenNotification, "exiting"), (NSWindow.didExitFullScreenNotification, "exited")] {
        observers.append(NotificationCenter.default.addObserver(forName: name, object: windows.first, queue: .main) { _ in fullscreenPhase = phase })
    }
    var sample = 0
    let timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { _ in
        sample += 1
        let windowStates: [[String: Any]] = windows.map { window in
            ["window_number": window.windowNumber, "identifier": window.identifier?.rawValue ?? "", "key": window.isKeyWindow, "main": window.isMainWindow, "on_active_space": window.isOnActiveSpace, "occlusion": window.occlusionState.rawValue, "occlusion_visible": window.occlusionState.contains(.visible), "fullscreen": window.styleMask.contains(.fullScreen), "minimized": window.isMiniaturized, "visible": window.isVisible]
        }
        let state: [String: Any] = ["sample": sample, "test_key_count": testKeyCount, "inventory_requests": inventoryRequests, "cg_inventory": cgInventory, "self_hide_requests": selfHideRequests, "launched": fixtureDelegate.launched, "app_window_count": app.windows.count, "fullscreen": windows.first?.styleMask.contains(.fullScreen) ?? false, "fullscreen_phase": fullscreenPhase, "active": app.isActive, "hidden": app.isHidden, "windows": windowStates]
        if let data = try? JSONSerialization.data(withJSONObject: state) {
            try? data.write(to: URL(fileURLWithPath: statePath), options: .atomic)
        }
    }
    withExtendedLifetime((signalSource, closeSource, selfHideSource, inventorySource, timer, observers)) { app.run() }
} else {
    app.run()
}
""")
        subprocess.run(["swiftc", str(source), "-o", str(executable)], check=True)
        helper = None
        application = None
        departure = None
        try:
            if fullscreen:
                # Keep focus-changing tests inside disposable fixtures. The
                # destination must be a distinct app with a real desktop window.
                destination = root / "AppFlash Departure Fixture.app"
                destination_executable = destination / "Contents/MacOS/departure"
                destination_executable.parent.mkdir(parents=True)
                shutil.copy2(executable, destination_executable)
                with (destination / "Contents/Info.plist").open("wb") as stream:
                    plistlib.dump({
                        "CFBundleIdentifier": "local.appflash.departure-test." + fixture_id,
                        "CFBundleExecutable": "departure",
                        "CFBundleName": "AppFlash Departure Fixture",
                        "CFBundlePackageType": "APPL",
                    }, stream)
                departure = subprocess.Popen([str(destination_executable), "--windows"],
                                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                             env=dict(os.environ, APPFLASH_FIXTURE_STATE=str(root / "departure-state.json")))
                time.sleep(0.3)
            helper = subprocess.Popen([str(native)], stdin=subprocess.PIPE,
                                      stdout=subprocess.PIPE, text=True, bufsize=1)
            assert not listed(helper, fixture.resolve())
            arguments = [str(executable)] + (["--windows"] if control_windows else [])
            environment = dict(os.environ)
            if rebuild_focus_root:
                environment["APPFLASH_REBUILD_FOCUS_ROOT"] = "1"
            if control_windows:
                environment["APPFLASH_FIXTURE_STATE"] = str(root / "fixture-state.json")
            application = subprocess.Popen(arguments, stdout=subprocess.DEVNULL,
                                           stderr=subprocess.DEVNULL, env=environment)
            wait_listed(helper, fixture.resolve(), True)
            if control_windows:
                if fullscreen_keyboard:
                    fullscreen_keyboard_controls(helper, fixture.resolve(), application, destination.resolve(), departure, root, rebuild_focus_root)
                elif busy_target:
                    busy_target_controls(helper, fixture.resolve(), application)
                elif cold_fullscreen:
                    cold_fullscreen_controls(helper, fixture.resolve(), application, root)
                else:
                    window_controls(helper, fixture.resolve(), application, root, fullscreen, check_space, self_hide_probe)
            stop(application)
            wait_listed(helper, fixture.resolve(), False)
            assert "error" in request(helper, {"command": "not-a-command"})
            helper.stdin.close()
            assert helper.wait(timeout=5) == 0, "Helper did not exit cleanly on stdin EOF"
            print("Native IPC, live application launch/exit updates and EOF shutdown passed")
            if control_windows and not cold_fullscreen and not busy_target:
                print("Two-window exact activation, batch collapse and restoration passed")
            if fullscreen and not cold_fullscreen:
                print("Full-screen exact AX focus, hide/unhide and preservation passed")
            if check_space:
                print("Exact fixture key window on active Space verified for every activation")
        finally:
            stop(application)
            stop(helper)
            stop(departure)


if __name__ == "__main__":
    main()
