import CoreBluetooth
import VcCore

/// Called on ble.q.
protocol BLECentralDelegate: AnyObject {
    func central(didReceive bytes: Data, from source: Source, t1: UInt64)
    /// Before any notification of the new link: the rig's packet numbering restarts here.
    func centralDidConnect()
    func centralDidBecomeReady()
    func centralDidDisconnect()
}

final class BLECentral: NSObject, CBCentralManagerDelegate, CBPeripheralDelegate {
    let queue = DispatchQueue(label: "ble.q", qos: .userInteractive)
    weak var delegate: BLECentralDelegate?

    private var manager: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var ctrl: CBCharacteristic?
    private var subscribed = Set<CBUUID>()
    private var heldWrite: Data?
    /// For the screen; written on ble.q.
    private(set) var stateText = "idle"

    override init() {
        super.init()
        manager = CBCentralManager(delegate: self, queue: queue,
                                   options: [CBCentralManagerOptionShowPowerAlertKey: true])
    }

    // MARK: writes (call on ble.q)

    func write(_ data: Data, withResponse: Bool) {
        guard let p = peripheral, let c = ctrl else { return }
        if withResponse {
            p.writeValue(data, for: c, type: .withResponse)
        } else if p.canSendWriteWithoutResponse {
            p.writeValue(data, for: c, type: .withoutResponse)
        } else {
            heldWrite = data // sent from peripheralIsReady
        }
    }

    // MARK: central

    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        if central.state == .poweredOn {
            scan()
        } else {
            stateText = "bluetooth \(central.state.rawValue)"
        }
    }

    private func scan() {
        stateText = "scanning"
        manager.scanForPeripherals(withServices: [Uuids.cps], options: nil)
    }

    func centralManager(_ central: CBCentralManager, didDiscover p: CBPeripheral,
                        advertisementData: [String: Any], rssi RSSI: NSNumber) {
        guard peripheral == nil else { return }
        peripheral = p
        manager.stopScan()
        stateText = "connecting"
        manager.connect(p, options: nil)
    }

    func centralManager(_ central: CBCentralManager, didConnect p: CBPeripheral) {
        stateText = "discovering"
        delegate?.centralDidConnect()
        p.delegate = self
        p.discoverServices(Array(Uuids.wanted.keys))
    }

    func centralManager(_ central: CBCentralManager, didFailToConnect p: CBPeripheral, error: Error?) {
        reconnect(p)
    }

    func centralManager(_ central: CBCentralManager, didDisconnectPeripheral p: CBPeripheral, error: Error?) {
        reconnect(p)
    }

    private func reconnect(_ p: CBPeripheral) {
        subscribed.removeAll()
        ctrl = nil
        heldWrite = nil
        stateText = "reconnecting"
        delegate?.centralDidDisconnect()
        manager.connect(p, options: nil) // no timeout: completes whenever the rig is back
    }

    // MARK: peripheral

    func peripheral(_ p: CBPeripheral, didDiscoverServices error: Error?) {
        for s in p.services ?? [] {
            p.discoverCharacteristics(Uuids.wanted[s.uuid], for: s)
        }
    }

    func peripheral(_ p: CBPeripheral, didDiscoverCharacteristicsFor s: CBService, error: Error?) {
        for c in s.characteristics ?? [] {
            if Uuids.notifying.contains(c.uuid) {
                p.setNotifyValue(true, for: c)
            } else if c.uuid == Uuids.ctrl {
                ctrl = c
            } else if c.uuid == Uuids.cpFeature {
                p.readValue(for: c) // logged once; Day 11 checks the crank bit
            }
        }
    }

    func peripheral(_ p: CBPeripheral, didUpdateNotificationStateFor c: CBCharacteristic, error: Error?) {
        if c.isNotifying { subscribed.insert(c.uuid) }
        if subscribed == Uuids.notifying && ctrl != nil {
            stateText = "ready"
            delegate?.centralDidBecomeReady()
        }
    }

    func peripheral(_ p: CBPeripheral, didUpdateValueFor c: CBCharacteristic, error: Error?) {
        let t1 = Clock.nowNs() // T1: the first line, before anything else
        if c.uuid == Uuids.cpFeature, let v = c.value, v.count >= 4 {
            let crank = (v[0] & 0x08) != 0
            stateText = crank ? stateText : "meter has no crank data: no cadence"
            print("vc: CP Feature \(v.map { String(format: "%02X", $0) }.joined()) crank=\(crank)")
            return
        }
        guard error == nil, let bytes = c.value, let source = Self.source(of: c.uuid) else { return }
        delegate?.central(didReceive: bytes, from: source, t1: t1)
    }

    func peripheralIsReady(toSendWriteWithoutResponse p: CBPeripheral) {
        if let data = heldWrite, let c = ctrl {
            heldWrite = nil
            p.writeValue(data, for: c, type: .withoutResponse)
        }
    }

    private static func source(of uuid: CBUUID) -> Source? {
        switch uuid {
        case Uuids.hrm: return .hr
        case Uuids.cpm: return .power
        case Uuids.csc: return .csc
        case Uuids.telem: return .telem
        default: return nil // CP Feature read: not a stream
        }
    }
}
