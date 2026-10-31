import AppKit
import ApplicationServices
import Darwin

struct NativeFailure: Error { let message: String }
struct StoredWindow { let element: AXUIElement; let app: NSRunningApplication; let minimized: Bool }
var stored: [UInt64: StoredWindow] = [:]
var nextID: UInt64 = 1
let traceWindowControl = ProcessInfo.processInfo.environment["APPFLASH_TRACE_WINDOW_CONTROL"] != nil

func readAttribute(_ element: AXUIElement, _ key: String) -> (AXError, CFTypeRef?) {
    let began = ProcessInfo.processInfo.systemUptime
    var value: CFTypeRef?
    let status = AXUIElementCopyAttributeValue(element, key as CFString, &value)
    if traceWindowControl && ProcessInfo.processInfo.systemUptime - began > 0.05 {
        FileHandle.standardError.write(Data("TRACE AX \(key) \(ProcessInfo.processInfo.systemUptime - began)s status=\(status.rawValue)\n".utf8))
    }
    return (status, value)
}

func attribute(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    let (status, value) = readAttribute(element, key)
    return status == .success ? value : nil
}

// Pure classification of off-screen WindowServer chrome. This never suppresses
// a known eligible AXWindow; it only narrows supplemental discovery evidence.
func isOffscreenChromeSurface(frame: CGRect, offscreen: Bool, alpha: Double?, menuBarFrames: [CGRect], activeDisplayBounds: [CGRect]? = nil) -> Bool {
    let outsideDisplays = activeDisplayBounds.map { displays in
        !displays.isEmpty && displays.allSatisfy { display in
            let overlap = frame.intersection(display)
            return overlap.isNull || overlap.width == 0 || overlap.height == 0
        }
    } ?? false
    return (alpha == 0 && (offscreen || outsideDisplays)) || (offscreen && menuBarFrames.contains(frame))
}

func activeQuartzDisplayBounds() -> [CGRect]? {
    var count: UInt32 = 0
    guard CGGetActiveDisplayList(0, nil, &count) == .success, count > 0 else { return nil }
    var displays = [CGDirectDisplayID](repeating: 0, count: Int(count))
    var actual: UInt32 = 0
    guard CGGetActiveDisplayList(count, &displays, &actual) == .success, actual > 0, actual <= count else { return nil }
    let bounds = displays.prefix(Int(actual)).map { CGDisplayBounds($0) }
    guard bounds.allSatisfy({ $0.origin.x.isFinite && $0.origin.y.isFinite && $0.width.isFinite && $0.height.isFinite && $0.width > 0 && $0.height > 0 }) else { return nil }
    return bounds
}

// Compatibility discovery is isolated here: public AXWindow lists omit other
// Spaces on this OS, even with AXChildren/main/focused merged. These optional
// private symbols can change or disappear; failure leaves public discovery intact.
// Token wire format (not object memory layout): pid:u32, reserved:u32,
// "coco":u32, AX element number:u64, little endian. Format reference:
// https://github.com/lwouis/alt-tab-macos/blob/master/src/macos/api-wrappers/AXUIElement.swift
// This implementation scans only exact missing WindowServer IDs, never titles
// or approximate positions. A time-limited scan is best effort, not proof that
// every possible remote AX element has been inspected. Unresolved WindowServer
// surfaces may have no AX root at all; they must not block operations on confirmed
// AX windows. Keep scanning on later snapshots to supplement the public lists.
// Explicit nonstandard AXWindow
// roots are excluded only after classification and revalidated on later snapshots.
private final class CrossSpaceWindowDiscovery {
    typealias WindowIDFunction = @convention(c) (AXUIElement, UnsafeMutablePointer<CGWindowID>) -> Int32
    typealias TokenFunction = @convention(c) (CFData) -> Unmanaged<AXUIElement>?
    struct Progress {
        var cursor: UInt64 = 0
        var lastScan: TimeInterval = -.infinity
        var missing = Set<CGWindowID>()
        var nonStandard: [CGWindowID: AXUIElement] = [:]
        var verifiedRoots: [CGWindowID: AXUIElement] = [:]
        var retired = Set<CGWindowID>()
    }
    private enum Classification { case eligible, excluded, notWindow, unknown }
    static let shared = CrossSpaceWindowDiscovery()
    private let windowID: WindowIDFunction?
    private let fromToken: TokenFunction?
    private var progress: [pid_t: Progress] = [:]

