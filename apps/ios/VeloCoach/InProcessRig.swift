import Foundation
import VcCore

/// Lite path L1 (plan §5, rules K1/K3): the Simulator runs in-process on the iPhone instead of
/// on the iQOO, and its packets enter exactly the same Engine path as BLE notifications. There
/// is no radio, so every latency number from an L1 ride is labelled "in-process".
/// Everything here runs on ble.q.
final class InProcessRig {
    private let q: DispatchQueue
    /// (bytes, source, T1): the same call the BLE central makes.
    private let deliver: (Data, Source, UInt64) -> Void
    private var sim: Simulator?
    private var timer: DispatchSourceTimer?
    private var sent = [0, 0, 0] // per-channel packet numbers, as the rig's NotifyQueue keeps them
    private var stampSeq = 0
    private var riderSeq = 0
    private var effort: Int16 = 700 // the on-screen slider, thousandths of CP
    private var human = false

    private static let opLoad: UInt8 = 0x20
    private static let opStart: UInt8 = 0x21

    init(queue: DispatchQueue, deliver: @escaping (Data, Source, UInt64) -> Void) {
        q = queue
        self.deliver = deliver
    }

    /// Start stepping the simulator every 50 ms (the rig's SimDriver period).
    func begin() {
        timer?.cancel()
        let t = DispatchSource.makeTimerSource(flags: .strict, queue: q)
        t.schedule(deadline: .now(), repeating: .milliseconds(50), leeway: .milliseconds(2))
        t.setEventHandler { [weak self] in self?.step() }
        t.resume()
        timer = t
    }

    func end() {
        timer?.cancel()
        timer = nil
    }

    /// Every CTRL message the coach would have written to the rig.
    func apply(_ bytes: Data) {
        let now = Clock.nowNs()
        let op: UInt8? = bytes.count > 1 ? bytes[bytes.startIndex + 1] : nil
        if op == Self.opLoad {
            sent = [0, 0, 0] // a new ride: a new Engine on this side too
            stampSeq = 0
            if bytes.count >= 9 { human = bytes[bytes.startIndex + 8] != 0 }
        }
        do {
            let s = try sim ?? Simulator(scenarioId: 1, persona: 0, seed: nil, human: false)
            sim = s
            switch try s.applyCtrl(nowNs: now, bytes: bytes) {
            case .ack:
                break
            case let .pong(pingId, tRxNs):
                let tx = Clock.nowNs()
                deliver(encodePong(pingId: pingId, tRxNs: tRxNs, tTxNs: tx), .telem, tx)
            case .disconnect:
                break // no link to drop in-process
            }
            if human {
                s.setHuman(on: true)
                s.setHumanEffort(permille: effort)
                if op == Self.opStart {
                    // A baseline for the first cue's L_onset, once clock sync has a sample.
                    q.asyncAfter(deadline: .now() + 2) { [weak self] in self?.sendRiderInput() }
                }
            }
        } catch {
            print("vc: in-process rig: \(error)")
        }
    }

    /// The on-screen slider moved (human mode in L1).
    func setEffort(_ permille: Int16) {
        effort = permille
        sim?.setHumanEffort(permille: permille)
        sendRiderInput()
    }

    private func sendRiderInput() {
        riderSeq = (riderSeq + 1) & 0xFFFF
        let t = Clock.nowNs()
        deliver(encodeRiderInput(seq: UInt16(riderSeq), tNs: t, effortPermille: effort), .telem, t)
    }

    private func step() {
        guard let s = sim else { return }
        let now = Clock.nowNs()
        for p in s.step(nowNs: now) {
            let source: Source
            let i: Int
            switch p.channel {
            case .hrm: source = .hr; i = 0
            case .cpm: source = .power; i = 1
            case .csc: source = .csc; i = 2
            }
            let t0 = Clock.nowNs() // "handed to the stack"
            deliver(p.bytes, source, Clock.nowNs()) // T1
            sent[i] = (sent[i] + 1) & 0xFFFF
            stampSeq = (stampSeq + 1) & 0xFFFF
            let stamp = encodeStamp(seq: UInt16(stampSeq), channel: p.channel, charSeq: UInt16(sent[i]), t0Ns: t0)
            deliver(stamp, .telem, Clock.nowNs())
        }
    }
}
