#[path = "../cli/validation.rs"]
mod validation;

use std::path::Path;

#[test]
fn cases_dom_match() {
    let records = validation::run_cases(Path::new("validation/cases")).expect("run cases");
    let (_, failed) = validation::summarize(&records);

    let failures: Vec<_> = records
        .iter()
        .filter(|record| record.error.is_some() || !record.matched)
        .map(validation::failure_message)
        .collect();

    assert!(
        failed == 0 && failures.is_empty(),
        "validation cases failed:\n{}",
        failures.join("\n")
    );
}
