// dispmode: list or set a display's mode (session only; restore it afterwards!).
//   dispmode list <displayID>            every mode incl. HiDPI (points, pixels, Hz, ioModeID)
//   dispmode current <displayID>
//   dispmode set <displayID> <ioModeID>  CGConfigureDisplayWithDisplayMode, .forSession
//   dispmode origin <displayID> [x y]    print, or set (.forSession), the display's global origin
//                                        (top-left, points); a mode change can move a display
// The configuration is for this login session only; `set <id> <original ioModeID>` restores it.
import CoreGraphics
import Foundation
let a = CommandLine.arguments
guard a.count >= 3, let id = UInt32(a[2]) else { print("usage: dispmode list|current|set <displayID> [ioModeID]"); exit(2) }
let opts = [kCGDisplayShowDuplicateLowResolutionModes: kCFBooleanTrue] as CFDictionary
let modes = (CGDisplayCopyAllDisplayModes(id, opts) as? [CGDisplayMode]) ?? []
func desc(_ m: CGDisplayMode) -> String {
    "ioModeID \(m.ioDisplayModeID): \(m.width)x\(m.height) pt, \(m.pixelWidth)x\(m.pixelHeight) px (backing \(m.pixelWidth / max(m.width, 1))x) @ \(m.refreshRate) Hz usable=\(m.isUsableForDesktopGUI())"
}
switch a[1] {
case "list": for m in modes { print(desc(m)) }
case "current": if let m = CGDisplayCopyDisplayMode(id) { print(desc(m)) }
case "set":
    guard a.count > 3, let want = Int32(a[3]), let m = modes.first(where: { $0.ioDisplayModeID == want }) else { print("no such mode"); exit(1) }
    var cfg: CGDisplayConfigRef?
    CGBeginDisplayConfiguration(&cfg)
    CGConfigureDisplayWithDisplayMode(cfg, id, m, nil)
    let r = CGCompleteDisplayConfiguration(cfg, .forSession)
    print("set \(id) -> \(desc(m)): \(r == .success ? "ok" : "error \(r.rawValue)")")
    if let c = CGDisplayCopyDisplayMode(id) { print("now: \(desc(c))") }
case "origin":
    if a.count > 4, let x = Int32(a[3]), let y = Int32(a[4]) {
        var cfg: CGDisplayConfigRef?
        CGBeginDisplayConfiguration(&cfg)
        CGConfigureDisplayOrigin(cfg, id, x, y)
        let r = CGCompleteDisplayConfiguration(cfg, .forSession)
        print("origin \(id) -> (\(x), \(y)): \(r == .success ? "ok" : "error \(r.rawValue)")")
    }
    print("display \(id) bounds \(CGDisplayBounds(id))")
default: print("unknown")
}
