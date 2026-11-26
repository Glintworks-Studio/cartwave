// swatch: a native AppKit reference window of known sRGB colours (MAC_CATCHUP §4.7).
// AppKit colour-matches NSColor(srgbRed:…) to the display it is on, which is the
// "correctly managed" baseline the app's GL window is compared with.
//   swiftc -O swatch.swift -o swatch; swatch <x> <y> [secs=60]    (global top-left, points)
// Swatches, 64 x 64 pt each, left to right: the tokens listed in `colors`.
import AppKit

let colors: [(String, UInt32)] = [
    ("lime", 0xC7D74E), ("pulse", 0xC9D24A), ("triangle", 0x7F86E0), ("noise", 0xB5AC98),
    ("vrc6", 0xD9579A), ("well", 0x151515), ("bg", 0x0A0A0A), ("text", 0xE6E6E6),
]
let a = CommandLine.arguments
let (gx, gy) = (Double(a[1])!, Double(a[2])!)
let secs = a.count > 3 ? Double(a[3])! : 60

final class V: NSView {
    override func draw(_ r: NSRect) {
        for (i, (_, c)) in colors.enumerated() {
            NSColor(srgbRed: CGFloat((c >> 16) & 255) / 255, green: CGFloat((c >> 8) & 255) / 255, blue: CGFloat(c & 255) / 255, alpha: 1).setFill()
            NSRect(x: CGFloat(i) * 64, y: 0, width: 64, height: 64).fill()
        }
    }
}
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let h = NSScreen.screens[0].frame.height
let w = NSWindow(contentRect: NSRect(x: gx, y: h - gy - 64, width: Double(colors.count) * 64, height: 64),
                 styleMask: [.borderless], backing: .buffered, defer: false)
w.contentView = V()
w.level = .floating
w.orderFrontRegardless()
print("swatch window \(w.windowNumber) on '\(w.screen?.localizedName ?? "?")' colorSpace \(w.colorSpace?.localizedName ?? "?")")
fflush(stdout)
DispatchQueue.main.asyncAfter(deadline: .now() + secs) { app.terminate(nil) }
app.run()