    private init() {
        // Keep the framework loaded as long as the resolved function pointers.
        let library = dlopen("/System/Library/Frameworks/ApplicationServices.framework/Frameworks/HIServices.framework/HIServices", RTLD_LAZY | RTLD_LOCAL)
        if let library = library,
           let idSymbol = dlsym(library, "_AXUIElementGetWindow"),
           let tokenSymbol = dlsym(library, "_AXUIElementCreateWithRemoteToken") {
            windowID = unsafeBitCast(idSymbol, to: WindowIDFunction.self)
            fromToken = unsafeBitCast(tokenSymbol, to: TokenFunction.self)
        } else {
            windowID = nil
            fromToken = nil
        }
    }

    func removeExited(_ livePIDs: Set<pid_t>) {
        progress = progress.filter { livePIDs.contains($0.key) }
    }

    func isOnscreen(_ window: AXUIElement) -> Bool? {
        guard let getID = windowID else { return nil }
        var id: CGWindowID = 0
        guard getID(window, &id) == AXError.success.rawValue, id != 0,
              let visible = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else { return nil }
        return visible.contains { ($0[kCGWindowNumber as String] as? NSNumber)?.uint32Value == id }
    }

    private func classify(_ window: AXUIElement, deadline: TimeInterval) -> Classification {
        guard ProcessInfo.processInfo.systemUptime < deadline else { return .unknown }
        let (roleStatus, role) = readAttribute(window, kAXRoleAttribute)
        // A descendant with the same containing ID cannot exclude its root.
        guard roleStatus == .success else { return .unknown }
        guard role as? String == kAXWindowRole else { return .notWindow }
        guard ProcessInfo.processInfo.systemUptime < deadline else { return .unknown }
        let (minimizedStatus, minimized) = readAttribute(window, kAXMinimizedAttribute)
        if minimizedStatus == .success, minimized as? Bool == true { return .eligible }
        guard ProcessInfo.processInfo.systemUptime < deadline else { return .unknown }
        let (subroleStatus, subrole) = readAttribute(window, kAXSubroleAttribute)
        guard subroleStatus == .success, let subrole = subrole as? String else { return .unknown }
        if subrole == kAXStandardWindowSubrole { return .eligible }
        // Explicit root + nonstandard subrole + confirmed nonminimized.
        return minimizedStatus == .success && minimized as? Bool == false ? .excluded : .unknown
    }

    private func menuBarFrames(pid: pid_t, known: [AXUIElement], deadline: TimeInterval) -> [CGRect] {
        let application = AXUIElementCreateApplication(pid)
        AXUIElementSetMessagingTimeout(application, 0.005)
        var bars = known
        if let value = attribute(application, kAXMenuBarAttribute), CFGetTypeID(value) == AXUIElementGetTypeID() {
            bars.append(unsafeBitCast(value, to: AXUIElement.self))
        }
        AXUIElementSetMessagingTimeout(application, 0)
        var seen: [AXUIElement] = []
        var frames: [CGRect] = []
        for bar in bars {
            if ProcessInfo.processInfo.systemUptime >= deadline { break }
            if seen.contains(where: { CFEqual($0, bar) }) { continue }
            seen.append(bar)
            AXUIElementSetMessagingTimeout(bar, 0.005)
            defer { AXUIElementSetMessagingTimeout(bar, 0) }
            guard attribute(bar, kAXRoleAttribute) as? String == kAXMenuBarRole,
                  let positionValue = attribute(bar, kAXPositionAttribute), CFGetTypeID(positionValue) == AXValueGetTypeID(),
                  let sizeValue = attribute(bar, kAXSizeAttribute), CFGetTypeID(sizeValue) == AXValueGetTypeID() else { continue }
            let position = unsafeBitCast(positionValue, to: AXValue.self)
            let size = unsafeBitCast(sizeValue, to: AXValue.self)
            var point = CGPoint.zero
            var extent = CGSize.zero
            guard AXValueGetType(position) == .cgPoint, AXValueGetType(size) == .cgSize,
                  AXValueGetValue(position, .cgPoint, &point), AXValueGetValue(size, .cgSize, &extent),
                  point.x.isFinite, point.y.isFinite, extent.width.isFinite, extent.height.isFinite,
                  extent.width > 0, extent.height > 0 else { continue }
            frames.append(CGRect(origin: point, size: extent))
        }
        return frames
    }

