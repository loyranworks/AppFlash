"""Exercise production candidate ordering under a bounded validation budget."""
import pathlib,subprocess,tempfile,os
source=(pathlib.Path(__file__).resolve().parent.parent/'src-tauri/native/macos.swift').read_text()
start=source.index('func prioritizedWindowCandidates(')
end=source.index('\n}',start)+2
checks='''
let known = (1...400).map { AXUIElementCreateApplication(pid_t($0)) }
let discovered = AXUIElementCreateApplication(777)
let duplicate = AXUIElementCreateApplication(1)
let ordered = prioritizedWindowCandidates(known: known, verified: [duplicate, discovered])
// Only two validations fit. Public roots must not starve a newly found exact root.
let admitted = ordered.prefix(2)
precondition(admitted.contains { CFEqual($0, discovered) }, "Newly discovered root starved behind public candidates")
precondition(ordered.count == 401, "A cached public root must not consume the budget twice")
precondition(CFEqual(ordered[1], known[0]), "Existing public order must remain stable after supplemental roots")
precondition(prioritizedWindowCandidates(known: known, verified: []).count == 400)
print("Cached supplemental root admitted within a two-validation budget; duplicates excluded; public order stable")
'''
with tempfile.TemporaryDirectory(prefix='appflash-priority-') as temporary:
 root=pathlib.Path(temporary);p=root/'main.swift';binary=root/'test'
 p.write_text('import ApplicationServices\n'+source[start:end]+'\n'+checks)
 subprocess.run(['swiftc','-module-cache-path',os.environ.get('CLANG_MODULE_CACHE_PATH',str(root/'modules')),str(p),'-o',str(binary)],check=True)
 subprocess.run([str(binary)],check=True)
