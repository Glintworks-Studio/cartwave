// thermlog: logs the thermal state once per interval, for MAC_CATCHUP §0.
//   swiftc -O thermlog.swift -o thermlog; thermlog <out.txt> [interval_s=30]
// Each line: local time, ProcessInfo.thermalState, lowPowerMode, and the
// `pmset -g therm` warning levels (no sudo needed).
import Foundation
setvbuf(stdout, nil, _IOLBF, 0)
let a = CommandLine.arguments
guard a.count >= 2 else { print("usage: thermlog <out> [interval]"); exit(2) }
let every = a.count > 2 ? Double(a[2])! : 30
let names = ["nominal", "fair", "serious", "critical"]
let fmt = DateFormatter(); fmt.dateFormat = "yyyy-MM-dd HH:mm:ss"
FileManager.default.createFile(atPath: a[1], contents: nil)
let fh = FileHandle(forWritingAtPath: a[1])!
fh.seekToEndOfFile()
while true {
    let p = Process(); p.executableURL = URL(fileURLWithPath: "/usr/bin/pmset"); p.arguments = ["-g", "therm"]
    let pipe = Pipe(); p.standardOutput = pipe; try? p.run(); p.waitUntilExit()
    let out = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
    let pm = out.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }.joined(separator: " | ")
    let st = ProcessInfo.processInfo.thermalState.rawValue
    let line = "\(fmt.string(from: Date())) thermalState=\(names[st]) lowPower=\(ProcessInfo.processInfo.isLowPowerModeEnabled) pmset=[\(pm)]\n"
    fh.write(line.data(using: .utf8)!)
    Thread.sleep(forTimeInterval: every)
}
