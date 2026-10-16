import AVFoundation
import VcCore

/// Called on ble.q.
protocol AudioCuePlayerDelegate: AnyObject {
    func cuePlayed(id: UInt32, onsetNs: UInt64, endNs: UInt64, callbackNs: UInt64)
    func cueFailed(id: UInt32, reason: AudioFailure)
}

final class AudioCuePlayer {
    weak var delegate: AudioCuePlayerDelegate?
    /// Earbud route delay from the Day 15 loopback; 0 until measured.
    var routeDelayNs: UInt64 = 0
    /// Callback minus computed end (ms), last 20 cues: the Day 15 cross-check.
    private(set) var callbackSkewMs: [Int] = []

    private struct Playing {
        let id: UInt32
        let t3Ns: UInt64
        let durMs: UInt16
        let generation: Int
    }

    private struct Manifest: Decodable {
        struct Entry: Decodable { let file: String; let ms: Int }
        let rate: Int
        let buffers: [String: Entry]
    }

    private let q: DispatchQueue // ble.q: every method below runs on it
    private let engine = AVAudioEngine()
    private let node = AVAudioPlayerNode()
    private var bank: [String: (buffer: AVAudioPCMBuffer, ms: UInt16)] = [:]
    private var format: AVAudioFormat?
    private var click: AVAudioPCMBuffer?
    private var loggedRoute = ""
    /// iOS's own estimate of the route's output latency (T4_est - T3), ms; ble.q.
    private(set) var outputLatencyMs = 0
    private var current: Playing?
    private var generation = 0

