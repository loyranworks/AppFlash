"""Compile and exercise the production geometry predicate without launching AppKit."""

import os
import pathlib
import subprocess
import tempfile

source = (pathlib.Path(__file__).resolve().parent.parent / "src-tauri/native/macos.swift").read_text()
start = source.index("func isOffscreenChromeSurface(")
end = source.index("\n}", start) + 2
# Compile the actual production function, leaving the helper's NSApplication
# entry point out of this non-GUI test executable.
function = source[start:end]
checks = r'''
let menu = CGRect(x: 0, y: 0, width: 1920, height: 30)
let other = CGRect(x: 100, y: 100, width: 360, height: 232)
precondition(!isOffscreenChromeSurface(frame: menu, offscreen: false, alpha: 1, menuBarFrames: [menu]), "On-screen menu-sized windows must remain candidates")
precondition(isOffscreenChromeSurface(frame: menu, offscreen: true, alpha: 1, menuBarFrames: [menu]), "Exact off-screen menu proxy should be classified")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: false, alpha: 0, menuBarFrames: []), "An on-screen transparent surface has no off-screen proof")
precondition(isOffscreenChromeSurface(frame: other, offscreen: true, alpha: 0, menuBarFrames: []), "Fully transparent off-screen surface should be classified")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: true, alpha: 1, menuBarFrames: [menu]), "Unrelated opaque off-screen windows must remain candidates")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: true, alpha: nil, menuBarFrames: []), "Unavailable alpha must remain unknown")
precondition(!isOffscreenChromeSurface(frame: CGRect(x: 0, y: 0, width: 1920, height: 31), offscreen: true, alpha: 1, menuBarFrames: [menu]), "Menu region match must be exact")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: true, alpha: -1, menuBarFrames: []), "Negative alpha is invalid metadata, not transparency proof")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: true, alpha: .nan, menuBarFrames: []), "NaN alpha is not transparency proof")
let primary = CGRect(x: 0, y: 0, width: 1920, height: 1080)
let toolbar = CGRect(x: 0, y: -32, width: 1920, height: 32)
precondition(isOffscreenChromeSurface(frame: toolbar, offscreen: false, alpha: 0, menuBarFrames: [], activeDisplayBounds: [primary]), "Transparent toolbar touching only the display boundary has no screen pixels")
precondition(!isOffscreenChromeSurface(frame: other, offscreen: false, alpha: 0, menuBarFrames: [], activeDisplayBounds: [primary]), "Transparent onscreen rectangle inside a display must remain a candidate")
precondition(!isOffscreenChromeSurface(frame: toolbar, offscreen: false, alpha: 0, menuBarFrames: [], activeDisplayBounds: [primary, CGRect(x: 0, y: -1080, width: 1920, height: 1080)]), "A secondary display above the primary invalidates outside-display proof")
precondition(!isOffscreenChromeSurface(frame: toolbar, offscreen: false, alpha: 0, menuBarFrames: [], activeDisplayBounds: nil), "Failed display inventory is not outside-display proof")
precondition(!isOffscreenChromeSurface(frame: toolbar, offscreen: false, alpha: 0, menuBarFrames: [], activeDisplayBounds: []), "Empty display inventory is not outside-display proof")
precondition(!isOffscreenChromeSurface(frame: toolbar, offscreen: false, alpha: 1, menuBarFrames: [], activeDisplayBounds: [primary]), "Outside-display geometry alone must not exclude an opaque candidate")
print("Fifteen production chrome classification checks passed")
'''
with tempfile.TemporaryDirectory(prefix="appflash-inventory-unit-") as temporary:
    root = pathlib.Path(temporary)
    swift = root / "main.swift"
    binary = root / "test"
    swift.write_text("import CoreGraphics\n" + function + "\n" + checks)
    subprocess.run(["swiftc", "-module-cache-path", os.environ.get("CLANG_MODULE_CACHE_PATH", str(root / "modules")), str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
