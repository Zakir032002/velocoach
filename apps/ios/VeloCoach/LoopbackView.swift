import QuartzCore
import SwiftUI

final class Loopback: NSObject, ObservableObject {
    @Published var white = false
    @Published var fired = 0
    private let audio: AudioCuePlayer
    private var link: CADisplayLink?
    private var targetS: CFTimeInterval = 0

    init(audio: AudioCuePlayer) { self.audio = audio }

    func run(count: Int = 20) {
        fired = 0
        for k in 0..<count {
            DispatchQueue.main.asyncAfter(deadline: .now() + .seconds(3 * k + 2)) { self.fire() }
        }
    }

    private func fire() {
        let host = mach_absolute_time() + Clock.nsToTicks(500_000_000)
        targetS = CFTimeInterval(Clock.ticksToNs(host)) / 1e9 // CACurrentMediaTime's clock
        audio.scheduleClick(atHost: host)
        link?.invalidate()
        link = CADisplayLink(target: self, selector: #selector(frame(_:)))
        link?.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
        link?.add(to: .main, forMode: .common)
    }

    @objc private func frame(_ l: CADisplayLink) {
        let half = (l.targetTimestamp - l.timestamp) / 2
        if white {
            white = false // one frame of white, then black
            l.invalidate()
            link = nil
            fired += 1
        } else if l.targetTimestamp >= targetS - half {
            white = true
        }
    }
}

struct LoopbackView: View {
    @StateObject var loop: Loopback

    var body: some View {
        ZStack {
            (loop.white ? Color.white : Color.black).ignoresSafeArea()
            VStack {
                Spacer()
                Text("\(loop.fired) / 20").foregroundStyle(.gray)
                Button("Start (20 clicks)") { loop.run() }.padding()
            }
        }
    }
}
