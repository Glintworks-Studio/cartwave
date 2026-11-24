// usage: swiftc -O display_info.swift -o display_info && ./display_info
// Prints each display's refresh rate from NSScreen, the active CGDisplayMode and
// CVDisplayLink, plus backing scale and the available refresh rates (PERF-macos.md §1).
import AppKit
import CoreGraphics
import CoreVideo

for s in NSScreen.screens {
    let id = s.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as! CGDirectDisplayID
    print("NSScreen '\(s.localizedName)' id=\(id) maxFPS=\(s.maximumFramesPerSecond) minInterval=\(s.minimumRefreshInterval) maxInterval=\(s.maximumRefreshInterval) granularity=\(s.displayUpdateGranularity) backingScale=\(s.backingScaleFactor) frame=\(s.frame)")
}
var n: UInt32 = 0
CGGetActiveDisplayList(0, nil, &n)
var ids = [CGDirectDisplayID](repeating: 0, count: Int(n))
CGGetActiveDisplayList(n, &ids, &n)
for id in ids {
    let m = CGDisplayCopyDisplayMode(id)!
    print("CG display \(id): builtin=\(CGDisplayIsBuiltin(id)) main=\(CGDisplayIsMain(id)) vendor=\(CGDisplayVendorNumber(id)) mode \(m.width)x\(m.height) px \(m.pixelWidth)x\(m.pixelHeight) refreshRate=\(m.refreshRate) ioModeID=\(m.ioDisplayModeID)")
    var link: CVDisplayLink?
    CVDisplayLinkCreateWithCGDisplay(id, &link)
    let nom = CVDisplayLinkGetNominalOutputVideoRefreshPeriod(link!)
    print("  CVDisplayLink nominal period = \(nom.timeValue)/\(nom.timeScale) -> \(Double(nom.timeScale)/Double(nom.timeValue)) Hz")
    CVDisplayLinkStart(link!)
    Thread.sleep(forTimeInterval: 0.5)
    print("  CVDisplayLink actual period = \(CVDisplayLinkGetActualOutputVideoRefreshPeriod(link!)) s -> \(1.0/CVDisplayLinkGetActualOutputVideoRefreshPeriod(link!)) Hz")
    CVDisplayLinkStop(link!)
    let opts = [kCGDisplayShowDuplicateLowResolutionModes: kCFBooleanTrue] as CFDictionary
    let modes = CGDisplayCopyAllDisplayModes(id, opts) as! [CGDisplayMode]
    let rates = Set(modes.map { $0.refreshRate })
    print("  available refresh rates: \(rates.sorted())")
}
