// macdrive: drives a running ChipDAW window with real (HID-level) input events and
// reads its macOS menu bar through the Accessibility API. MAC_CATCHUP §4.
//
//   swiftc -O macdrive.swift -o macdrive
//   macdrive <pid> <cmd> [args] [; <cmd> [args]]...
//
// Coordinates are logical points in the window's *content* (below the title bar),
// like the Windows harness's client coordinates.
//
//   focus                          activate the app, click the title bar
//   key <name> [mods]              mods: comma list of cmd,alt,shift,ctrl (e.g. key o cmd,alt)
//   type <text>                    letters, digits, space, '-', '.', '+'
//   move x y | click x y [mods] | dclick x y [mods] | rclick x y [mods]
//   drag x0 y0 x1 y1 [mods] [steps=30] [ms_per_step=8]
//   down x y [mods] | up x y [mods]
//   scroll x y lines [mods]        lines > 0 = wheel up / away; a "mouse notch" (line units)
//   pscroll x y px [mods]          pixel (trackpad-style continuous) scroll, vertical
//   pinch x y amount               trackpad magnify gesture is not synthesizable: prints "unsupported"
//   sleep s
//   shot <file.png>                window screenshot (screencapture -l, no shadow)
//   menus                          dump the menu bar: title, enabled, key equivalent, check mark
//   menu <Top> <Item> [<Sub>]      AXPress a menu item by title path
//   info                           window frame, content offset, display
//
// The terminal needs Accessibility and Screen Recording permission.
import AppKit
import ApplicationServices

setvbuf(stdout, nil, _IOLBF, 0)
var argv = Array(CommandLine.arguments.dropFirst())
guard argv.count >= 2, let pid = pid_t(argv[0]) else {
    print("usage: macdrive <pid> <cmd> [args] [; <cmd> ...]"); exit(2)
}
argv.removeFirst()
let contentH = Double(ProcessInfo.processInfo.environment["MACDRIVE_CONTENT_H"] ?? "") ?? 0

func pause(_ s: Double) { Thread.sleep(forTimeInterval: s) }

func windowInfo() -> (id: CGWindowID, rect: CGRect)? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    var best: (CGWindowID, CGRect)?
    for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
        let b = CGRect(dictionaryRepresentation: w[kCGWindowBounds as String] as! CFDictionary)!
        if best == nil || b.width * b.height > best!.1.width * best!.1.height {
            best = (w[kCGWindowNumber as String] as! CGWindowID, b)
        }
    }
    return best
}

let axApp = AXUIElementCreateApplication(pid)
func axAttr(_ e: AXUIElement, _ a: String) -> CFTypeRef? {
    var v: CFTypeRef?
    return AXUIElementCopyAttributeValue(e, a as CFString, &v) == .success ? v : nil
}
func axWindow() -> AXUIElement? { (axAttr(axApp, kAXWindowsAttribute) as? [AXUIElement])?.first }

// Title-bar height: from MACDRIVE_CONTENT_H if set, else twice the close button's centre
// below the window's top (the traffic lights sit centred in the title bar): 32 pt on
// macOS 26 at 1x and 2x. Fallback 32.
func titleBar(_ rect: CGRect) -> Double {
    if contentH > 0 { return rect.height - contentH }
    if let w = axWindow() {
        for c in (axAttr(w, kAXChildrenAttribute) as? [AXUIElement]) ?? [] where (axAttr(c, kAXSubroleAttribute) as? String) == "AXCloseButton" {
            var p = CGPoint.zero, sz = CGSize.zero
            if let v = axAttr(c, kAXPositionAttribute) { AXValueGetValue(v as! AXValue, .cgPoint, &p) }
            if let v = axAttr(c, kAXSizeAttribute) { AXValueGetValue(v as! AXValue, .cgSize, &sz) }
            if sz.height > 0 { return 2 * (p.y + sz.height / 2 - rect.minY) }
        }
    }
    return 32
}

var win = windowInfo()
func pt(_ x: Double, _ y: Double) -> CGPoint {
    let r = win!.rect
    return CGPoint(x: r.minX + x, y: r.minY + titleBar(r) + y)
}

// MARK: input