    func additionalWindows(pid: pid_t, known: [AXUIElement]) -> [AXUIElement] {
        let deadline = ProcessInfo.processInfo.systemUptime + 0.15
        guard let inventory = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
            return []
        }
        var missing = Set<CGWindowID>()
        var inventoryFrames: [CGWindowID: CGRect] = [:]
        var inventoryOffscreen = Set<CGWindowID>()
        var observedCGIDs = Set<CGWindowID>()
        let activeDisplays = activeQuartzDisplayBounds()
        for record in inventory {
            guard (record[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid,
                  let id = (record[kCGWindowNumber as String] as? NSNumber)?.uint32Value else { continue }
            observedCGIDs.insert(id)
            guard (record[kCGWindowLayer as String] as? NSNumber)?.intValue == 0,
                  let bounds = record[kCGWindowBounds as String] as? [String: Any],
                  let rect = CGRect(dictionaryRepresentation: bounds as CFDictionary),
                  rect.origin.x.isFinite, rect.origin.y.isFinite,
                  rect.width.isFinite, rect.height.isFinite, rect.width > 0, rect.height > 0 else { continue }
            let offscreen = (record[kCGWindowIsOnscreen as String] as? NSNumber)?.boolValue != true
            let alpha = (record[kCGWindowAlpha as String] as? NSNumber)?.doubleValue
            // Exact zero plus off-screen metadata or zero area on every active
            // Quartz display proves no pixels. Known AX windows stay candidates.
            if isOffscreenChromeSurface(frame: rect, offscreen: offscreen, alpha: alpha, menuBarFrames: [], activeDisplayBounds: activeDisplays) { continue }
            missing.insert(id)
            inventoryFrames[id] = rect
            if offscreen { inventoryOffscreen.insert(id) }
        }
        if var state = progress[pid] {
            // A retired ID is evidence only for this uninterrupted inventory
            // lifetime. Its disappearance drops both the proof and retirement.
            state.verifiedRoots = state.verifiedRoots.filter { observedCGIDs.contains($0.key) }
            state.retired.formIntersection(observedCGIDs)
            progress[pid] = state
        }
        guard !missing.isEmpty else { return [] }
        // Without either optional symbol, public enumeration/activation remains
        // usable, including operations on its confirmed windows.
        guard let getID = windowID, let create = fromToken else {
            return []
        }
        let inventoryIDs = missing
        var state = progress[pid] ?? Progress()
        var seenKnown: [AXUIElement] = []
        var currentValidRoots = Set<CGWindowID>()
        for window in known {
            if ProcessInfo.processInfo.systemUptime >= deadline { break }
            if seenKnown.contains(where: { CFEqual($0, window) }) { continue }
            seenKnown.append(window)
            var id: CGWindowID = 0
            AXUIElementSetMessagingTimeout(window, 0.005)
            let status = getID(window, &id)
            let classification = status == AXError.success.rawValue && observedCGIDs.contains(id) ? classify(window, deadline: deadline) : .unknown
            AXUIElementSetMessagingTimeout(window, 0)
            switch classification {
            case .eligible:
                missing.remove(id)
                currentValidRoots.insert(id)
                state.verifiedRoots[id] = window
            case .excluded:
                missing.remove(id)
                currentValidRoots.insert(id)
                state.verifiedRoots[id] = window
                state.nonStandard[id] = window
            case .notWindow, .unknown: break
            }
        }
        // AXRole.invalidUIElement is conclusive for a previously verified
        // root, even when WindowServer keeps that same CG surface after close.
        // A new valid root with the same ID always wins over retirement. There
        // is no title/position matching and no retirement of unknown CG IDs.
        for (id, root) in state.verifiedRoots where !currentValidRoots.contains(id) {
            if ProcessInfo.processInfo.systemUptime >= deadline { break }
            AXUIElementSetMessagingTimeout(root, 0.005)
            let (status, _) = readAttribute(root, kAXRoleAttribute)
            AXUIElementSetMessagingTimeout(root, 0)
            if status == .invalidUIElement {
                state.retired.insert(id)
                state.verifiedRoots.removeValue(forKey: id)
            }
        }
        state.retired.subtract(currentValidRoots)
        missing.subtract(state.retired.intersection(inventoryOffscreen))
        state.nonStandard = state.nonStandard.filter { inventoryIDs.contains($0.key) }
        var found: [AXUIElement] = []
        for (id, window) in state.nonStandard where missing.contains(id) {
            if ProcessInfo.processInfo.systemUptime >= deadline { break }
            AXUIElementSetMessagingTimeout(window, 0.005)
            var currentID: CGWindowID = 0
            let validID = getID(window, &currentID) == AXError.success.rawValue && currentID == id
            let classification = validID ? classify(window, deadline: deadline) : .unknown
            AXUIElementSetMessagingTimeout(window, 0)
            switch classification {
            case .eligible:
                found.append(window)
                missing.remove(id)
                state.verifiedRoots[id] = window
                state.retired.remove(id)
                state.nonStandard.removeValue(forKey: id)
            case .excluded:
                missing.remove(id)
                state.verifiedRoots[id] = window
                state.retired.remove(id)
            case .unknown, .notWindow: state.nonStandard.removeValue(forKey: id)
            }
        }
        // Some WindowServer menu-bar proxy surfaces have normal layer 0 and no
        // AXWindow root. Classify only unresolved IDs against the app's actual
        // AXMenuBar frame. Known eligible roots were handled above and always
        // take priority; this never maps an activation target by geometry.
        // No title, fixed height, NSStatusBar thickness, or screen approximation.
        let barFrames = menuBarFrames(pid: pid, known: known, deadline: deadline)
        let menuBarSurfaces = Set(missing.filter { id in
            guard let rect = inventoryFrames[id] else { return false }
            return isOffscreenChromeSurface(frame: rect, offscreen: inventoryOffscreen.contains(id), alpha: nil, menuBarFrames: barFrames)
        })
        missing.subtract(menuBarSurfaces)
        guard !missing.isEmpty else {
            state.missing.removeAll()
            progress[pid] = state
            return found
        }
        // A newly missing CG ID may belong to an element before the saved cursor.
        if !missing.isSubset(of: state.missing) { state.cursor = 0 }
        state.missing = missing
        let now = ProcessInfo.processInfo.systemUptime
        guard now - state.lastScan >= 1 else {
            progress[pid] = state
            return found
        }
        state.lastScan = now
        scan: while !missing.isEmpty && ProcessInfo.processInfo.systemUptime < deadline {
            let number = state.cursor
            state.cursor &+= 1
            var token = Data()
            var owner = UInt32(bitPattern: pid).littleEndian
            var reserved: UInt32 = 0
            var marker = UInt32(0x636f636f).littleEndian
            var elementNumber = number.littleEndian
            withUnsafeBytes(of: &owner) { token.append(contentsOf: $0) }
            withUnsafeBytes(of: &reserved) { token.append(contentsOf: $0) }
            withUnsafeBytes(of: &marker) { token.append(contentsOf: $0) }
            withUnsafeBytes(of: &elementNumber) { token.append(contentsOf: $0) }
            guard let window = create(token as CFData)?.takeRetainedValue() else { continue }
            AXUIElementSetMessagingTimeout(window, 0.005)
            var id: CGWindowID = 0
            let status = getID(window, &id)
            if status == AXError.cannotComplete.rawValue { state.cursor = number; break }
            guard status == AXError.success.rawValue, missing.contains(id), ProcessInfo.processInfo.systemUptime < deadline else { continue }
            let classification = classify(window, deadline: deadline)
            AXUIElementSetMessagingTimeout(window, 0)
            switch classification {
            case .eligible:
                found.append(window)
                missing.remove(id)
                state.verifiedRoots[id] = window
                state.retired.remove(id)
            case .excluded:
                state.nonStandard[id] = window
                missing.remove(id)
                state.verifiedRoots[id] = window
                state.retired.remove(id)
            case .notWindow: break
            case .unknown:
                state.cursor = number
                break scan
            }
        }
        state.missing = missing
        progress[pid] = state
        return found
    }
}

func requireAccess() throws {
    if !AXIsProcessTrusted() { throw NativeFailure(message: "请在系统设置的辅助功能（设备控制和数据访问）中授权 AppFlash，然后退出并重新打开 AppFlash。") }
}

func checkedSet(_ element: AXUIElement, _ key: String, _ value: CFTypeRef) throws {
    let status = AXUIElementSetAttributeValue(element, key as CFString, value)
    if status == .cannotComplete { throw NativeFailure(message: "目标应用暂时没有响应，请在桌面切换完成后重试。") }
    if status != .success { throw NativeFailure(message: "目标应用不允许修改窗口状态。") }
}

func setHidden(_ app: NSRunningApplication, _ hidden: Bool, deadline suppliedDeadline: Date? = nil) throws {
    let application = AXUIElementCreateApplication(app.processIdentifier)
    let (initialStatus, initialValue) = readAttribute(application, kAXHiddenAttribute)
    if initialStatus == .success, let current = initialValue as? Bool, current == hidden { return }

    // These methods report whether a request was sent, not its final state.
    // Some apps reject the AppKit request while exposing writable AXHidden.
    let accepted = hidden ? app.hide() : app.unhide()
    var fallbackStatus: AXError?
    if !accepted {
        fallbackStatus = AXUIElementSetAttributeValue(application, kAXHiddenAttribute as CFString, hidden ? kCFBooleanTrue : kCFBooleanFalse)
    }
    let deadline = suppliedDeadline ?? Date(timeIntervalSinceNow: 1.2)
    var refreshed = false
    var retriedHide = false
    while true {
        let (status, value) = readAttribute(application, kAXHiddenAttribute)
        if status == .success, let current = value as? Bool {
            if current == hidden { return }
        } else if refreshed && app.isHidden == hidden {
            // isHidden is cached until a main-runloop turn; only consult it
            // after yielding when AXHidden is unavailable for this app.
            return
        }
        if hidden && refreshed && !retriedHide && status == .success && value as? Bool == false,
           attribute(application, kAXFrontmostAttribute) as? Bool == false {
            retriedHide = true
            _ = app.hide()
            fallbackStatus = AXUIElementSetAttributeValue(application, kAXHiddenAttribute as CFString, kCFBooleanTrue)
        }
        if Date() >= deadline {
            let action = hidden ? "隐藏" : "恢复"
            throw NativeFailure(message: "系统未能\(action)目标应用（AppKit请求 \(accepted)，AXHidden读取 \(status.rawValue)，设置 \(fallbackStatus.map { String($0.rawValue) } ?? "未使用")）")
        }
        RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.01))
        refreshed = true
    }
}

