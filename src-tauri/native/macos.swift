import AppKit
import ApplicationServices

struct NativeFailure: Error { let message: String }
struct StoredWindow { let element: AXUIElement; let app: NSRunningApplication }
var stored: [UInt64: StoredWindow] = [:]
var nextID: UInt64 = 1

func attribute(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

func requireAccess() throws {
    if !AXIsProcessTrusted() { throw NativeFailure(message: "请在系统设置 → 隐私与安全性 → 辅助功能中授权 AppFlash 原生组件，然后重试。") }
}

func checkedSet(_ element: AXUIElement, _ key: String, _ value: CFTypeRef) throws {
    if AXUIElementSetAttributeValue(element, key as CFString, value) != .success { throw NativeFailure(message: "目标应用不允许修改窗口状态。") }
}

func handle(_ request: [String: Any]) throws -> Any {
    let command = request["command"] as? String ?? ""
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
        if matched.isEmpty { return ["windows": [], "active": NSNull()] as [String: Any] }
        try requireAccess()
        var windows: [[String: Any]] = []
        var active: Any = NSNull()
        var live = Set<UInt64>()
        for app in matched {
            let element = AXUIElementCreateApplication(app.processIdentifier)
            guard let list = attribute(element, kAXWindowsAttribute) as? [AXUIElement] else { throw NativeFailure(message: "无法读取目标应用窗口；请检查辅助功能授权。") }
            let focused = attribute(element, kAXFocusedWindowAttribute)
            for window in list {
                let title = attribute(window, kAXTitleAttribute) as? String ?? ""
                if title.isEmpty { continue }
                let existing = stored.first { $0.value.app.processIdentifier == app.processIdentifier && CFEqual($0.value.element, window) }
                let id = existing?.key ?? nextID
                if existing == nil { nextID += 1 }
                stored[id] = StoredWindow(element: window, app: app)
                live.insert(id)
                windows.append(["id": id, "minimized": attribute(window, kAXMinimizedAttribute) as? Bool ?? false])
                if NSWorkspace.shared.frontmostApplication?.processIdentifier == app.processIdentifier, let focused = focused, CFEqual(focused, window) { active = id }
            }
        }
        let pids = Set(matched.map { $0.processIdentifier })
        stored = stored.filter { !pids.contains($0.value.app.processIdentifier) || live.contains($0.key) }
        return ["windows": windows, "active": active]
    }
    try requireAccess()
    guard let number = request["id"] as? NSNumber, let window = stored[number.uint64Value] else { throw NativeFailure(message: "目标窗口已关闭") }
    if command == "minimize" {
        try checkedSet(window.element, kAXMinimizedAttribute, kCFBooleanTrue)
        return true
    }
    if command == "activate" {
        try checkedSet(window.element, kAXMinimizedAttribute, kCFBooleanFalse)
        _ = window.app.activate(options: [.activateIgnoringOtherApps])
        if AXUIElementPerformAction(window.element, kAXRaiseAction as CFString) != .success { throw NativeFailure(message: "系统拒绝置前窗口") }
        return true
    }
    throw NativeFailure(message: "不支持的原生命令")
}

while let line = readLine() {
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
