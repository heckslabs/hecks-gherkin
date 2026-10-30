//! `hecks-gherkin <file>`: prints the bluebook text for a sentence file, or refuses with
//! `file:line:column: message` and exits non-zero.

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: hecks-gherkin <file.bluebook>");
        return ExitCode::from(2);
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::from(2);
        }
    };
    match hecks_gherkin::project(&source) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(diagnostic) => {
            eprintln!("{path}:{diagnostic}");
            ExitCode::FAILURE
        }
    }
}
