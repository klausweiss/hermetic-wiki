use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
hermetic - turn a directory of markdown notes into a static wiki

Usage:
  hermetic build <notes-dir> [<output-dir>]   Build the site (output defaults to ./site)
  hermetic --help                             Show this help
  hermetic --version                          Show the version
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--help" | "-h" | "help"] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["--version" | "-V"] => {
            println!("hermetic {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["build", notes] => build(notes, "site"),
        ["build", notes, out] => build(notes, out),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn build(notes: &str, out: &str) -> ExitCode {
    match hermetic::site::build(Path::new(notes), Path::new(out)) {
        Ok(report) => {
            for d in &report.diagnostics {
                eprintln!("warning: {d}");
            }
            eprintln!(
                "built {} pages into {} ({} warnings)",
                report.pages,
                report.out.display(),
                report.diagnostics.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