var lastDepartureWindow: StoredWindow?

func departureWindowIsEligible(_ window: StoredWindow, deadline: Date) -> Bool {
    guard Date() < deadline, !window.app.isTerminated, !window.app.isHidden,
          attribute(AXUIElementCreateApplication(window.app.processIdentifier), kAXHiddenAttribute) as? Bool == false,
          Date() < deadline, attribute(window.element, kAXRoleAttribute) as? String == kAXWindowRole,
          Date() < deadline, attribute(window.element, kAXSubroleAttribute) as? String == kAXStandardWindowSubrole,
          Date() < deadline, attribute(window.element, kAXMinimizedAttribute) as? Bool == false else { return false }
    return true
}

func departureWindows(_ app: NSRunningApplication, deadline: Date) -> [StoredWindow] {
    guard Date() < deadline else { return [] }
    let application = AXUIElementCreateApplication(app.processIdentifier)
    var roots = attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
    for key in [kAXMainWindowAttribute, kAXFocusedWindowAttribute] where Date() < deadline {
        if let value = attribute(application, key), CFGetTypeID(value) == AXUIElementGetTypeID() {
            roots.append(unsafeBitCast(value, to: AXUIElement.self))
        }
    }
    var result: [StoredWindow] = []
    for root in roots where Date() < deadline {
        if result.contains(where: { CFEqual($0.element, root) }) { continue }
        let window = StoredWindow(element: root, app: app, minimized: false)
        if departureWindowIsEligible(window, deadline: deadline) { result.append(window) }
    }
    return result
}

