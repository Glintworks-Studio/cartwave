// Validation reference for the pacing method (PERF-macos.md §4).
// Build: swiftc -O testwin.swift -o testwin
// usage: testwin <k> <secs> : window that changes content every k-th display-link callback (k=0: static)
import AppKit
let k = Int(CommandLine.arguments[1])!; let secs = Double(CommandLine.arguments[2])!
final class V: NSView {
    var n = 0, shown = 0
    override func draw(_ r: NSRect) { NSColor(calibratedHue: CGFloat(shown % 64) / 64, saturation: 0.6, brightness: 0.8, alpha: 1).setFill(); r.fill() }
    @objc func tick(_ l: CADisplayLink) { n += 1; if k > 0 && n % k == 0 { shown += 1; needsDisplay = true } }
}
let app = NSApplication.shared; app.setActivationPolicy(.regular)
let w = NSWindow(contentRect: NSRect(x: 300, y: 300, width: 400, height: 300), styleMask: [.titled], backing: .buffered, defer: false)
let v = V(frame: w.contentLayoutRect); w.contentView = v; w.title = "testwin"; w.makeKeyAndOrderFront(nil)
let dl = v.displayLink(target: v, selector: #selector(V.tick(_:))); dl.add(to: .main, forMode: .common)
print("window", w.windowNumber); fflush(stdout)
DispatchQueue.main.asyncAfter(deadline: .now() + secs) { print("callbacks", v.n, "updates", v.shown, "rate", Double(v.n) / secs); exit(0) }
app.run()
