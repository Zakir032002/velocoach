import Foundation
import UIKit
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
    @Published private(set) var latencyPanel: [String] = []
    @Published private(set) var running = false
    @Published private(set) var badge: String?
    @Published private(set) var outcomes: [CueLine] = []
    @Published private(set) var reportCard = ""
    @Published private(set) var lastRideFiles: [URL] = []
    /// The policy hash of a fresh Engine: in Study mode it must read the same every session (M4).
    @Published private(set) var freshPolicyHash = ""
    @Published var scenarioId: UInt8 = 1
    @Published var persona: UInt8 = 0
    @Published var studyMode = false
    @Published var rider = 0 // index into StudyConfig.riders
    @Published var session = 1
    @Published var humanRider = false
    /// Lite path L1: run the Simulator on this iPhone instead of the iQOO (no radio).
    @Published var inProcess = false
    /// The on-screen slider for human mode in L1, thousandths of CP.
    @Published var localEffort: Double = 700 {
        didSet {
            let p = Int16(localEffort.rounded())
            q.async { if self.useLocal { self.local.setEffort(p) } }
        }
    }
    /// Debug field: ride second at which an HR-redline fault is sent (demo scene D3).
    @Published var redlineAtS = ""

    let audio: AudioCuePlayer
    private let ble = BLECentral()
    private var q: DispatchQueue { ble.queue }
    private var local: InProcessRig!

    /// Everything the ride needs from the screen, copied on main before hopping to ble.q.
    private struct RideSettings {
        let scenario: UInt8
        let persona: UInt8
        let study: Bool
        let rider: Int
        let session: Int
        let human: Bool
        let redlineAtS: Int?
        let inProcess: Bool
    }

    // ble.q only.
    private var engine: Engine?
    private var timer: DispatchSourceTimer?
    private var lastTick: UInt64 = 0
    private var tickCount = 0
    private var startNs: UInt64 = 0
    private var rideHuman = false
    private var lastOutcomeId: UInt32?
    private var skewFlags = 0
    private var reconnects = 0
    private var cueCount = 0
    private var soakActive = false
    private var useLocal = false // L1: CTRL goes to the in-process rig, BLE data is ignored
    private var rideGeneration = 0 // scheduled RATE/FAULT messages of an older ride must not fire

    init() {
        audio = AudioCuePlayer(queue: ble.queue)
        ble.delegate = self
        audio.delegate = self
        local = InProcessRig(queue: ble.queue) { [weak self] bytes, source, t1 in
            guard let self, self.useLocal else { return }
            self.ingest(bytes, from: source, t1: t1)
        }
        UIDevice.current.isBatteryMonitoringEnabled = true
        DispatchQueue.global(qos: .utility).async {
            let hash = (try? Engine.forScenario(cfg: Self.config(), scenarioId: 1))?.snapshot().policyHash ?? ""
            DispatchQueue.main.async { self.freshPolicyHash = hash }
        }
    }

    private static func config() -> EngineConfig {
        EngineConfig(cpW: 250, wprimeJ: 20_000, hrMax: 190, profile: .solo,
                     cooldownBetweenAlertsS: 20, promptSurgeCountdownS: 5, steps: [])
    }

    // MARK: ride control (any thread)

    func startRide() {
        let settings = RideSettings(
            scenario: scenarioId, persona: persona, study: studyMode, rider: rider, session: session,
            human: humanRider, redlineAtS: Int(redlineAtS.trimmingCharacters(in: .whitespaces)),
            inProcess: inProcess)
        q.async { self.beginRide(settings) }
    }

    func stopRide() {
        q.async {
            self.send(encodeStop(), withResponse: true)
            self.local.end()
            self.timer?.cancel()
            self.timer = nil
            self.rideGeneration += 1
            if self.soakActive { self.soakRow(event: "stop") }
            self.engine?.endSession(tNs: Clock.nowNs())
            if let e = self.engine { self.pollOutcome(e) } // a window abandoned by the stop
            let card = self.engine?.reportCard().text ?? ""
            self.drainLedger(close: true)
            self.flushCapture(close: true)
            self.engine = nil
            self.soakActive = false
            DispatchQueue.main.async {
                self.reportCard = card
                self.running = false
            }
        }
    }

    private func beginRide(_ s: RideSettings) {
        if s.study && s.rider != 0 && StudyConfig.protocolHash == "PENDING" {
            // The guard script's twin: a counted record must carry a committed protocol hash.
            DispatchQueue.main.async {
                self.error = "STUDY.md is not committed yet: set StudyConfig.protocolHash (scripts/check-protocol-hash.sh)"
            }
            return
        }
        do {
            engine = try Engine.forScenario(cfg: Self.config(), scenarioId: s.scenario)
        } catch {
            let msg = "\(error)"
            DispatchQueue.main.async { self.error = msg }
            return
        }
        rideGeneration += 1
        let gen = rideGeneration
        useLocal = s.inProcess
        if useLocal { local.begin() }
        rideHuman = s.human
        lastOutcomeId = nil
        skewFlags = 0
        reconnects = 0
        cueCount = 0
        tickCount = 0
        let seed = s.study ? StudyConfig.seed(rider: s.rider, session: s.session) : 42
        // Provenance for vc-report's banner: every sensor stream here is simulated.
        let names = ["dana", "sam", "ned", "calib"]
        let personaName = s.human ? "human-slider"
            : s.persona >= 1 && Int(s.persona) <= names.count ? names[Int(s.persona) - 1]
            : scenarios().first { $0.id == s.scenario }?.defaultPersona ?? "default"
        engine?.setDataLabel(label: "simulated persona=\(personaName) seed=\(seed) link=\(s.inProcess ? "in-process" : "ble")")
        send(encodeLoad(scenario: s.scenario, persona: s.persona, seed: UInt32(truncatingIfNeeded: seed), human: s.human),
                  withResponse: true)
        send(encodeStart(), withResponse: true)
        let now = Clock.nowNs()
        startNs = now
        lastTick = now
        engine?.resetLatency()
        engine?.setCapture(on: true)
        let soak = scenarios().first { $0.id == s.scenario }?.name == "soak"
        openFiles(soak: soak)
        if s.study {
            do {
                try engine?.setMode(mode: .study, seed: seed, protocolHash: StudyConfig.protocolHash)
                engine?.setRider(riderId: StudyConfig.riders[s.rider])
            } catch {
                let msg = "\(error)"
                DispatchQueue.main.async { self.error = msg }
                engine = nil
                return
            }
        }
        // The Day 15 loopback result: onsets become `loopback`, not `scheduled`.
        let routeMs = max(0, UserDefaults.standard.integer(forKey: "routeDelayMs"))
        audio.routeDelayNs = UInt64(routeMs) * 1_000_000
        engine?.setRouteDelayKnown(known: routeMs > 0)
        engine?.start(tNs: now)
        startTimer()
        pingBurst() // a bound within a second of Start, not only of the connection
        soakActive = soak
        if soak {
            soakRow(event: "start")
            scheduleSoakRates(gen)
        }
        if let at = s.redlineAtS { scheduleHrRedline(at: at, gen) }
        DispatchQueue.main.async {
            self.cues = []
            self.outcomes = []
            self.badge = nil
            self.reportCard = ""
            self.error = nil
            self.running = true
        }
    }

    private func startTimer() {
        timer?.cancel()
        let t = DispatchSource.makeTimerSource(flags: .strict, queue: q)
        t.schedule(deadline: .now(), repeating: .milliseconds(250), leeway: .milliseconds(5))
        t.setEventHandler { [weak self] in self?.tick(Clock.nowNs()) }
        t.resume()
        timer = t
    }

    // MARK: the link (ble.q)

    /// CTRL to the rig: over BLE, or to the in-process simulator in L1.
    private func send(_ data: Data, withResponse: Bool) {
        if useLocal {
            local.apply(data)
        } else {
            ble.write(data, withResponse: withResponse)
        }
    }

    private var linkText: String { useLocal ? "in-process (L1, no radio)" : ble.stateText }

    // MARK: clock sync (ble.q)

    private func sendPing() {
        guard let e = engine else { return }
        let a = Clock.nowNs() // a: just before the write
        let id = e.makePing(tNs: a)
        send(encodePing(pingId: id, tClientNs: a), withResponse: false)
    }

    private func pingBurst() {
        for k in 0..<8 { // a bound within a second of every (re)connection
            q.asyncAfter(deadline: .now() + .milliseconds(100 * k)) { self.sendPing() }
        }
    }

    // MARK: scripted CTRL (ble.q)

    private func scheduleSoakRates(_ gen: Int) {
        let plan: [(Int, UInt8)] = [(1_500, 10), (1_560, 40), (2_700, 20), (2_760, 40)] // (s, hz x10)
        for (at, hz) in plan {
            q.asyncAfter(deadline: .now() + .seconds(at)) { [weak self] in
                guard let self, self.engine != nil, self.rideGeneration == gen else { return }
                self.send(encodeRate(channel: .cpm, hzX10: hz), withResponse: true)
                self.soakEvent("RATE cpm \(Double(hz) / 10) Hz")
            }
        }
    }

    /// Demo scene D3: the simulator's heart rate crosses the redline at ride second `at`.
    private func scheduleHrRedline(at: Int, _ gen: Int) {
        q.asyncAfter(deadline: .now() + .seconds(at)) { [weak self] in
            guard let self, self.engine != nil, self.rideGeneration == gen else { return }
            self.send(encodeFault(kind: .hrRedline, channel: .all, durMs: 300_000, param: 0), withResponse: true)
        }
    }

    // MARK: ble.q

    private func tick(_ now: UInt64) {
        guard let e = engine else { return }
        lastTick = now
        let d = e.tick(tNs: now)
        let t2 = Clock.nowNs() // T2: the Engine has decided
        if let d { handle(d, at: now, t2: t2) }
        let s = e.snapshot()
        let link = linkText
        DispatchQueue.main.async {
            self.snapshot = s
            self.link = link
        }
        pollOutcome(e)
        tickCount += 1
        if tickCount % 4 == 0 {
            sendPing() // 1 Hz
            let panel = latencyLines(e)
            DispatchQueue.main.async { self.latencyPanel = panel }
            flushCapture()
            drainLedger()
        }
        if soakActive && tickCount % 40 == 0 { soakRow() } // every 10 s
    }

    private func latencyLines(_ e: Engine) -> [String] {
        let c = e.clockStatus()
        let ls = e.linkStats()
        func line(_ name: String, _ seg: LatencySegment, ms: Bool, note: String) -> String {
            let l = e.latencySummary(segment: seg)
            let k = ms ? 1000.0 : 1.0
            return String(format: "%@ p50 %.0f  p90 %.0f  p99 %.0f  max %.0f %@  (%@ n=%llu)",
                          name, l.p50Us / k, l.p90Us / k, l.p99Us / k, l.maxUs / k, ms ? "ms" : "µs", note, l.count)
        }
        let t0t3 = line("T0→T3", .t0t3, ms: true, note: "packet-triggered cues;")
        let bound = c.boundUs.map { String(format: " ± %.0f µs", $0) } ?? ""
        let lines = [
            String(format: "clock   θ %@ ms · rtt_min %@ µs · ±%@ µs (n %u)",
                   c.thetaMs.map { String(format: "%.1f", $0) } ?? "–",
                   c.rttMinUs.map { String(format: "%.0f", $0) } ?? "–",
                   c.boundUs.map { String(format: "%.0f", $0) } ?? "–", c.samples),
            line("T0→T1", .t0t1, ms: true, note: "every packet;"),
            line("T1→T2", .t1t2, ms: false, note: "every packet;"),
            line("T1→T3", .t1t3, ms: true, note: "every scheduled cue;"),
            t0t3 + bound,
            "stamps \(ls.stamps) · lost \(ls.stampsLost) · T0>T1 \(ls.negativeT0t1) · skew flags \(skewFlags)",
        ]
        // L1 has no radio: say so on every number.
        return useLocal ? ["IN-PROCESS (L1): no BLE, latency is not over the air"] + lines : lines
    }

    func central(didReceive bytes: Data, from source: Source, t1: UInt64) {
        guard !useLocal else { return } // L1 ride: the iQOO is not the rig
        ingest(bytes, from: source, t1: t1)
    }

    private func ingest(_ bytes: Data, from source: Source, t1: UInt64) {
        guard let e = engine else { return }
        let d = e.ingest(tNs: t1, source: source, bytes: bytes)
        let t2 = Clock.nowNs() // T2: the Engine has decided
        e.recordLatency(segment: .t1t2, ns: t2 - t1)
        if let d { handle(d, at: t1, t2: t2) }
        if t1 > lastTick + 300_000_000 { tick(t1) }
    }

    func centralDidConnect() {
        guard !useLocal else { return }
        // Before the first notification of the new link: the rig's packet numbering restarts now.
        engine?.resetLink()
        if engine != nil {
            reconnects += 1
            soakEvent("connect")
        }
    }

    func centralDidBecomeReady() {
        pingBurst()
        let link = linkText
        DispatchQueue.main.async { self.link = link }
    }

    func centralDidDisconnect() {
        guard !useLocal else { return }
        // The Engine's quality states and the dropout cue handle silence; the soak log notes it.
        soakEvent("disconnect")
    }

    private func handle(_ d: CueDecision, at t: UInt64, t2: UInt64) {
        cueCount += 1
        let secs = Double(t - startNs) / 1e9
        let line = CueLine(id: d.id, text: String(format: "%6.1f s  %@  %@  %@", secs, d.directive, "\(d.arm)", d.phraseKey))
        DispatchQueue.main.async {
            self.cues.insert(line, at: 0)
            if self.cues.count > 30 { self.cues.removeLast() }
        }
        guard d.arm != .sham else { return } // a sham plays nothing and tells the rider nothing
        guard let played = audio.play(d, now: t) else {
            engine?.onCueFailed(cueId: d.id, reason: .other) // no buffer for it: never silent about it
            return
        }
        engine?.onCueScheduled(cueId: d.id, t2Ns: t2, t3Ns: played.t3Ns)
        send(encodeCuePlayed(d: d, durMs: played.durMs), withResponse: false)
    }

    // MARK: badges

    /// The last observed cue, once: a badge for six seconds, then it stays in the list.
    private func pollOutcome(_ e: Engine) {
        guard let o = e.lastOutcome(), o.cueId != lastOutcomeId else { return }
        lastOutcomeId = o.cueId
        let text = Self.badgeText(o, human: rideHuman)
        let line = CueLine(id: o.cueId, text: "\(o.directive) · \(o.arm) · \(text)")
        DispatchQueue.main.async {
            self.badge = text
            self.outcomes.insert(line, at: 0)
            DispatchQueue.main.asyncAfter(deadline: .now() + 6) { if self.badge == text { self.badge = nil } }
        }
    }

    static func badgeText(_ o: CueOutcomeFfi, human: Bool) -> String {
        let secs: (Int32?) -> String = { $0.map { String(format: " %.1f s", Double($0) / 1000) } ?? "" }
        switch o.verdict {
        case "complied": return "✓" + secs(human ? (o.lOnsetMs ?? o.lObsMs) : o.lObsMs)
        case "ignored": return "✗ ignored"
        case "partial": return "≈ partial"
        case "reversed": return "↺ reversed"
        case "confounded": return "? " + (o.confounds.first ?? "confounded")
        default: return "– " + o.verdict
        }
    }

    // MARK: files (io.q)
    private let io = DispatchQueue(label: "io.q", qos: .utility)
    private var captureFile: FileHandle? // io.q only
    private var ledgerFile: FileHandle? // io.q only
    private var soakFile: FileHandle? // io.q only

    private func openFiles(soak: Bool) {
        let dir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("captures", isDirectory: true)
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        let capture = dir.appendingPathComponent("ride-\(stamp).vcraw")
        let ledger = dir.appendingPathComponent("ledger-\(stamp).jsonl")
        let soakUrl = dir.appendingPathComponent("soak-\(stamp).csv")
        DispatchQueue.main.async { self.lastRideFiles = [] }
        io.async {
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            FileManager.default.createFile(atPath: capture.path, contents: nil)
            self.captureFile = try? FileHandle(forWritingTo: capture)
            FileManager.default.createFile(atPath: ledger.path, contents: nil)
            self.ledgerFile = try? FileHandle(forWritingTo: ledger)
            var urls = [capture, ledger]
            if soak {
                let header = "t_s,event,link,hr_q,power_q,cadence_q,decode_errors,implausible,stamps,stamps_lost,"
                    + "reconnects,cues,t1t2_p99_us,t0t3_p99_ms,bound_us,battery_pct\n"
                FileManager.default.createFile(atPath: soakUrl.path, contents: Data(header.utf8))
                self.soakFile = try? FileHandle(forWritingTo: soakUrl)
                _ = try? self.soakFile?.seekToEnd()
                urls.append(soakUrl)
            }
            DispatchQueue.main.async { self.lastRideFiles.append(contentsOf: urls) }
        }
    }

    /// About once a second, on ble.q: the ledger records as JSONL, appended on io.q.
    private func drainLedger(close: Bool = false) {
        guard let e = engine else { return }
        let text = e.drainLedgerJsonl(max: 256)
        io.async {
            if !text.isEmpty, let data = text.data(using: .utf8) { try? self.ledgerFile?.write(contentsOf: data) }
            if close {
                try? self.ledgerFile?.close()
                self.ledgerFile = nil
            }
        }
    }

    private func flushCapture(close: Bool = false) {
        guard let e = engine else { return }
        let chunk = e.takeCapture() // on ble.q
        io.async {
            if !chunk.isEmpty { try? self.captureFile?.write(contentsOf: chunk) }
            if close {
                try? self.captureFile?.close()
                self.captureFile = nil
            }
        }
    }

    // MARK: soak log (S6)

    /// One CSV row every 10 s, plus one per event (RATE sent, connect, disconnect, start, stop).
    private func soakRow(event: String = "") {
        guard soakActive, let e = engine else { return }
        let s = e.snapshot()
        let link = e.linkStats()
        let t1t2 = e.latencySummary(segment: .t1t2)
        let t0t3 = e.latencySummary(segment: .t0t3)
        let clock = e.clockStatus()
        let t = Double(Clock.nowNs() - startNs) / 1e9
        let battery = Int(UIDevice.current.batteryLevel * 100)
        let row = String(format: "%.1f,%@,%@,%@,%@,%@,%u,%u,%u,%u,%ld,%ld,%.0f,%.1f,%.0f,%ld\n",
                         t, event, linkText, s.hrQuality, s.powerQuality, s.cadenceQuality,
                         s.decodeErrors, s.implausible, link.stamps, link.stampsLost,
                         reconnects, cueCount, t1t2.p99Us, t0t3.p99Us / 1000, clock.boundUs ?? -1,
                         battery)
        io.async {
            try? self.soakFile?.write(contentsOf: Data(row.utf8))
            if event == "stop" {
                try? self.soakFile?.close()
                self.soakFile = nil
            }
        }
    }

    private func soakEvent(_ name: String) { soakRow(event: name) }

    // MARK: AudioCuePlayerDelegate (ble.q)

    func cuePlayed(id: UInt32, onsetNs: UInt64, endNs: UInt64, callbackNs: UInt64) {
        engine?.onCuePlayed(cueId: id, tOnsetNs: onsetNs, tEndNs: endNs, tCallbackNs: callbackNs)
        let skewMs = (Int64(bitPattern: callbackNs) - Int64(bitPattern: endNs)) / 1_000_000
        if abs(skewMs) > 50 { skewFlags += 1 } // flagged, never corrected
        print("vc: cue \(id) played; callback - computed end = \(skewMs) ms")
    }

    func cueFailed(id: UInt32, reason: AudioFailure) {
        engine?.onCueFailed(cueId: id, reason: reason)
    }
}
