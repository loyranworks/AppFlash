"""Compile production fallback policy and reject incomplete/contradictory evidence."""
import os
import pathlib
import subprocess
import tempfile

source = (pathlib.Path(__file__).resolve().parent.parent /
          "src-tauri/native/macos.swift").read_text()
start = source.index("func acceptsLocalWindowConfirmation(")
end = source.index("\n}", start) + 2
checks = '''
func accepts(_ status: AXError = .noValue, front: Bool = true,
             active: Bool? = true, exact: Bool = true, hidden: Bool? = false,
             minimized: Bool? = false, onscreen: Bool = true) -> Bool {
    acceptsLocalWindowConfirmation(systemStatus: status, frontMatches: front,
        axFrontmost: active, localWindowMatches: exact, hidden: hidden,
        minimized: minimized, onscreen: onscreen)
}
precondition(accepts(), "Explicitly missing system focus must allow verified exact local window")
precondition(accepts(.attributeUnsupported))
precondition(accepts(.notImplemented))
for status: AXError in [.success, .cannotComplete, .apiDisabled, .invalidUIElement,
                       .failure, .illegalArgument, .notEnoughPrecision] {
    precondition(!accepts(status), "A valid/conflicting or hard-failing system query cannot fall back")
}
precondition(!accepts(front: false), "Other foreground PID")
precondition(!accepts(active: false), "Inactive app")
precondition(!accepts(active: nil), "Unknown app activation")
precondition(!accepts(exact: false), "Wrong or unavailable local window")
precondition(!accepts(hidden: true), "Hidden app")
precondition(!accepts(hidden: nil), "Unknown hidden state")
precondition(!accepts(minimized: true), "Minimized target")
precondition(!accepts(minimized: nil), "Unknown minimized state")
precondition(!accepts(onscreen: false), "Off-Space or unverified WindowServer target")
print("Exact local window confirmation accepts only explicit missing system focus and complete evidence")
'''
with tempfile.TemporaryDirectory(prefix="appflash-focus-policy-") as temporary:
    root = pathlib.Path(temporary)
    swift = root / "main.swift"
    binary = root / "test"
    swift.write_text("import ApplicationServices\n" + source[start:end] + checks)
    subprocess.run(["swiftc", "-module-cache-path",
                    os.environ.get("CLANG_MODULE_CACHE_PATH", str(root / "modules")),
                    str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True)
