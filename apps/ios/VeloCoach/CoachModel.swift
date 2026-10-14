import Foundation
import VcCore

struct CueLine: Identifiable {
    let id: UInt32
    let text: String
}
final class CoachModel: ObservableObject, BLECentralDelegate, AudioCuePlayerDelegate {
    // Published on main only.
    @Published private(set) var snapshot: Snapshot?
    @Published private(set) var cues: [CueLine] = []
    @Published private(set) var link = "idle"
    @Published private(set) var error: String?
    @Published var scenarioId: UInt8 = 1
    @Published var persona: UInt8 = 0

    let audio: AudioCuePlayer
    private let ble = BLECentral()
    private var q: DispatchQueue { ble.queue }

    // ble.q only.
    private var engine: Engine?
    private var timer: DispatchSourceTimer?
    private var lastTick: UInt64 = 0
    private var startNs: UInt64 = 0

    init() {
        audio = AudioCuePlayer(queue: ble.queue)
        ble.delegate = self
        audio.delegate = self
    }

    // MARK: ride control (any thread)

    func startRide() {
        let (sid, persona) = (scenarioId, self.persona)
        q.async { self.beginRide(scenario: sid, persona: persona) }
    }

    func stopRide() {
        q.async {
            self.ble.write(encodeStop(), withResponse: true)
            self.timer?.cancel()
            self.timer = nil
            self.engine = nil
        }
    }

    private func beginRide(scenario: UInt8, persona: UInt8) {
        let cfg = EngineConfig(cpW: 250, wprimeJ: 20_000, hrMax: 190, profile: .solo,
                               cooldownBetweenAlertsS: 20, promptSurgeCountdownS: 5, steps: [])
        do {
            engine = try Engine.forScenario(cfg: cfg, scenarioId: scenario)
        } catch {
            let msg = "\(error)"
            DispatchQueue.main.async { self.error = msg }
            return
        }
        ble.write(encodeLoad(scenario: scenario, persona: persona, seed: 42, human: false), withResponse: true)
        ble.write(encodeStart(), withResponse: true)
        let now = Clock.nowNs()
        startNs = now
        lastTick = now
        engine?.start(tNs: now)
        startTimer()
        DispatchQueue.main.async { self.cues = []; self.error = nil }
    }

    private func startTimer() {
        timer?.cancel()
        let t = DispatchSource.makeTimerSource(flags: .strict, queue: q)
        t.schedule(deadline: .now(), repeating: .milliseconds(250), leeway: .milliseconds(5))
        t.setEventHandler { [weak self] in self?.tick(Clock.nowNs()) }
        t.resume()
        timer = t
    }

    // MARK: ble.q

    private func tick(_ now: UInt64) {
        guard let e = engine else { return }
        lastTick = now
        if let d = e.tick(tNs: now) {
            handle(d, at: now)
        }
        let s = e.snapshot()
        let link = ble.stateText
        DispatchQueue.main.async {
            self.snapshot = s
            self.link = link
        }
    }

    func central(didReceive bytes: Data, from source: Source, t1: UInt64) {
        guard let e = engine else { return }
        if let d = e.ingest(tNs: t1, source: source, bytes: bytes) {
            handle(d, at: t1)
        }
        // A stalled timer (screen locked) must not hold back time-based cues.
        if t1 > lastTick + 300_000_000 {
            tick(t1)
        }
    }

    func centralDidBecomeReady() {
        let link = ble.stateText
        DispatchQueue.main.async { self.link = link }
    }

    func centralDidDisconnect() {
        // Nothing to do: the Engine's quality states and the dropout cue handle silence.
    }

    private func handle(_ d: CueDecision, at t: UInt64) {
        let secs = Double(t - startNs) / 1e9
        let line = CueLine(id: d.id, text: String(format: "%6.1f s  %@  %@  %@", secs, d.directive, "\(d.arm)", d.phraseKey))
        DispatchQueue.main.async {
            self.cues.insert(line, at: 0)
            if self.cues.count > 30 { self.cues.removeLast() }
        }
        guard d.arm != .sham else { return } // a sham plays nothing and tells the rider nothing
        guard let durMs = audio.play(d, now: t) else {
            engine?.onCueFailed(cueId: d.id, reason: .other) // no buffer for it: never silent about it
            return
        }
        ble.write(encodeCuePlayed(d: d, durMs: durMs), withResponse: false)
    }

    // MARK: AudioCuePlayerDelegate (ble.q)

    func cuePlayed(id: UInt32, onsetNs: UInt64, endNs: UInt64, callbackNs: UInt64) {
        engine?.onCuePlayed(cueId: id, tOnsetNs: onsetNs, tEndNs: endNs, tCallbackNs: callbackNs)
        print("vc: cue \(id) played; callback - computed end = \(audio.callbackSkewMs.last ?? 0) ms")
    }

    func cueFailed(id: UInt32, reason: AudioFailure) {
        engine?.onCueFailed(cueId: id, reason: reason)
    }
}