let src = CGEventSource(stateID: .hidSystemState)
func post(_ e: CGEvent?) { e?.post(tap: .cghidEventTap) }
func flags(_ m: String) -> CGEventFlags {
    var f: CGEventFlags = []
    for p in m.split(separator: ",") {
        switch p {
        case "cmd": f.insert(.maskCommand)
        case "alt", "opt": f.insert(.maskAlternate)
        case "shift": f.insert(.maskShift)
        case "ctrl": f.insert(.maskControl)
        default: break
        }
    }
    return f
}
let modKeys: [(CGEventFlags, CGKeyCode)] = [(.maskCommand, 55), (.maskShift, 56), (.maskAlternate, 58), (.maskControl, 59)]
// Press the modifier keys themselves (flagsChanged events), so the app's own
// modifier tracking (winit) sees them, then send the key with the flags set.
func modsDown(_ f: CGEventFlags) {
    var acc: CGEventFlags = []
    for (m, code) in modKeys where f.contains(m) {
        acc.insert(m)
        let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); e?.flags = acc; post(e); pause(0.02)
    }
}
func modsUp(_ f: CGEventFlags) {
    var acc = f
    for (m, code) in modKeys.reversed() where f.contains(m) {
        acc.remove(m)
        let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); e?.flags = acc; post(e); pause(0.02)
    }
}
let keyCodes: [String: CGKeyCode] = [
    "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12,
    "w": 13, "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23,
    "=": 24, "+": 24, "9": 25, "7": 26, "-": 27, "8": 28, "0": 29, "]": 30, "o": 31, "u": 32, "[": 33, "i": 34,
    "p": 35, "return": 36, "enter": 36, "l": 37, "j": 38, "'": 39, "k": 40, ";": 41, "\\": 42, ",": 43, "/": 44,
    "n": 45, "m": 46, ".": 47, "tab": 48, "space": 49, " ": 49, "`": 50, "delete": 51, "backspace": 51,
    "escape": 53, "esc": 53, "f1": 122, "f2": 120, "f3": 99, "f4": 118, "f5": 96, "f6": 97, "f7": 98,
    "home": 115, "end": 119, "pageup": 116, "pagedown": 121, "fwddelete": 117, "left": 123, "right": 124,
    "down": 125, "up": 126,
]
func key(_ name: String, _ m: String = "") {
    guard let code = keyCodes[name.lowercased()] else { print("unknown key \(name)"); return }
    let f = flags(m)
    var kf = f
    if name == "+" { kf.insert(.maskShift) }
    modsDown(kf)
    let d = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true); d?.flags = kf; post(d); pause(0.03)
    let u = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false); u?.flags = kf; post(u); pause(0.03)
    modsUp(kf)
    pause(0.08)
}
func typeText(_ s: String) {
    for ch in s {
        let c = String(ch)
        if c.uppercased() == c && c.lowercased() != c { key(c.lowercased(), "shift") } else { key(c) }
    }
}
func mouse(_ type: CGEventType, _ p: CGPoint, _ f: CGEventFlags = [], button: CGMouseButton = .left, clicks: Int64 = 1) {
    let e = CGEvent(mouseEventSource: src, mouseType: type, mouseCursorPosition: p, mouseButton: button)
    e?.flags = f
    e?.setIntegerValueField(.mouseEventClickState, value: clicks)
    post(e)
}
func click(_ p: CGPoint, _ f: CGEventFlags, count: Int = 1, right: Bool = false) {
    mouse(.mouseMoved, p, f); pause(0.05)
    for i in 1...count {
        mouse(right ? .rightMouseDown : .leftMouseDown, p, f, button: right ? .right : .left, clicks: Int64(i)); pause(0.03)
        mouse(right ? .rightMouseUp : .leftMouseUp, p, f, button: right ? .right : .left, clicks: Int64(i)); pause(0.06)
    }
}
func scroll(_ p: CGPoint, _ amount: Int32, _ f: CGEventFlags, pixels: Bool) {
    mouse(.mouseMoved, p, f); pause(0.05)
    let e = CGEvent(scrollWheelEvent2Source: src, units: pixels ? .pixel : .line, wheelCount: 1, wheel1: amount, wheel2: 0, wheel3: 0)
    e?.location = p
    e?.flags = f
    if pixels { e?.setIntegerValueField(.scrollWheelEventIsContinuous, value: 1) }
    post(e); pause(0.05)
}

// MARK: menus (AX)