    init(queue: DispatchQueue) {
        q = queue
        loadBank()
        click = makeClick(format: format)
        configureSession()
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: format)
        engine.prepare()
        startEngine()
        observe()
    }

    // MARK: playing (ble.q)

    /// Schedule a cue at an explicit host time. Returns its length in ms and T3 (the
    /// scheduled render start), or nil if no buffer exists for it (the caller reports
    /// that as a failure).
    func play(_ d: CueDecision, now _: UInt64) -> (durMs: UInt16, t3Ns: UInt64)? {
        let name = d.earconKey.map { "\(d.phraseKey)__\($0)" } ?? d.phraseKey
        guard let clip = bank[name] else { return nil }
        if !engine.isRunning { startEngine() }

        if d.interrupt, let playing = current {
            generation += 1 // the interrupted clip's callback will be ignored
            node.stop()
            node.play()
            current = nil
            delegate?.cueFailed(id: playing.id, reason: .interrupted)
        }

        let leadNs = UInt64(d.leadMs) * 1_000_000
        let hostNow = mach_absolute_time() // AVAudio's clock (stops in sleep)
        let rideNow = Clock.nowNs()        // our clock (does not), sampled together
        let t3 = rideNow + leadNs
        let when = AVAudioTime(hostTime: hostNow + Clock.nsToTicks(leadNs))

        let gen = generation
        let id = d.id
        current = Playing(id: id, t3Ns: t3, durMs: clip.ms, generation: gen)
        node.scheduleBuffer(clip.buffer, at: when, options: [], completionCallbackType: .dataPlayedBack) { [weak self] _ in
            let callbackNs = Clock.nowNs()
            self?.q.async { self?.finished(id: id, generation: gen, callbackNs: callbackNs) }
        }
        return (clip.ms, t3)
    }

    /// Day 15 loopback: the click at an explicit host time (only while no cue plays).
    func scheduleClick(atHost host: UInt64) {
        q.async {
            guard self.current == nil, let click = self.click else { return }
            if !self.engine.isRunning { self.startEngine() }
            self.node.scheduleBuffer(click, at: AVAudioTime(hostTime: host), options: [], completionHandler: nil)
        }
    }

    private func makeClick(format: AVAudioFormat?) -> AVAudioPCMBuffer? {
        guard let f = format else { return nil }
        let n = AVAudioFrameCount(f.sampleRate * 0.005)
        guard let b = AVAudioPCMBuffer(pcmFormat: f, frameCapacity: n), let ch = b.floatChannelData else { return nil }
        b.frameLength = n
        let ramp = Float(f.sampleRate * 0.001)
        for i in 0..<Int(n) {
            let env = min(1, Float(i) / ramp, Float(Int(n) - 1 - i) / ramp)
            ch[0][i] = 0.8 * env * sinf(2 * .pi * 1000 * Float(i) / Float(f.sampleRate))
        }
        return b
    }

    /// `.voicePrompt` has its own volume, adjustable only while it is speaking:
    /// play the calibration clip three times so the side buttons act on it.
    func calibrate() {
        q.async {
            guard self.current == nil, let clip = self.bank["CAL"] else { return }
            if !self.engine.isRunning { self.startEngine() }
            for _ in 0..<3 { self.node.scheduleBuffer(clip.buffer, completionHandler: nil) }
        }
    }

    private func finished(id: UInt32, generation gen: Int, callbackNs: UInt64) {
        guard gen == generation, let playing = current, playing.id == id else { return } // stale
        current = nil
        let onset = playing.t3Ns + routeDelayNs
        let end = onset + UInt64(playing.durMs) * 1_000_000
        callbackSkewMs.append(Int((Int64(bitPattern: callbackNs) - Int64(bitPattern: end)) / 1_000_000))
        if callbackSkewMs.count > 20 { callbackSkewMs.removeFirst() }
        delegate?.cuePlayed(id: id, onsetNs: onset, endNs: end, callbackNs: callbackNs)
    }

    private func fail(_ reason: AudioFailure) {
        guard let playing = current else { return }
        current = nil
        generation += 1
        delegate?.cueFailed(id: playing.id, reason: reason)
    }

    // MARK: setup and recovery

    private func loadBank() {
        // assets/cues, bundled as a folder reference (scripts/render_cues.sh makes it).
        guard let dir = Bundle.main.url(forResource: "cues", withExtension: nil)
                ?? Bundle.main.url(forResource: "Cues", withExtension: nil),
              let data = try? Data(contentsOf: dir.appendingPathComponent("manifest.json")),
              let manifest = try? JSONDecoder().decode(Manifest.self, from: data)
        else {
            print("vc: cue bank missing (is Cues a folder reference?)")
            return
        }
        for (name, entry) in manifest.buffers {
            guard let file = try? AVAudioFile(forReading: dir.appendingPathComponent(entry.file)),
                  let buffer = AVAudioPCMBuffer(pcmFormat: file.processingFormat, frameCapacity: AVAudioFrameCount(file.length)),
                  (try? file.read(into: buffer)) != nil
            else { continue }
            format = file.processingFormat
            bank[name] = (buffer, UInt16(clamping: entry.ms))
        }
        print("vc: \(bank.count) cue buffers loaded")
    }

    private func configureSession() {
        let s = AVAudioSession.sharedInstance()
        do {
            try s.setCategory(.playback, mode: .voicePrompt, options: [.duckOthers])
            try s.setActive(true)
        } catch {
            print("vc: audio session: \(error)")
        }
        logRoute()
    }

    /// outputLatency once per route: LATENCY.md compares it (T4_est) with the loopback (T4_meas).
    private func logRoute() {
        let s = AVAudioSession.sharedInstance()
        let route = s.currentRoute.outputs.map { "\($0.portType.rawValue) \($0.portName)" }.joined(separator: ", ")
        outputLatencyMs = Int((s.outputLatency * 1000).rounded())
        guard route != loggedRoute else { return }
        loggedRoute = route
        print("vc: route [\(route)] outputLatency \(outputLatencyMs) ms")
    }

    private func startEngine() {
        do {
            try engine.start()
            node.play()
        } catch {
            print("vc: audio engine: \(error)")
        }
    }

    private func observe() {
        let nc = NotificationCenter.default
        nc.addObserver(forName: .AVAudioEngineConfigurationChange, object: engine, queue: nil) { [weak self] _ in
            self?.q.async {
                self?.fail(.interrupted)
                self?.startEngine() // the engine stopped itself; bring it back
            }
        }
        nc.addObserver(forName: AVAudioSession.interruptionNotification, object: nil, queue: nil) { [weak self] n in
            let raw = n.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt
            let began = raw.flatMap(AVAudioSession.InterruptionType.init(rawValue:)) == .began
            self?.q.async {
                if began {
                    self?.fail(.interrupted)
                } else {
                    self?.configureSession()
                    self?.startEngine()
                }
            }
        }
        nc.addObserver(forName: AVAudioSession.routeChangeNotification, object: nil, queue: nil) { [weak self] n in
            let raw = n.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt
            self?.q.async { self?.logRoute() }
            if raw.flatMap(AVAudioSession.RouteChangeReason.init(rawValue:)) == .oldDeviceUnavailable {
                self?.q.async { self?.fail(.routeLost) } // the earbuds went away mid-cue
            }
        }
        nc.addObserver(forName: AVAudioSession.mediaServicesWereResetNotification, object: nil, queue: nil) { [weak self] _ in
            self?.q.async {
                self?.fail(.engineReset)
                self?.configureSession()
                self?.startEngine()
            }
        }
    }
}
