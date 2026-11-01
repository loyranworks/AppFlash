"""Run the production departure control flow with controlled OS boundaries.

There is no WindowServer or GUI input in this test. The exact production
prepareToHide body runs with controlled AppKit/AX boundaries and a settlement
stub. The real settlement helper is exercised separately by
test-macos-departure-visibility.py; GUI tests compose both real functions.
This catches a failure retrying several apps and the multi-display case where
an inactive fullscreen window remains onscreen on another display.
"""
import pathlib
import os
import subprocess
import tempfile

source = (pathlib.Path(__file__).resolve().parent.parent /
          "src-tauri/native/macos.swift").read_text()
start = source.index("func prepareToHide(")
end = source.index("\nfunc collapse(", start)
preamble = r'''
import Foundation
import CoreGraphics
typealias AXUIElement = Int
typealias CGWindowID = UInt32
let kAXFrontmostAttribute = "AXFrontmost"
let kCGWindowOwnerPID = "owner"
let kCGWindowNumber = "number"
let kCGWindowLayer = "layer"
let kCGNullWindowID = 0
let traceWindowControl = false
enum CGWindowListOption { case optionOnScreenOnly, excludeDesktopElements }
enum Policy { case regular }
final class NSRunningApplication {
    let processIdentifier: pid_t
    let bundleIdentifier: String?
    var activationPolicy = Policy.regular
    var isTerminated = false
    var isHidden = false
    var launchDate: Date?
    init(_ pid: pid_t) {
        processIdentifier = pid
        bundleIdentifier = "test.\(pid)"
        launchDate = Date(timeIntervalSince1970: Double(100 - pid))
    }
}
final class NSWorkspace {
    static let shared = NSWorkspace()
    var runningApplications: [NSRunningApplication] = []
    var frontmostApplication: NSRunningApplication?
}
struct StoredWindow {
    let element: AXUIElement
    let app: NSRunningApplication
    let minimized = false
    var number: CGWindowID? { UInt32(element * 10) }
}
struct NativeFailure: Error { let message: String }
var lastDepartureWindow: StoredWindow?
var sourceActive: Bool? = true
var sourceVisible = true
var destinationVisible: Bool? = true
var separateDisplays = true
var failActivation = true
var keepWrongFront = false
var actions: [pid_t] = []
var cgWindows: [[String: Any]] = []
var ineligible = Set<pid_t>()
var activatedForInventory: Bool?
var layoutReads = 0
var invalidLayout = false
var changeLayoutBeforeActivation = false
var changeLayoutDuringActivation = false
var reorderLayout = false
let sourceApp = NSRunningApplication(1)
func AXUIElementCreateApplication(_ pid: pid_t) -> AXUIElement { Int(pid) }
func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    if key == "AXFullScreen" { return element == 1 }
    if key == kAXFrontmostAttribute { return sourceActive }
    return nil
}
func CFEqual(_ lhs: Int, _ rhs: Int) -> Bool { lhs == rhs }
final class CrossSpaceWindowDiscovery {
    static let shared = CrossSpaceWindowDiscovery()
    func isOnscreen(_ element: AXUIElement) -> Bool? { element == 1 ? sourceVisible : destinationVisible }
}
func testWindowInventory(_ options: [CGWindowListOption], _ id: Int) -> Any? { cgWindows }
func activeQuartzDisplayBounds() -> [CGRect]? {
    layoutReads += 1
    if invalidLayout { return nil }
    let first = CGRect(x: 0, y: 0, width: 1920, height: 1080)
    let second = CGRect(x: 1920, y: 0, width: 1920, height: 1080)
    if (changeLayoutBeforeActivation && layoutReads >= 2) ||
       (changeLayoutDuringActivation && !actions.isEmpty) { return [first] }
    return reorderLayout && layoutReads >= 2 ? [second, first] : [first, second]
}
func departureWindowIsEligible(_ window: StoredWindow, deadline: Date) -> Bool {
    !ineligible.contains(window.app.processIdentifier)
}
func departureWindows(_ app: NSRunningApplication, deadline: Date) -> [StoredWindow] {
    let w = StoredWindow(element: Int(app.processIdentifier), app: app)
    return departureWindowIsEligible(w, deadline: deadline) ? [w] : []
}
func activate(_ window: StoredWindow, deadline: Date) throws {
    actions.append(window.app.processIdentifier)
    if failActivation { throw NativeFailure(message: "external activation rejected") }
    sourceActive = false
    if !keepWrongFront { NSWorkspace.shared.frontmostApplication = window.app }
}
func hasExactFocus(_ window: StoredWindow, deadline: Date) -> Bool { true }
func departureHasSettled(_ window: StoredWindow, fullScreen: [StoredWindow], deadline: Date) -> Bool {
    destinationVisible == true && (!sourceVisible || separateDisplays)
}
func reset() {
    lastDepartureWindow = nil; sourceActive = true; sourceVisible = true
    destinationVisible = true
    separateDisplays = true
    failActivation = true; keepWrongFront = false; actions = []; cgWindows = []; ineligible = []
    activatedForInventory = nil
    layoutReads = 0; invalidLayout = false; changeLayoutBeforeActivation = false
    changeLayoutDuringActivation = false; reorderLayout = false
    NSWorkspace.shared.runningApplications = [NSRunningApplication(2), NSRunningApplication(3), NSRunningApplication(4)]
    NSWorkspace.shared.frontmostApplication = sourceApp
}
func prepare(_ extra: [StoredWindow] = []) -> Bool {
    do {
        let outcome: Any = try prepareToHide(sourceApp, windows: [StoredWindow(element: 1, app: sourceApp)] + extra,
                                             deadline: Date(timeIntervalSinceNow: 0.06))
        activatedForInventory = outcome as? Bool
        return true
    } catch { return false }
}
var failures: [String] = []
func expect(_ condition: Bool, _ message: String) {
    if !condition { failures.append(message); print("FAIL:", message, "activation PIDs:", actions) }
}
'''
checks = r'''
reset()
expect(!prepare(), "Rejected activation cannot count as preparation")
expect(actions.count == 1, "One failed shortcut must not try several unrelated applications")

reset()
lastDepartureWindow = StoredWindow(element: 4, app: NSWorkspace.shared.runningApplications[2])
expect(!prepare(), "Rejected cached activation must fail")
expect(actions == [4], "Failed cached target must not fall through to other applications")

reset()
sourceActive = false
NSWorkspace.shared.frontmostApplication = NSWorkspace.shared.runningApplications[0]
expect(prepare(), "Inactive fullscreen app on another display may proceed to actual hide")
expect(actions.isEmpty, "Already inactive app must not change other display's application")
expect(activatedForInventory == false, "Allowing hide is distinct from activating a destination for inventory")

reset()
failActivation = false
expect(prepare(), "Verified target and inactive source suffice even while source remains onscreen")
expect(actions.count == 1, "Source onscreen is not a reason to continue hopping applications")
expect(activatedForInventory == true, "Successful explicit departure authorizes inventory refresh")

reset()
failActivation = false; sourceVisible = false; keepWrongFront = true
expect(!prepare(), "AX focus contradicting workspace front PID cannot count as successful departure")
expect(actions.count == 1, "Inconsistent foreground must stop rather than activate another application")

reset()
failActivation = false
lastDepartureWindow = StoredWindow(element: 4, app: NSWorkspace.shared.runningApplications[2])
cgWindows = [[kCGWindowOwnerPID: NSNumber(value: 3), kCGWindowNumber: NSNumber(value: 30), kCGWindowLayer: NSNumber(value: 0)]]
expect(prepare(), "An already visible verified destination can be selected")
expect(actions == [3], "Prefer already visible external window over an offscreen cached window")

reset()
sourceActive = nil
expect(!prepare(), "Unknown source AX state cannot authorize changing another application")
expect(actions.isEmpty, "Unknown source state must not trigger trial activations")

reset()
sourceActive = false
expect(!prepare(), "Conflicting source AX and workspace state must fail conservatively")
expect(actions.isEmpty, "Conflicting initial foreground state must not change another app")

reset()
failActivation = false; sourceVisible = false; destinationVisible = false
expect(!prepare(), "Focus alone cannot confirm departure before destination reaches an active display")
expect(actions.count == 1, "Destination still offscreen must not trigger another application")

reset()
failActivation = false; separateDisplays = false
expect(!prepare(), "Same-display source still visible means Space transition has not settled")
expect(actions.count == 1, "Same-display transition must never trigger another candidate")

reset()
sourceActive = false; separateDisplays = false
NSWorkspace.shared.frontmostApplication = NSWorkspace.shared.runningApplications[0]
expect(prepare(), "Already inactive app may attempt actual hide without forcing a Space transition")
expect(actions.isEmpty, "Already inactive source must not activate any app even during a transition")
expect(activatedForInventory == false, "Already inactive branch must not claim it completed a new departure")

reset()
sourceActive = false
NSWorkspace.shared.frontmostApplication = NSWorkspace.shared.runningApplications[0]
ineligible = [2]
expect(prepare(), "Foreground Finder desktop without a standard window must not block actual hide")
expect(actions.isEmpty, "Windowless foreground is not a reason to try unrelated applications")

reset()
failActivation = false
let otherBoundProcess = StoredWindow(element: 2, app: NSWorkspace.shared.runningApplications[0])
expect(prepare([otherBoundProcess]), "Can leave a binding containing multiple target processes")
expect(actions == [3], "A process about to be collapsed cannot be the departure destination")

reset()
failActivation = false; invalidLayout = true
expect(!prepare(), "Unavailable display layout must not authorize an activation")
expect(actions.isEmpty, "No activation before a valid display layout exists")

reset()
failActivation = false; changeLayoutBeforeActivation = true
expect(!prepare(), "Hotplug during read-only selection must abort before activation")
expect(actions.isEmpty, "Changing display layout must not cause a trial activation")

reset()
failActivation = false; changeLayoutDuringActivation = true
expect(!prepare(), "Hotplug during activation cannot claim settled inventory")
expect(actions.count == 1, "A changed layout stops without trying another target")

reset()
failActivation = false; reorderLayout = true
expect(prepare(), "Display enumeration order alone is not a layout change")

if !failures.isEmpty { exit(1) }
print("Production departure: one target only, no hopping after failures, inactive visible source accepted, foreground checked")
'''
with tempfile.TemporaryDirectory(prefix="appflash-departure-policy-") as temporary:
    root = pathlib.Path(temporary)
    swift = root / "main.swift"
    binary = root / "test"
    body = source[start:end].replace("CGWindowListCopyWindowInfo(", "testWindowInventory(")
    swift.write_text(preamble + body + checks)
    subprocess.run(["swiftc", "-module-cache-path", os.environ.get("CLANG_MODULE_CACHE_PATH", str(root / "modules")),
                    str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
