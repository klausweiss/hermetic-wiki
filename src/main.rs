use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hermetic::serve::{self, Bind};

const USAGE: &str = "\
hermetic - turn a directory of markdown notes into a static wiki

Usage:
  hermetic build <notes-dir> [<output-dir>]   Build the site (output defaults to ./site)
  hermetic serve [<notes-dir>] [options]      Serve the notes as a live site
  hermetic --help                             Show this help
  hermetic --version                          Show the version

Serve options:
  --port <n>        Listen on exactly this port (default: 8000, or the next free one)
  --host <address>  Listen on this address instead of 127.0.0.1
  --public          Listen on all interfaces and show the local-network URL.
                    Anyone on your network can then read the notes.
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
        ["serve", rest @ ..] => match parse_serve(rest) {
            Ok(options) => serve(options),
            Err(msg) => {
                eprintln!("error: {msg}\n");
                eprint!("{USAGE}");
                ExitCode::from(2)
            }
        },
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

fn parse_serve(args: &[&str]) -> Result<serve::Options, String> {
    let mut notes: Option<PathBuf> = None;
    let mut port = None;
    let mut host: Option<IpAddr> = None;
    let mut public = false;
    let mut args = args.iter();
    while let Some(&arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .copied()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match arg {
            "--port" => {
                let v = value("--port")?;
                port = Some(v.parse().map_err(|_| format!("invalid port `{v}`"))?);
            }
            "--host" => {
                let v = value("--host")?;
                host = Some(v.parse().map_err(|_| format!("invalid address `{v}`"))?);
            }
            "--public" => public = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            dir if notes.is_none() => notes = Some(PathBuf::from(dir)),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    let bind = match (host, public) {
        (Some(_), true) => return Err("--host and --public cannot be combined".to_owned()),
        (Some(ip), false) => Bind::Host(ip),
        (None, true) => Bind::Public,
        (None, false) => Bind::Loopback,
    };
    Ok(serve::Options {
        notes: notes.unwrap_or_else(|| PathBuf::from(".")),
        bind,
        port,
        timing: serve::watch::Timing::default(),
    })
}

fn serve(options: serve::Options) -> ExitCode {
    let notes = options.notes.clone();
    match serve::start(options) {
        Ok(running) => {
            for d in &running.diagnostics {
                eprintln!("warning: {d}");
            }
            eprintln!(
                "serving {} pages from {} ({} warnings); press Ctrl+C to stop",
                running.pages,
                notes.display(),
                running.diagnostics.len()
            );
            let mut urls = running.urls.iter();
            if let Some(local) = urls.next() {
                eprintln!("  Local:            {local}");
            }
            for network in urls {
                eprintln!("  On your network:  {network}");
            }
            running.wait();
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