func prepareToHide(_ app: NSRunningApplication, windows: [StoredWindow], deadline: Date) throws {
    let appWindows = windows.filter { $0.app.processIdentifier == app.processIdentifier }
    let fullScreen = appWindows.filter { attribute($0.element, "AXFullScreen") as? Bool == true }
    guard !fullScreen.isEmpty else { return }
    let discovery = CrossSpaceWindowDiscovery.shared
    let source = AXUIElementCreateApplication(app.processIdentifier)
    func sourceIsAway() -> Bool {
        attribute(source, kAXFrontmostAttribute) as? Bool == false && fullScreen.allSatisfy { discovery.isOnscreen($0.element) == false }
    }
    if sourceIsAway() { return }
    // Activating Finder without a real window can leave its menu bar active ON
    // the source's fullscreen Space. Raise a verified window in another running
    // app instead. Never launch, unminimize a destination, exit fullscreen, move
    // windows, or change desktop preferences. A successful destination is cached.
    var attempted: [AXUIElement] = []
    func tryDestination(_ window: StoredWindow) -> Bool {
        let attemptDeadline = min(deadline, Date(timeIntervalSinceNow: 0.7))
        guard window.app.processIdentifier != app.processIdentifier,
              !attempted.contains(where: { CFEqual($0, window.element) }),
              departureWindowIsEligible(window, deadline: attemptDeadline) else { return false }
        attempted.append(window.element)
        do { try activate(window, deadline: attemptDeadline) } catch { return false }
        while Date() < attemptDeadline {
            if hasExactFocus(window, deadline: attemptDeadline) && sourceIsAway() {
                lastDepartureWindow = window
                return true
            }
            RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.01))
        }
        return false
    }
    if let cached = lastDepartureWindow {
        if tryDestination(cached) { return }
        lastDepartureWindow = nil
    }
    let apps = NSWorkspace.shared.runningApplications.filter {
        $0.activationPolicy == .regular && !$0.isTerminated && !$0.isHidden && $0.processIdentifier != app.processIdentifier
    }.sorted {
        let leftFinder = $0.bundleIdentifier == "com.apple.finder"
        let rightFinder = $1.bundleIdentifier == "com.apple.finder"
        if leftFinder != rightFinder { return leftFinder }
        return ($0.launchDate ?? .distantPast) > ($1.launchDate ?? .distantPast)
    }
    for destination in apps where Date() < deadline {
        for window in departureWindows(destination, deadline: deadline) where Date() < deadline {
            if tryDestination(window) { return }
        }
    }
    throw NativeFailure(message: "未能切离目标全屏桌面；请保持另一个应用窗口可用后重试，目标应用尚未收起。")
}

