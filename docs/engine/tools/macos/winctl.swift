// Window churn for the engine's test D (and display moves for test J), driven
// through the Accessibility API. The terminal running it needs Accessibility
// permission (as for chipbench).
//
//   swiftc -O winctl.swift -o winctl
//   winctl <pid> churn <secs>        repeat: 4 sizes, a 30-step live resize
//                                    sweep, minimise 2 s / restore, hide 2 s /
//                                    unhide, fully occlude 3 s / uncover
//   winctl <pid> move <x> <y>        move the window (global points), e.g. onto
//                                    another display
//   winctl <pid> info                window frame and the display it is on
//
// Every action is printed with a host-clock time stamp (ns, mach_absolute_time)
// so it can be lined up with the app's per-second log.

import AppKit
import ApplicationServices

setvbuf(stdout, nil, _IOLBF, 0)
let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]) else {
    print("usage: winctl <pid> churn <secs> | move <x> <y> | info"); exit(2)
}
var tb = mach_timebase_info_data_t(); mach_timebase_info(&tb)
func hostNs() -> UInt64 { mach_absolute_time() * UInt64(tb.numer) / UInt64(tb.denom) }
func log(_ s: String) { print("\(hostNs()) \(s)") }

let app = AXUIElementCreateApplication(pid)
func window() -> AXUIElement? {
    for _ in 0..<100 {
        var v: CFTypeRef?
        if AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &v) == .success,
           let ws = v as? [AXUIElement], let w = ws.first { return w }
        usleep(100_000)
    }
    return nil
}
guard let win = window() else { print("no window for pid \(pid) (Accessibility permission?)"); exit(1) }

func set(_ attr: String, _ value: AXValue) { AXUIElementSetAttributeValue(win, attr as CFString, value) }
func resize(_ w: Double, _ h: Double) {
    var s = CGSize(width: w, height: h); set(kAXSizeAttribute, AXValueCreate(.cgSize, &s)!)
}
func move(_ x: Double, _ y: Double) {
    var p = CGPoint(x: x, y: y); set(kAXPositionAttribute, AXValueCreate(.cgPoint, &p)!)
}
func minimise(_ on: Bool) { AXUIElementSetAttributeValue(win, kAXMinimizedAttribute as CFString, on as CFBoolean) }
func frame() -> CGRect {
    var p = CGPoint.zero, s = CGSize.zero, v: CFTypeRef?
    if AXUIElementCopyAttributeValue(win, kAXPositionAttribute as CFString, &v) == .success { AXValueGetValue(v as! AXValue, .cgPoint, &p) }
    if AXUIElementCopyAttributeValue(win, kAXSizeAttribute as CFString, &v) == .success { AXValueGetValue(v as! AXValue, .cgSize, &s) }
    return CGRect(origin: p, size: s)
}
func pause(_ s: Double) { RunLoop.current.run(until: Date().addingTimeInterval(s)) }

/// The display under the window's centre (AX coordinates: top-left origin).
func displayInfo() -> String {
    let f = frame()
    var ids = [CGDirectDisplayID](repeating: 0, count: 8); var n: UInt32 = 0
    CGGetDisplaysWithPoint(CGPoint(x: f.midX, y: f.midY), 8, &ids, &n)
    guard n > 0 else { return "frame \(f), no display" }
    let mode = CGDisplayCopyDisplayMode(ids[0])
    return "frame \(f), display \(ids[0]) \(CGDisplayPixelsWide(ids[0]))x\(CGDisplayPixelsHigh(ids[0])) @ \(mode?.refreshRate ?? 0) Hz"
}

_ = NSApplication.shared
NSApp.setActivationPolicy(.accessory)
let running = NSRunningApplication(processIdentifier: pid)

/// An opaque window above everything on the window's display, so the app is
/// fully occluded (what App Nap / occlusion notifications react to).
func occlude(_ secs: Double) {
    let screen = NSScreen.screens.first { $0.frame.contains(NSPoint(x: frame().midX, y: NSScreen.screens[0].frame.maxY - frame().midY)) } ?? NSScreen.screens[0]
    let w = NSWindow(contentRect: screen.frame, styleMask: .borderless, backing: .buffered, defer: false, screen: screen)
    w.level = .screenSaver
    w.backgroundColor = .black
    w.isOpaque = true
    w.ignoresMouseEvents = true
    w.orderFrontRegardless()
    log("occlude on")
    pause(secs)
    w.orderOut(nil)
    log("occlude off")
}

switch args[2] {
case "info":
    print(displayInfo())
case "move" where args.count >= 5:
    move(Double(args[3])!, Double(args[4])!)
    pause(0.5)
    log("moved: \(displayInfo())")
case "churn":
    let secs = Double(args.count > 3 ? args[3] : "110")!
    let t0 = Date()
    let origin = frame().origin
    let sizes: [(Double, Double)] = [(1440, 900), (1100, 700), (1700, 1000), (1300, 820)]
    var i = 0
    log("churn start: \(displayInfo())")
    while Date().timeIntervalSince(t0) < secs {
        let s = sizes[i % sizes.count]
        resize(s.0, s.1); log("resize \(Int(s.0))x\(Int(s.1))"); pause(1.5)
        log("sweep")
        for k in 0..<30 { resize(1000 + 20 * Double(k), 650 + 10 * Double(k)); pause(0.016) }
        minimise(true); log("minimise"); pause(2.0)
        minimise(false); log("restore"); pause(1.5)
        running?.hide(); log("hide"); pause(2.0)
        running?.unhide(); log("unhide"); pause(1.5)
        occlude(3.0); pause(1.5)
        i += 1
    }
    resize(1440, 900); move(origin.x, origin.y)
    log("churn end after \(i) cycles")
default:
    print("unknown command"); exit(2)
}
