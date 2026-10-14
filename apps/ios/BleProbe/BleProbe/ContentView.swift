//
//  ContentView.swift
//  BleProbe
//
//  Created by ZAKIR HUSSAIN on 05/10/26.
//

import SwiftUI
import CoreBluetooth
import Combine
import VcCore

final class BleProbe: NSObject, ObservableObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    @Published var lines: [String] = []
    @Published var status = "starting"

    private let q = DispatchQueue(label: "ble.serial", qos: .userInteractive)
    private var central: CBCentralManager!
    private var target: CBPeripheral?
    private var lastT1: UInt64 = 0

    private let hrService = CBUUID(string: "180D")
    private let hrmChar = CBUUID(string: "2A37")

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: q)
    }

    private func note(_ s: String, status: String? = nil) {
        DispatchQueue.main.async {
            self.lines.insert(s, at: 0)
            if self.lines.count > 200 { self.lines.removeLast() }
            if let status { self.status = status }
        }
    }

    func centralManagerDidUpdateState(_ c: CBCentralManager) {
        guard c.state == .poweredOn else {
            note("bluetooth state \(c.state.rawValue)", status: "BT unavailable")
            return
        }
        note("scanning for 0x180D", status: "scanning")
        c.scanForPeripherals(withServices: [hrService])
    }

    func centralManager(_ c: CBCentralManager, didDiscover p: CBPeripheral, advertisementData: [String: Any], rssi RSSI: NSNumber) {
        note("found \(p.name ?? "unnamed") rssi \(RSSI)", status: "connecting")
        target = p
        p.delegate = self
        c.stopScan()
        c.connect(p)
    }

    func centralManager(_ c: CBCentralManager, didConnect p: CBPeripheral) {
        p.discoverServices([hrService])
    }

    func centralManager(_ c: CBCentralManager, didDisconnectPeripheral p: CBPeripheral, error: Error?) {
        note("disconnected, reconnecting", status: "reconnecting")
        c.connect(p) // iOS keeps this request pending until the peripheral returns
    }

    func peripheral(_ p: CBPeripheral, didDiscoverServices error: Error?) {
        for s in p.services ?? [] {
            p.discoverCharacteristics([hrmChar], for: s)
        }
    }

    func peripheral(_ p: CBPeripheral, didDiscoverCharacteristicsFor s: CBService, error: Error?) {
        for ch in s.characteristics ?? [] {
            p.setNotifyValue(true, for: ch)
        }
    }

    func peripheral(_ p: CBPeripheral, didUpdateNotificationStateFor ch: CBCharacteristic, error: Error?) {
        note("notify \(ch.isNotifying ? "on" : "off") \(error?.localizedDescription ?? "")",
             status: ch.isNotifying ? "streaming" : "idle")
    }

    func peripheral(_ p: CBPeripheral, didUpdateValueFor ch: CBCharacteristic, error: Error?) {
        let t1 = clock_gettime_nsec_np(CLOCK_UPTIME_RAW) // stamp FIRST, as in the design doc
        guard error == nil, let v = ch.value else { return }
        let dtMs = lastT1 == 0 ? 0 : Double(t1 - lastT1) / 1e6
        lastT1 = t1
        let hex = v.map { String(format: "%02X", $0) }.joined(separator: " ")
        note("\(hex) (+\(Int(dtMs)) ms)")
    }
}
struct ContentView: View {
    @StateObject private var probe = BleProbe()
    @State private var didRunVcSmokeTest = false
    @State private var vcResult = "Rust smoke test: waiting..."

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 8) {
                Text(vcResult)
                    .font(.system(.footnote, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(.horizontal)

                List(Array(probe.lines.enumerated()), id: \.offset) { _, line in
                    Text(line).font(.system(.footnote, design: .monospaced))
                }
            }
            .navigationTitle(probe.status)
            .onAppear {
                guard !didRunVcSmokeTest else { return }
                didRunVcSmokeTest = true
                vcResult = vcSmokeTest()
            }
        }
    }
}

func vcSmokeTest() -> String {
    do {
        let cfg = EngineConfig(
            cpW: 250,
            wprimeJ: 20_000,
            hrMax: 190,
            profile: .solo,
            cooldownBetweenAlertsS: 20,
            promptSurgeCountdownS: 5,
            steps: []
        )

        let engine = try Engine.forScenario(cfg: cfg, scenarioId: 1)
        engine.start(tNs: 0)
        _ = engine.ingest(tNs: 0, source: .power, bytes: Data())

        let s = engine.snapshot()
        let names = scenarios().map(\.name)

        let passed = s.decodeErrors == 1
            && s.policyHash.count == 16
            && names.count == 6

        return """
        Rust FFI smoke test: \(passed ? "PASS" : "CHECK")
        decodeErrors=\(s.decodeErrors)
        policyHash=\(s.policyHash)
        scenarios (\(names.count)): \(names)
        """
    } catch {
        return "Rust FFI smoke test: ERROR\n\(error)"
    }
}

