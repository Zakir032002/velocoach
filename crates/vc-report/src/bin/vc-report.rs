//! vc-report ledger/*.jsonl -o report.html [--audit]
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = args
        .iter()
        .position(|a| a == "-o")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "report.html".into());
    let audit = args.iter().any(|a| a == "--audit");
    let files: Vec<&String> = args.iter().filter(|a| a.ends_with(".jsonl")).collect();
    let (mut lines, mut bad) = (Vec::new(), 0);
    for f in &files {
        match std::fs::read_to_string(f) {
            Ok(text) => {
                let (l, b) = vc_report::read(&text);
                lines.extend(l);
                bad += b;
            }
            Err(e) => {
                eprintln!("{f}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    if let Err(e) = std::fs::write(
        &out,
        vc_report::html::render(&lines, "velocoach-rs report card"),
    ) {
        eprintln!("{out}: {e}");
        return ExitCode::FAILURE;
    }
    println!(
        "{} lines from {} files ({bad} malformed) -> {out}",
        lines.len(),
        files.len()
    );
    if audit {
        let checks = vc_report::audit::audit(&lines);
        for c in &checks {
            println!(
                "{} {:<16} {}",
                if c.pass { "PASS" } else { "FAIL" },
                c.id,
                c.detail
            );
        }
        if checks.iter().any(|c| !c.pass) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