func collapse(_ windows: [StoredWindow]) throws {
    let deadline = Date(timeIntervalSinceNow: 2)
    var apps: [pid_t: NSRunningApplication] = [:]
    for window in windows { apps[window.app.processIdentifier] = window.app }
    var fullScreen = windows.contains { attribute($0.element, "AXFullScreen") as? Bool == true }
    for app in apps.values {
        let element = AXUIElementCreateApplication(app.processIdentifier)
        guard let all = attribute(element, kAXWindowsAttribute) as? [AXUIElement] else {
            throw NativeFailure(message: "无法读取目标应用全屏窗口状态")
        }
        if all.contains(where: { attribute($0, "AXFullScreen") as? Bool == true }) { fullScreen = true }
    }
    if fullScreen {
        // Minimizing or changing AXFullScreen would destroy the user's Space.
        for app in apps.values {
            try prepareToHide(app, windows: windows, deadline: deadline)
            try setHidden(app, true, deadline: deadline)
        }
    } else {
        var failures: [String] = []
        for window in windows where attribute(window.element, kAXMinimizedAttribute) as? Bool != true {
            do { try checkedSet(window.element, kAXMinimizedAttribute, kCFBooleanTrue) }
            catch { failures.append((error as? NativeFailure)?.message ?? error.localizedDescription) }
        }
        if !failures.isEmpty { throw NativeFailure(message: failures.joined(separator: "；")) }
    }
}

func hasExactFocus(_ window: StoredWindow, deadline: Date) -> Bool {
    let system = AXUIElementCreateSystemWide()
    guard Date() < deadline,
          let focusedApp = attribute(system, kAXFocusedApplicationAttribute),
          CFGetTypeID(focusedApp) == AXUIElementGetTypeID() else { return false }
    let application = unsafeBitCast(focusedApp, to: AXUIElement.self)
    var pid: pid_t = 0
    guard AXUIElementGetPid(application, &pid) == .success,
          pid == window.app.processIdentifier, Date() < deadline,
          let focusedWindow = attribute(application, kAXFocusedWindowAttribute) else { return false }
    return CFEqual(focusedWindow, window.element)
}

func activate(_ window: StoredWindow, deadline suppliedDeadline: Date? = nil) throws {
    let deadline = suppliedDeadline ?? Date(timeIntervalSinceNow: 1.2)
    func requireTime() throws {
        if Date() >= deadline { throw NativeFailure(message: "系统未将焦点切换到目标窗口（可能位于其他桌面）") }
    }
    try requireTime()
    // Full-screen windows often reject AXMinimized writes, even false -> false.
    if attribute(window.element, kAXMinimizedAttribute) as? Bool == true {
        try checkedSet(window.element, kAXMinimizedAttribute, kCFBooleanFalse)
    }
    try requireTime()
    try setHidden(window.app, false, deadline: deadline)
    try requireTime()
    _ = window.app.activate(options: [])
    let application = AXUIElementCreateApplication(window.app.processIdentifier)
    _ = AXUIElementSetAttributeValue(window.element, kAXMainAttribute as CFString, kCFBooleanTrue)
    try requireTime()
    _ = AXUIElementSetAttributeValue(application, kAXFocusedWindowAttribute as CFString, window.element)
    try requireTime()
    _ = AXUIElementSetAttributeValue(window.element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    // AXMain success only designates the main window; always raise the exact AX
    // reference, then require both the focused application and focused window.
    try requireTime()
    _ = AXUIElementPerformAction(window.element, kAXRaiseAction as CFString)
    while true {
        try requireTime()
        if hasExactFocus(window, deadline: deadline) { return }
        // AppKit's activation state and Space transitions need the main runloop.
        // Do not block it by polling cached NSRunningApplication.isActive.
        RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.01))
    }
}

