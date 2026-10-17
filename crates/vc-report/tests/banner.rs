//! Every report page carries the provenance banner (README: "SIMULATED DATA — persona …,
//! seeds …, observer_version …"), built from the ledger's session_start records.
#[test]
fn the_report_says_simulated_data_with_persona_seed_and_observer_version() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../golden/creep_study_11.jsonl"
    ))
    .expect("golden ledger");
    let (lines, bad) = vc_report::read(&text);
    assert_eq!(bad, 0);
    let expected = format!(
        "SIMULATED DATA — persona calib, seeds 11, observer_version {}",
        vc_efficacy::OBSERVER_VERSION
    );
    assert_eq!(vc_report::html::banner(&lines), expected);
    let html = vc_report::html::render(&lines, "t");
    assert!(html.contains(&format!(r#"<div class="sim" role="note">{expected}</div>"#)));
}

#[test]
fn a_ledger_without_a_data_label_still_gets_the_banner() {
    let line = r#"{"record_type":"session_start","schema":"velocoach.irp.v1","observer_version":"vc-efficacy 0.2.0","session_id":"a","rider_id":"R01","mode":"coach","tuner_frozen":false,"policy_hash":"0","protocol_hash":null,"profile":"solo","seed":3,"t_ns":0}"#;
    let (lines, bad) = vc_report::read(line);
    assert_eq!(bad, 0);
    assert_eq!(
        vc_report::html::banner(&lines),
        "SIMULATED DATA — persona not recorded, seeds 3, observer_version vc-efficacy 0.2.0"
    );
}
