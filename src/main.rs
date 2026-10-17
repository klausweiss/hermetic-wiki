use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hermetic::help;
use hermetic::serve::{self, Bind};

const USAGE: &str = "\
hermetic - turn a directory of markdown notes into a static wiki

Usage:
  hermetic build <notes-dir> [<output-dir>] [options]
                                              Build the site (output defaults to ./site)
  hermetic serve [<notes-dir>] [options]      Serve the notes as a live site
  hermetic help [<topic>]                     Show the full reference: commands,
                                              notes layout, syntax, and pages
  hermetic --help                             Show this help
  hermetic --version                          Show the version

Options (build and serve):
  --title <name>    Site title shown in every page header (default: the
                    notes directory's name)

Serve options:
  --port <n>        Listen on exactly this port (default: 8000, or the next free one)
  --host <address>  Listen on this address instead of 127.0.0.1
  --public          Listen on all interfaces and show the local-network URL.
                    Anyone on your network can then read the notes.
  --open            Open the wiki in the default browser once it is served
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--help" | "-h"] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["help"] => {
            print!("{}", help::reference());
            ExitCode::SUCCESS
        }
        ["help", name] => match help::topic(name) {
            Some(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            None => {
                eprintln!(
                    "error: unknown help topic `{name}` (topics: {})",
                    help::TOPICS.join(", ")
                );
                ExitCode::from(2)
            }
        },
        ["--version" | "-V"] => {
            println!("hermetic {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["build", rest @ ..] => match parse_build(rest) {
            Ok((notes, out, title)) => build(notes, out, title),
            Err(msg) => usage_error(&msg),
        },
        ["serve", rest @ ..] => match parse_serve(rest) {
            Ok((options, open)) => serve(options, open),
            Err(msg) => usage_error(&msg),
        },
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("error: {msg}\n");
    eprint!("{USAGE}");
    ExitCode::from(2)
}

/// Parses `build` arguments into the notes directory, output directory, and title.
fn parse_build<'a>(args: &[&'a str]) -> Result<(&'a str, &'a str, Option<&'a str>), String> {
    let mut dirs = Vec::new();
    let mut title = None;
    let mut args = args.iter();
    while let Some(&arg) = args.next() {
        match arg {
            "--title" => title = Some(*args.next().ok_or("--title needs a value")?),
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            dir => dirs.push(dir),
        }
    }
    match dirs.as_slice() {
        [notes] => Ok((notes, "site", title)),
        [notes, out] => Ok((notes, out, title)),
        [] => Err("missing notes directory".to_owned()),
        [_, _, extra, ..] => Err(format!("unexpected argument `{extra}`")),
    }
}

fn build(notes: &str, out: &str, title: Option<&str>) -> ExitCode {
    match hermetic::site::build(Path::new(notes), Path::new(out), title) {
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

/// Parses `serve` arguments into the server options and whether to open a browser.
fn parse_serve(args: &[&str]) -> Result<(serve::Options, bool), String> {
    let mut notes: Option<PathBuf> = None;
    let mut port = None;
    let mut host: Option<IpAddr> = None;
    let mut public = false;
    let mut open = false;
    let mut title = None;
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
            "--open" => open = true,
            "--title" => title = Some(value("--title")?.to_owned()),
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
    let options = serve::Options {
        notes: notes.unwrap_or_else(|| PathBuf::from(".")),
        bind,
        port,
        title,
        timing: serve::watch::Timing::default(),
    };
    Ok((options, open))
}

fn serve(options: serve::Options, open: bool) -> ExitCode {
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
            if open
                && let Some(local) = running.urls.first()
                && let Err(e) = open_in_browser(local)
            {
                eprintln!("warning: cannot open a browser: {e}");
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

/// Opens `url` in the default browser with the platform's opener, without waiting for it.
fn open_in_browser(url: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    } else {
        Command::new("xdg-open")
    };
    let mut child = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reap the opener when it exits so it does not linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}