func handle(_ request: [String: Any]) throws -> Any {
    let command = request["command"] as? String ?? ""
    let began = ProcessInfo.processInfo.systemUptime
    defer {
        if traceWindowControl { FileHandle.standardError.write(Data("TRACE handler \(command) \(ProcessInfo.processInfo.systemUptime - began)s\n".utf8)) }
    }
    if command == "access" {
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        _ = AXIsProcessTrustedWithOptions(options)
        return true
    }
    if command == "icon" {
        guard let path = request["path"] as? String else { throw NativeFailure(message: "无效应用路径") }
        let image = NSWorkspace.shared.icon(forFile: path)
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 96, pixelsHigh: 96, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0), let context = NSGraphicsContext(bitmapImageRep: bitmap) else { throw NativeFailure(message: "图标不可用") }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = context
        context.imageInterpolation = .high
        image.draw(in: NSRect(x: 0, y: 0, width: 96, height: 96), from: .zero, operation: .copy, fraction: 1)
        NSGraphicsContext.restoreGraphicsState()
        guard let png = bitmap.representation(using: .png, properties: [:]) else { throw NativeFailure(message: "图标不可用") }
        return "data:image/png;base64," + png.base64EncodedString()
    }
    let apps = NSWorkspace.shared.runningApplications.filter { $0.activationPolicy == .regular && $0.bundleURL != nil }
    if command == "list" {
        var seen = Set<String>()
        return apps.compactMap { app -> [String: String]? in
            let path = app.bundleURL!.resolvingSymlinksInPath().path
            guard seen.insert(path).inserted else { return nil }
            return ["name": app.localizedName ?? app.bundleURL!.lastPathComponent, "path": path]
        }.sorted { ($0["name"] ?? "") < ($1["name"] ?? "") }
    }
    if command == "snapshot" {
        guard let path = request["path"] as? String else { throw NativeFailure(message: "无效应用路径") }
        let target = URL(fileURLWithPath: path).resolvingSymlinksInPath().path
        let matched = apps.filter { $0.bundleURL!.resolvingSymlinksInPath().path == target }
        // A process exiting is conclusive; absence from AXWindows is not.
        stored = stored.filter { !$0.value.app.isTerminated }
        CrossSpaceWindowDiscovery.shared.removeExited(Set(apps.map { $0.processIdentifier }))
        if matched.isEmpty { return ["windows": [], "active": NSNull()] as [String: Any] }
        try requireAccess()
        var windows: [[String: Any]] = []
        var active: Any = NSNull()
        for app in matched {
            let element = AXUIElementCreateApplication(app.processIdentifier)
            let cached = stored.filter { $0.value.app.processIdentifier == app.processIdentifier }
            let (listStatus, listValue) = readAttribute(element, kAXWindowsAttribute)
            if listStatus == .cannotComplete { throw NativeFailure(message: "目标应用暂时没有响应，请在桌面切换完成后重试。") }
            guard let published = listValue as? [AXUIElement], listStatus == .success else {
                throw NativeFailure(message: "无法读取目标应用窗口（AX错误 \(listStatus.rawValue)）；请检查辅助功能授权。")
            }
            var candidates = published
            candidates += attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
            for key in [kAXMainWindowAttribute, kAXFocusedWindowAttribute] {
                if let value = attribute(element, key), CFGetTypeID(value) == AXUIElementGetTypeID() {
                    candidates.append(unsafeBitCast(value, to: AXUIElement.self))
                }
            }
            candidates += cached.sorted { $0.key < $1.key }.map { $0.value.element }
            candidates += CrossSpaceWindowDiscovery.shared.additionalWindows(pid: app.processIdentifier, known: candidates)
            let focused = attribute(element, kAXFocusedWindowAttribute)
            var seen: [AXUIElement] = []
            for window in candidates {
                if seen.contains(where: { CFEqual($0, window) }) { continue }
                seen.append(window)
                let existing = cached.first { CFEqual($0.value.element, window) }
                let (roleStatus, roleValue) = readAttribute(window, kAXRoleAttribute)
                if roleStatus == .invalidUIElement {
                    if let id = existing?.key { stored.removeValue(forKey: id) }
                    continue
                }
                // Failure to answer is not proof that a previously admitted
                // window closed. AXWindows omits valid windows on other Spaces.
                if existing == nil {
                    if roleStatus != .success {
                        throw NativeFailure(message: "无法读取窗口角色（AX错误 \(roleStatus.rawValue)）")
                    }
                    guard roleValue as? String == kAXWindowRole else { continue }
                }
                let minimized = attribute(window, kAXMinimizedAttribute) as? Bool ?? existing?.value.minimized ?? false
                if existing == nil {
                    guard minimized || attribute(window, kAXSubroleAttribute) as? String == kAXStandardWindowSubrole else { continue }
                }
                let id = existing?.key ?? nextID
                if existing == nil { nextID += 1 }
                stored[id] = StoredWindow(element: window, app: app, minimized: minimized)
                windows.append(["id": id, "minimized": minimized])
                if NSWorkspace.shared.frontmostApplication?.processIdentifier == app.processIdentifier, let focused = focused, CFEqual(focused, window) { active = id }
            }
        }
        return ["windows": windows, "active": active]
    }
    try requireAccess()
    if command == "collapse" || command == "prepare-collapse" {
        if command == "prepare-collapse", request["path"] as? String == nil { throw NativeFailure(message: "无效应用路径") }
        guard let numbers = request["ids"] as? [NSNumber] else { throw NativeFailure(message: "无效窗口列表") }
        let windows = try numbers.map { number -> StoredWindow in
            guard let window = stored[number.uint64Value] else { throw NativeFailure(message: "目标窗口已关闭") }
            return window
        }
        if command == "prepare-collapse" {
            // A window on the ordinary desktop may not have any AX root until
            // that desktop is visited. Before hiding a fullscreen app, reuse the
            // existing safe departure, then let Rust replan this SAME key press.
            // Unknown CG-only surfaces never block operations on known windows.
            var prepared = Set<pid_t>()
            let deadline = Date(timeIntervalSinceNow: 2)
            for window in windows where attribute(window.element, "AXFullScreen") as? Bool == true {
                if prepared.insert(window.app.processIdentifier).inserted {
                    try prepareToHide(window.app, windows: windows, deadline: deadline)
                }
            }
            if prepared.isEmpty { return NSNull() }
            return try handle(["command": "snapshot", "path": request["path"]!])
        }
        try collapse(windows)
        return true
    }
    guard let number = request["id"] as? NSNumber, let window = stored[number.uint64Value] else { throw NativeFailure(message: "目标窗口已关闭") }
    if command == "minimize" {
        try checkedSet(window.element, kAXMinimizedAttribute, kCFBooleanTrue)
        return true
    }
    if command == "activate" {
        try activate(window)
        return true
    }
    throw NativeFailure(message: "不支持的原生命令")
}

