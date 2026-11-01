"""Production per-display settlement using controlled WindowServer records."""
import os
import pathlib
import subprocess
import tempfile

source = (pathlib.Path(__file__).resolve().parent.parent /
          "src-tauri/native/macos.swift").read_text()
start = source.index("func departureHasSettled(")
end = source.index("\nfunc prepareToHide(", start)
body = source[start:end]
assert body.count("CGWindowListCopyWindowInfo(") == 1
body = body.replace("CGWindowListCopyWindowInfo(", "testWindowInventory(")
preamble = r'''
import Foundation
import CoreGraphics
struct App { let processIdentifier: pid_t }
struct StoredWindow { let app: App; let number: CGWindowID? }
var displays: [CGRect]? = [CGRect(x: 0, y: 0, width: 1920, height: 1080), CGRect(x: 1920, y: 0, width: 1920, height: 1080)]
var records: [[String: Any]]? = []
func activeQuartzDisplayBounds() -> [CGRect]? { displays }
func testWindowInventory(_ options: CGWindowListOption, _ relative: CGWindowID) -> CFArray? {
    records.map { $0 as CFArray }
}
let sourceWindow = StoredWindow(app: App(processIdentifier: 101), number: 11)
let target = StoredWindow(app: App(processIdentifier: 202), number: 22)
func record(_ number: Int, _ owner: Int, _ frame: CGRect, _ onscreen: Bool) -> [String: Any] {
    [kCGWindowNumber as String: NSNumber(value: number),
     kCGWindowOwnerPID as String: NSNumber(value: owner),
     kCGWindowIsOnscreen as String: NSNumber(value: onscreen),
     kCGWindowBounds as String: ["X": frame.origin.x, "Y": frame.origin.y, "Width": frame.width, "Height": frame.height]]
}
func setup(_ sourceVisible: Bool = true, destinationX: CGFloat = 2100) {
    displays = [CGRect(x: 0, y: 0, width: 1920, height: 1080), CGRect(x: 1920, y: 0, width: 1920, height: 1080)]
    records = [record(11, 101, CGRect(x: 0, y: 0, width: 1920, height: 1080), sourceVisible),
               record(22, 202, CGRect(x: destinationX, y: 100, width: 500, height: 400), true)]
}
func ready(_ destination: StoredWindow = target, sources: [StoredWindow] = [sourceWindow]) -> Bool {
    departureHasSettled(destination, fullScreen: sources, deadline: Date(timeIntervalSinceNow: 1))
}
'''
checks = r'''
setup(); precondition(ready(), "Internal fullscreen may stay visible while external target is active")
setup(destinationX: 100); precondition(!ready(), "Same display must finish leaving fullscreen")
setup(false, destinationX: 100); precondition(ready(), "Settled ordinary desktop exposes source windows")
setup(); records![1][kCGWindowIsOnscreen as String] = false; precondition(!ready())
setup(); records![1][kCGWindowOwnerPID as String] = 999; precondition(!ready())
setup(); records![0][kCGWindowOwnerPID as String] = 999; precondition(!ready())
setup(); records![0].removeValue(forKey: kCGWindowIsOnscreen as String); precondition(ready(), "SDK: absent optional key means source is offscreen")
setup(); records![0][kCGWindowIsOnscreen as String] = "not-a-CFBoolean"; precondition(!ready())
setup(); records![0].removeValue(forKey: kCGWindowBounds as String); precondition(!ready())
setup(); records![1].removeValue(forKey: kCGWindowBounds as String); precondition(!ready())
setup(); records = nil; precondition(!ready())
setup(); displays = nil; precondition(!ready())
setup(); displays = []; precondition(!ready())
setup(destinationX: 5000); precondition(!ready(), "Hotplug stale frame outside active displays is unknown")
setup(); records![0] = record(11, 101, CGRect(x: 0, y: 0, width: 3840, height: 1080), true); precondition(!ready(), "Source spanning target display must leave it")
setup(); displays = [CGRect(x: 0, y: 0, width: 1920, height: 1080), CGRect(x: 0, y: 0, width: 1920, height: 1080)]; precondition(!ready(), "Mirrored/stale outside destination cannot pass")
setup(); displays = [CGRect(x: -1920, y: 0, width: 1920, height: 1080), CGRect(x: 0, y: 0, width: 1920, height: 1080)]
records = [record(11, 101, CGRect(x: -1920, y: 0, width: 1920, height: 1080), true), record(22, 202, CGRect(x: 100, y: 100, width: 500, height: 400), true)]
precondition(ready(), "Negative Quartz display coordinates share one coordinate space")
setup(); precondition(!ready(StoredWindow(app: target.app, number: nil)))
setup(); precondition(!ready(sources: [StoredWindow(app: sourceWindow.app, number: nil)]))
setup(); precondition(!departureHasSettled(target, fullScreen: [sourceWindow], deadline: .distantPast))

// More than two displays and multiple fullscreen windows of the same app.
for count in [3, 4, 7] {
    displays = (0..<count).map { CGRect(x: $0 * 1920, y: 0, width: 1920, height: 1080) }
    let sources = (0..<(count - 1)).map { StoredWindow(app: sourceWindow.app, number: UInt32(11 + $0)) }
    records = (0..<(count - 1)).map { record(11 + $0, 101, CGRect(x: $0 * 1920, y: 0, width: 1920, height: 1080), true) }
    records!.append(record(22, 202, CGRect(x: (count - 1) * 1920 + 100, y: 100, width: 500, height: 400), true))
    precondition(ready(sources: sources), "Any number of unrelated fullscreen displays may stay visible")
    records![count - 2] = record(11 + count - 2, 101, CGRect(x: (count - 1) * 1920, y: 0, width: 1920, height: 1080), true)
    precondition(!ready(sources: sources), "Every source on the destination display must be checked")
    records![count - 2].removeValue(forKey: kCGWindowIsOnscreen as String)
    precondition(ready(sources: sources), "Only relevant fullscreen windows need to leave")
}

setup()
displays = [CGRect(x: 0, y: -1080, width: 1920, height: 1080), CGRect(x: 0, y: 0, width: 1920, height: 1080), CGRect(x: 0, y: 1080, width: 1920, height: 1080)]
records = [record(11, 101, CGRect(x: 0, y: -1080, width: 1920, height: 1080), true), record(22, 202, CGRect(x: 100, y: 1200, width: 500, height: 400), true)]
precondition(ready(), "Vertically arranged monitors use Quartz coordinates")

// Mixed display sizes and fractional window coordinates, with no backing-scale conversion.
displays = [CGRect(x: 0, y: 0, width: 1728, height: 1117), CGRect(x: 1728, y: -200, width: 2560, height: 1440), CGRect(x: -1440, y: 0, width: 1440, height: 900)]
records = [record(11, 101, CGRect(x: 0, y: 0, width: 1728, height: 1117), true), record(22, 202, CGRect(x: 1900.5, y: 100.25, width: 600.5, height: 400.25), true)]
precondition(ready(), "No mixed-DPI AX/AppKit coordinate conversion is used")

setup(destinationX: 100)
displays = [CGRect(x: 0, y: 0, width: 1920, height: 1080), CGRect(x: 0, y: 0, width: 1920, height: 1080)]
precondition(!ready(), "Mirrored display footprints are not separate destinations")
records![0].removeValue(forKey: kCGWindowIsOnscreen as String)
precondition(ready())

// Target spans display 2/3: fullscreen on either relevant display must leave.
displays = (0..<3).map { CGRect(x: $0 * 1920, y: 0, width: 1920, height: 1080) }
let thirdSource = StoredWindow(app: sourceWindow.app, number: 12)
records = [record(11, 101, CGRect(x: 0, y: 0, width: 1920, height: 1080), true), record(12, 101, CGRect(x: 3840, y: 0, width: 1920, height: 1080), true), record(22, 202, CGRect(x: 2000, y: 100, width: 2300, height: 400), true)]
precondition(!ready(sources: [sourceWindow, thirdSource]))
records![1].removeValue(forKey: kCGWindowIsOnscreen as String)
precondition(ready(sources: [sourceWindow, thirdSource]))

displays!.removeLast()
records![2] = record(22, 202, CGRect(x: 4000, y: 100, width: 500, height: 400), true)
precondition(!ready(sources: [sourceWindow, thirdSource]), "A detached destination display is not usable")
print("Per-display departure: exact owner/window, destination onscreen, same-display transition and hotplug unknowns checked")
'''
with tempfile.TemporaryDirectory(prefix="appflash-departure-visibility-") as temporary:
    root = pathlib.Path(temporary)
    swift = root / "main.swift"
    binary = root / "test"
    swift.write_text(preamble + body + checks)
    subprocess.run(["swiftc", "-module-cache-path",
                    os.environ.get("CLANG_MODULE_CACHE_PATH", str(root / "modules")),
                    str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
