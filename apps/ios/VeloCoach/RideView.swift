import SwiftUI
import VcCore

struct RideView: View {
    @StateObject private var model = CoachModel()
    private let scenarioList = scenarios()
    @AppStorage("didCalibrateVolume") private var didCalibrate = false
    @State private var showCalibration = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("link: \(model.link)").font(.headline)
            if let s = model.snapshot {
                Text(String(format: "%.0f W (3 s: %.0f) · %d bpm · %.0f rpm", s.powerW, s.p3W ?? 0, s.hrBpm, s.cadenceRpm))
                Text(String(format: "W′ %.0f %%", s.wbalFraction * 100))
                Text("quality  HR \(s.hrQuality) · power \(s.powerQuality) · cadence \(s.cadenceQuality)")
                Text("step \(s.stepIndex.map { String($0 + 1) } ?? "-") · \(Int(s.stepRemainingS)) s left\(s.paused ? " · paused" : "")")
                Text("decode errors \(s.decodeErrors) · implausible \(s.implausible)").font(.caption)
            }
            if let e = model.error { Text(e).foregroundStyle(.red) }

            Picker("Scenario", selection: $model.scenarioId) {
                ForEach(scenarioList, id: \.id) { Text($0.name).tag($0.id) }
            }
            Picker("Persona", selection: $model.persona) {
                ForEach(Array(["default", "dana", "sam", "ned", "calib"].enumerated()), id: \.offset) {
                    Text($0.element).tag(UInt8($0.offset))
                }
            }
            HStack {
                Button("Start") { model.startRide() }
                    .disabled(model.link != "ready")
                Button("Stop") { model.stopRide() }
                Button("Volume check") { model.audio.calibrate() }
            }
            .buttonStyle(.borderedProminent)

            List(model.cues) { Text($0.text).font(.caption.monospaced()) }
        }
        .padding()
        .onAppear { if !didCalibrate { showCalibration = true } }
        .alert("Set the cue volume", isPresented: $showCalibration) {
            Button("Play test") {
                model.audio.calibrate()
                didCalibrate = true
            }
        } message: {
            Text("Put your earbuds in, tap Play test, and use the side buttons while it speaks.")
        }
    }
}

@main
struct VeloCoachApp: App {
    var body: some Scene {
        WindowGroup { RideView() }
    }
}
