//! The ledger schema (velocoach.irp.v1) is a never-cut item: every record of the committed
//! golden ledger parses into the typed schema and serialises back to the same bytes, so a
//! field cannot be renamed, dropped or reordered without this test failing.
#[test]
fn the_golden_ledger_round_trips_byte_for_byte_through_the_schema() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../golden/creep_study_11.jsonl"
    ))
    .expect("golden ledger");
    let (lines, bad) = vc_report::read(&text);
    assert_eq!(bad, 0, "malformed lines");
    let originals: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), originals.len());
    for (line, original) in lines.iter().zip(&originals) {
        assert_eq!(vc_report::to_json(line), *original);
    }
    assert!(
        originals
            .iter()
            .all(|l| l.contains(r#""schema":"velocoach.irp.v1""#))
    );
}
