import AVFoundation
import SwiftUI
import VcCore

struct RideView: View {
    @StateObject private var model = CoachModel()
    private let scenarioList = scenarios()
    @AppStorage("didCalibrateVolume") private var didCalibrate = false
    /// d_route(earbuds) from the Day 15 loopback; 0 until measured. Read by the model at Start.
    @AppStorage("routeDelayMs") private var routeDelayMs = 0
    @State private var showCalibration = false
    @State private var showCard = false
    @State private var showLoopback = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                Text("link: \(model.link)").font(.headline)
                Text("audio: \(AVAudioSession.sharedInstance().currentRoute.outputs.first?.portType.rawValue ?? "none")")
                    .font(.caption)
                if let s = model.snapshot {
                    Text(String(format: "%.0f W (3 s: %.0f) · %d bpm · %.0f rpm", s.powerW, s.p3W ?? 0, s.hrBpm, s.cadenceRpm))
                    Text(String(format: "W′ %.0f %%", s.wbalFraction * 100))
                    Text("quality  HR \(s.hrQuality) · power \(s.powerQuality) · cadence \(s.cadenceQuality)")
                    Text("step \(s.stepIndex.map { String($0 + 1) } ?? "-") · \(Int(s.stepRemainingS)) s left\(s.paused ? " · paused" : "")")
                    Text("decode errors \(s.decodeErrors) · implausible \(s.implausible)").font(.caption)
                }
                ForEach(model.latencyPanel, id: \.self) { Text($0).font(.caption2.monospaced()) }
                if let e = model.error { Text(e).foregroundStyle(.red) }

                settings

                HStack {
                    Button("Start") { model.startRide() }
                        .disabled(model.link != "ready" || model.running)
                    Button("Stop") { model.stopRide() }
                    Button("Volume check") { model.audio.calibrate() }
                }
                .buttonStyle(.borderedProminent)

                if model.studyMode && model.running {
                    Text("Session running — the phone stays face down").font(.headline)
                } else {
                    if let b = model.badge {
                        Text(b).font(.system(size: 44, weight: .bold)).transition(.opacity)
                    }
                    ForEach(model.outcomes) { Text($0.text).font(.caption.monospaced()) }
                    ForEach(model.cues) { Text($0.text).font(.caption.monospaced()) }
                    Button("Report card") { showCard = true }
                }

                Button("Loopback") { showLoopback = true }
                if !model.running && !model.lastRideFiles.isEmpty {
                    ShareLink(items: model.lastRideFiles) {
                        Label("Export ride (\(model.lastRideFiles.count) files)", systemImage: "square.and.arrow.up")
                    }
                }
            }
            .padding()
        }
        .onAppear { if !didCalibrate { showCalibration = true } }
        .alert("Set the cue volume", isPresented: $showCalibration) {
            Button("Play test") {
                model.audio.calibrate()
                didCalibrate = true
            }
        } message: {
            Text("Put your earbuds in, tap Play test, and use the side buttons while it speaks.")
        }
        .sheet(isPresented: $showCard) {
            ScrollView { Text(model.reportCard.isEmpty ? "No ride yet." : model.reportCard).font(.body.monospaced()).padding() }
        }
        .sheet(isPresented: $showLoopback) {
            LoopbackView(loop: Loopback(audio: model.audio))
        }
    }

    private var settings: some View {
        VStack(alignment: .leading, spacing: 6) {
            Picker("Scenario", selection: $model.scenarioId) {
                ForEach(scenarioList, id: \.id) { Text($0.name).tag($0.id) }
            }
            Picker("Persona", selection: $model.persona) {
                ForEach(Array(["default", "dana", "sam", "ned", "calib"].enumerated()), id: \.offset) {
                    Text($0.element).tag(UInt8($0.offset))
                }
            }
            Toggle("Human rider (the iQOO slider)", isOn: $model.humanRider)
            Toggle("Study mode", isOn: $model.studyMode)
            if model.studyMode {
                Picker("Rider", selection: $model.rider) {
                    ForEach(0..<StudyConfig.riders.count, id: \.self) { Text(StudyConfig.riders[$0]).tag($0) }
                }
                .pickerStyle(.segmented)
                Picker("Session", selection: $model.session) {
                    ForEach(1...3, id: \.self) { Text("S\($0)").tag($0) }
                }
                .pickerStyle(.segmented)
                Text("seed \(StudyConfig.seed(rider: model.rider, session: model.session)) · protocol \(StudyConfig.protocolHash)")
                    .font(.caption.monospaced())
                // Must read the same in every Study session (M4): check it before pressing Start.
                Text("policy_hash \(model.snapshot?.policyHash ?? model.freshPolicyHash)")
                    .font(.caption.monospaced())
            }
            HStack {
                Text("Route delay (ms)")
                TextField("0", value: $routeDelayMs, format: .number)
                    .keyboardType(.numberPad)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
            }
            HStack {
                Text("HR redline at")
                TextField("–", text: $model.redlineAtS)
                    .keyboardType(.numberPad)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 80)
                Text("s")
            }
            .font(.caption)
        }
        .disabled(model.running)
    }
}

@main
struct VeloCoachApp: App {
    var body: some Scene {
        WindowGroup { RideView() }
    }
}