func children(_ e: AXUIElement) -> [AXUIElement] { (axAttr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] }
func title(_ e: AXUIElement) -> String { (axAttr(e, kAXTitleAttribute) as? String) ?? "" }
func modString(_ m: Int, _ ch: String) -> String {
    // kAXMenuItemCmdModifiers: bit0 shift, bit1 option, bit2 control, bit3 = NO command.
    var s = ""
    if m & 4 != 0 { s += "⌃" }
    if m & 2 != 0 { s += "⌥" }
    if m & 1 != 0 { s += "⇧" }
    if m & 8 == 0 { s += "⌘" }
    return s + ch
}
func dumpMenu(_ e: AXUIElement, _ depth: Int) {
    for item in children(e) {
        let role = (axAttr(item, kAXRoleAttribute) as? String) ?? ""
        if role == "AXMenu" { dumpMenu(item, depth); continue }
        let t = title(item)
        let enabled = (axAttr(item, kAXEnabledAttribute) as? Bool) ?? true
        let ch = (axAttr(item, kAXMenuItemCmdCharAttribute) as? String) ?? ""
        var glyph = ""
        if let g = axAttr(item, kAXMenuItemCmdGlyphAttribute) as? Int, g != 0 { glyph = "glyph\(g)" }
        let mods = (axAttr(item, kAXMenuItemCmdModifiersAttribute) as? Int) ?? 0
        let mark = (axAttr(item, kAXMenuItemMarkCharAttribute) as? String) ?? ""
        let keyText = ch.isEmpty && glyph.isEmpty ? "" : modString(mods, ch.isEmpty ? glyph : ch)
        if t.isEmpty && role == "AXMenuItem" && keyText.isEmpty { print(String(repeating: "  ", count: depth) + "----"); continue }
        print(String(repeating: "  ", count: depth) + "\(mark.isEmpty ? "" : "✓ ")\(t)\(enabled ? "" : " [disabled]")\(keyText.isEmpty ? "" : "\t\(keyText)")")
        let sub = children(item)
        if !sub.isEmpty { dumpMenu(item, depth + 1) }
    }
}
func findMenu(_ path: [String]) -> AXUIElement? {
    guard let bar = axAttr(axApp, kAXMenuBarAttribute) else { return nil }
    var cur = bar as! AXUIElement
    for name in path {
        var next: AXUIElement?
        var stack = children(cur)
        while let c = stack.first {
            stack.removeFirst()
            let role = (axAttr(c, kAXRoleAttribute) as? String) ?? ""
            if role == "AXMenu" { stack = children(c) + stack; continue }
            if title(c) == name { next = c; break }
        }
        guard let n = next else { return nil }
        cur = n
    }
    return cur
}

// MARK: run

