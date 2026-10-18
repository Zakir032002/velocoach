import CoreBluetooth

enum Uuids {
    static let hrs = CBUUID(string: "180D")
    static let hrm = CBUUID(string: "2A37")
    static let cps = CBUUID(string: "1818")
    static let cpm = CBUUID(string: "2A63")
    static let cpFeature = CBUUID(string: "2A65")
    static let cscs = CBUUID(string: "1816")
    static let csc = CBUUID(string: "2A5B")
    static let vcsim = CBUUID(string: "6C9F0001-5E1A-4C2B-9D3E-7A1B2C3D4E5F")
    static let telem = CBUUID(string: "6C9F0002-5E1A-4C2B-9D3E-7A1B2C3D4E5F")
    static let ctrl = CBUUID(string: "6C9F0003-5E1A-4C2B-9D3E-7A1B2C3D4E5F")

    /// What to discover in each service.
    static let wanted: [CBUUID: [CBUUID]] = [
        hrs: [hrm],
        cps: [cpm, cpFeature],
        cscs: [csc],
        vcsim: [telem, ctrl],
    ]
    static let notifying: Set<CBUUID> = [hrm, cpm, csc, telem]
}