let helperApp = NSApplication.shared
helperApp.setActivationPolicy(.prohibited)
// Applies only to this helper process. Per-element discovery probes restore 0,
// which means this bounded global timeout, not the OS's much longer default.
AXUIElementSetMessagingTimeout(AXUIElementCreateSystemWide(), 0.25)
if traceWindowControl { FileHandle.standardError.write(Data("TRACE system \(ProcessInfo.processInfo.operatingSystemVersionString)\n".utf8)) }
// NSWorkspace refreshes application state on the main run loop. Read stdin off
// that thread, but serialize AppKit and accessibility work on the main thread.
DispatchQueue.global(qos: .userInitiated).async {
    while let line = readLine() {
        DispatchQueue.main.sync {
            autoreleasepool {
                let response: [String: Any]
                do {
                    guard let data = line.data(using: .utf8), let request = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { throw NativeFailure(message: "无效请求") }
                    response = ["data": try handle(request)]
                } catch {
                    response = ["error": (error as? NativeFailure)?.message ?? error.localizedDescription]
                }
                if let data = try? JSONSerialization.data(withJSONObject: response, options: [.sortedKeys]) {
                    FileHandle.standardOutput.write(data)
                    FileHandle.standardOutput.write(Data([10]))
                }
            }
        }
    }
    DispatchQueue.main.async { helperApp.terminate(nil) }
}
helperApp.run()