var cmds: [[String]] = [[]]
for a in argv { if a == ";" { cmds.append([]) } else { cmds[cmds.count - 1].append(a) } }
for c in cmds where !c.isEmpty {
    win = windowInfo()
    let a = Array(c.dropFirst())
    func d(_ i: Int) -> Double { Double(a[i])! }
    func m(_ i: Int) -> CGEventFlags { a.count > i ? flags(a[i]) : [] }
    switch c[0] {
    case "focus":
        NSRunningApplication(processIdentifier: pid)?.activate()
        pause(0.2)
        if let r = win?.rect { click(CGPoint(x: r.midX + 200, y: r.minY + 12), []) }
        pause(0.2)
    case "key": key(a[0], a.count > 1 ? a[1] : "")
    case "type": typeText(a.joined(separator: " "))
    case "move": mouse(.mouseMoved, pt(d(0), d(1))); pause(0.05)
    case "click": click(pt(d(0), d(1)), m(2))
    case "dclick": click(pt(d(0), d(1)), m(2), count: 2)
    case "rclick": click(pt(d(0), d(1)), m(2), right: true)
    case "down": mouse(.mouseMoved, pt(d(0), d(1)), m(2)); pause(0.03); mouse(.leftMouseDown, pt(d(0), d(1)), m(2)); pause(0.05)
    case "up": mouse(.leftMouseUp, pt(d(0), d(1)), m(2)); pause(0.05)
    case "drag":
        let (p0, p1) = (pt(d(0), d(1)), pt(d(2), d(3)))
        let f = m(4)
        let steps = a.count > 5 ? Int(a[5])! : 30
        let ms = a.count > 6 ? Double(a[6])! : 8
        if !f.isEmpty { modsDown(f) }
        mouse(.mouseMoved, p0, f); pause(0.05); mouse(.leftMouseDown, p0, f); pause(0.06)
        for i in 1...steps {
            let t = Double(i) / Double(steps)
            mouse(.leftMouseDragged, CGPoint(x: p0.x + (p1.x - p0.x) * t, y: p0.y + (p1.y - p0.y) * t), f)
            pause(ms / 1000)
        }
        pause(0.05); mouse(.leftMouseUp, p1, f); pause(0.08)
        if !f.isEmpty { modsUp(f) }
    case "scroll":
        let f = m(3)
        if !f.isEmpty { modsDown(f) }
        scroll(pt(d(0), d(1)), Int32(a[2])!, f, pixels: false)
        if !f.isEmpty { modsUp(f) }
    case "pscroll":
        let f = m(3)
        if !f.isEmpty { modsDown(f) }
        scroll(pt(d(0), d(1)), Int32(a[2])!, f, pixels: true)
        if !f.isEmpty { modsUp(f) }
    case "pinch": print("pinch: unsupported (CGEvent can't synthesize trackpad magnify gestures)")
    case "sleep": pause(d(0))
    case "shot":
        guard let w = win else { print("no window"); break }
        let p = Process(); p.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        p.arguments = ["-x", "-o", "-l\(w.id)", a[0]]; try? p.run(); p.waitUntilExit()
        print("shot \(a[0]) window \(w.id) \(w.rect)")
    case "menus":
        if let bar = axAttr(axApp, kAXMenuBarAttribute) {
            for top in children(bar as! AXUIElement) {
                print("[\(title(top))]")
                dumpMenu(top, 1)
            }
        } else { print("no menu bar (Accessibility permission?)") }
    case "menu":
        if let item = findMenu(a) {
            let r = AXUIElementPerformAction(item, kAXPressAction as CFString)
            print("menu \(a.joined(separator: " > ")): \(r == .success ? "pressed" : "error \(r.rawValue)")")
        } else { print("menu \(a.joined(separator: " > ")): not found") }
    case "title":
        print((axWindow().flatMap { axAttr($0, kAXTitleAttribute) as? String }) ?? "(no window)")
    case "windows":
        // Every AX window and sheet of the app (native file panels and alerts show up here).
        for w in (axAttr(axApp, kAXWindowsAttribute) as? [AXUIElement]) ?? [] {
            let role = (axAttr(w, kAXRoleAttribute) as? String) ?? "?"
            let sub = (axAttr(w, kAXSubroleAttribute) as? String) ?? ""
            print("window role=\(role) subrole=\(sub) title='\(title(w))'")
            for c in children(w) where ((axAttr(c, kAXRoleAttribute) as? String) ?? "") == "AXSheet" {
                print("  sheet title='\(title(c))'")
                for b in children(c) where ((axAttr(b, kAXRoleAttribute) as? String) ?? "") == "AXButton" { print("    button '\(title(b))'") }
            }
            for b in children(w) where ((axAttr(b, kAXRoleAttribute) as? String) ?? "") == "AXButton" && !title(b).isEmpty { print("  button '\(title(b))'") }
            for t in children(w) where ((axAttr(t, kAXRoleAttribute) as? String) ?? "") == "AXStaticText" {
                print("  text '\((axAttr(t, kAXValueAttribute) as? String) ?? "")'")
            }
        }
    case "press":
        // Press a button by title in any window or sheet (native dialogs).
        var done = false
        func walk(_ e: AXUIElement, _ depth: Int) {
            if done || depth > 6 { return }
            for c in children(e) {
                if ((axAttr(c, kAXRoleAttribute) as? String) ?? "") == "AXButton" && title(c) == a[0] {
                    AXUIElementPerformAction(c, kAXPressAction as CFString); done = true; return
                }
                walk(c, depth + 1)
            }
        }
        for w in (axAttr(axApp, kAXWindowsAttribute) as? [AXUIElement]) ?? [] { walk(w, 0) }
        print("press '\(a[0])': \(done ? "pressed" : "not found")")
    case "setfield":
        // Type a path into a native save panel's name field: focuses it via AX and sets the value.
        var set = false
        func walk(_ e: AXUIElement, _ depth: Int) {
            if set || depth > 8 { return }
            for c in children(e) {
                if ((axAttr(c, kAXRoleAttribute) as? String) ?? "") == "AXTextField" {
                    AXUIElementSetAttributeValue(c, kAXValueAttribute as CFString, a[0] as CFString); set = true; return
                }
                walk(c, depth + 1)
            }
        }
        for w in (axAttr(axApp, kAXWindowsAttribute) as? [AXUIElement]) ?? [] { walk(w, 0) }
        print("setfield: \(set ? "set" : "no text field")")
    case "info":
        guard let w = win else { print("no window"); break }
        var disp: [CGDirectDisplayID] = Array(repeating: 0, count: 4); var n: UInt32 = 0
        CGGetDisplaysWithPoint(CGPoint(x: w.rect.midX, y: w.rect.midY), 4, &disp, &n)
        let scr = NSScreen.screens.first { ($0.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value == disp[0] }
        print("window \(w.id) frame \(w.rect) titleBar \(titleBar(w.rect)) display \(disp[0]) '\(scr?.localizedName ?? "?")' backing \(scr?.backingScaleFactor ?? 0) maxFPS \(scr?.maximumFramesPerSecond ?? 0)")
    default: print("unknown command \(c[0])")
    }
}
