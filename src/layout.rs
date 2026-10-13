//! Source directory conventions: which files are journals and pages, how page names map
//! to file paths and back, and how names are normalized and compared.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::diag::{Diagnostic, DiagnosticKind};

pub const JOURNALS_DIR: &str = "journals";
pub const PAGES_DIR: &str = "pages";
pub const ASSETS_DIR: &str = "assets";
/// Directories under the notes root that are read as input.
pub const INPUT_DIRS: [&str; 3] = [JOURNALS_DIR, PAGES_DIR, ASSETS_DIR];
pub const EXTENSION: &str = ".md";
/// The notes root's own markdown file: the page `index`, the site's home page.
pub const ROOT_INDEX: &str = "index.md";
pub const ROOT_INDEX_PAGE: &str = "index";
/// The page a file under `pages/` named `index` becomes: `index` is the home page.
pub const PAGES_INDEX_PAGE: &str = "index (in pages)";

/// Whether `name` is the home page's name (`index`, matched like page names).
pub fn is_home_name(name: &str) -> bool {
    NameKey::new(name) == NameKey::new(ROOT_INDEX_PAGE)
}

/// Whether a file name is a markdown source file (`x.md`, not a bare `.md`).
pub fn is_markdown(name: &str) -> bool {
    name.ends_with(EXTENSION) && name.len() > EXTENSION.len()
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// Trims surrounding whitespace and normalizes to Unicode NFC.
pub fn normalize_name(raw: &str) -> String {
    raw.trim().nfc().collect()
}

/// Why a page name is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    ContainsNul,
    EmptySegment,
    DotSegment,
    LeadingDot,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameError::Empty => "page name is empty",
            NameError::ContainsNul => "page name contains NUL",
            NameError::EmptySegment => "page name has an empty `/` segment",
            NameError::DotSegment => "page name has a `.` or `..` segment",
            NameError::LeadingDot => "page name has a segment starting with `.`",
        })
    }
}

/// Checks a page name (as written; it is trimmed first) for validity.
pub fn validate_name(name: &str) -> Result<(), NameError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.contains('\0') {
        return Err(NameError::ContainsNul);
    }
    for segment in name.split('/') {
        if segment.is_empty() {
            return Err(NameError::EmptySegment);
        }
        if segment == "." || segment == ".." {
            return Err(NameError::DotSegment);
        }
        if segment.starts_with('.') {
            return Err(NameError::LeadingDot);
        }
    }
    Ok(())
}

/// Key used to decide whether two names refer to the same page:
/// trimmed, NFC-normalized and lowercased.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NameKey(String);

impl NameKey {
    pub fn new(name: &str) -> Self {
        NameKey(normalize_name(name).to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Name <-> file path encoding
// ---------------------------------------------------------------------------

const RESERVED_DEVICE_NAMES: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn is_reserved_device_name(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or(segment);
    RESERVED_DEVICE_NAMES
        .iter()
        .any(|reserved| base.eq_ignore_ascii_case(reserved))
}

fn is_unsafe_char(c: char) -> bool {
    matches!(
        c,
        '\u{0}'..='\u{1f}' | '\u{7f}' | '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' | '%'
    )
}

fn push_escaped(out: &mut String, c: char) {
    let mut buf = [0u8; 4];
    for byte in c.encode_utf8(&mut buf).bytes() {
        out.push_str(&format!("%{byte:02X}"));
    }
}

/// Encodes one `/`-separated segment of a page name into a portable file name stem.
pub fn encode_segment(segment: &str) -> String {
    let reserved = is_reserved_device_name(segment);
    let last = segment.char_indices().last().map(|(i, _)| i);
    let mut out = String::with_capacity(segment.len());
    for (i, c) in segment.char_indices() {
        let escape = is_unsafe_char(c)
            || (i == 0 && reserved)
            || (Some(i) == last && (c == '.' || c == ' '));
        if escape {
            push_escaped(&mut out, c);
        } else {
            out.push(c);
        }
    }
    out
}

/// Decodes `%XX` sequences in a file name segment. A `%` not followed by two hex digits is
/// kept literally. If decoding would produce invalid UTF-8, the segment is returned as is.
pub fn decode_segment(segment: &str) -> String {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2]))
        {
            out.push(hi * 16 + lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| segment.to_string())
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Relative path (`pages/...md`) of the file that stores a regular page.
/// The name must be valid (see [`validate_name`]).
pub fn page_file_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(PAGES_DIR);
    let name = normalize_name(name);
    let segments: Vec<&str> = name.split('/').collect();
    let (last, dirs) = segments.split_last().expect("valid names have a segment");
    for dir in dirs {
        path.push(encode_segment(dir));
    }
    path.push(format!("{}{EXTENSION}", encode_segment(last)));
    path
}

/// Relative path (`journals/YYYY_MM_DD.md`) of the file a new journal day is written to.
pub fn journal_file_path(date: Date) -> PathBuf {
    let stem = date.to_string().replace('-', "_");
    PathBuf::from(JOURNALS_DIR).join(format!("{stem}{EXTENSION}"))
}

/// Relative path (`journals/YYYY-MM-DD.html`) of a journal day's HTML page.
pub fn journal_html_path(date: Date) -> PathBuf {
    PathBuf::from(JOURNALS_DIR).join(format!("{date}.html"))
}

/// Page name of a regular page stored at `rel` (relative to the root, under `pages/`).
pub fn page_name_from_path(rel: &Path) -> Option<String> {
    let mut components = rel.components();
    if components.next()?.as_os_str() != PAGES_DIR {
        return None;
    }
    let segments: Vec<String> = components
        .map(|c| c.as_os_str().to_str().map(str::to_owned))
        .collect::<Option<_>>()?;
    let (last, dirs) = segments.split_last()?;
    let stem = last.strip_suffix(EXTENSION)?;
    let mut decoded: Vec<String> = dirs.iter().map(|d| decode_segment(d)).collect();
    decoded.push(decode_segment(stem));
    Some(normalize_name(&decoded.join("/")))
}

// ---------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------

/// A calendar date, ordered chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// Parses a strict `YYYY-MM-DD` string with a valid calendar date.
    pub fn parse(s: &str) -> Option<Date> {
        let b = s.as_bytes();
        if !s.is_ascii() || b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let num = |range: std::ops::Range<usize>| -> Option<u16> {
            let part = &s[range];
            if !part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        let year = num(0..4)?;
        let month = num(5..7)? as u8;
        let day = num(8..10)? as u8;
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Date { year, month, day })
    }

    /// Parses the stem of a journal file name: `YYYY-MM-DD`, or `YYYY_MM_DD` as some
    /// other tools name them. New files are always written with underscores.
    pub fn parse_file_stem(stem: &str) -> Option<Date> {
        let b = stem.as_bytes();
        if b.len() == 10 && b[4] == b'_' && b[7] == b'_' {
            return Date::parse(&stem.replace('_', "-"));
        }
        Date::parse(stem)
    }
}

impl Date {
    /// Days since 1970-01-01 (proleptic Gregorian calendar).
    fn days_from_epoch(self) -> i64 {
        let (m, d) = (i64::from(self.month), i64::from(self.day));
        let y = i64::from(self.year) - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// English name of the day of the week.
    pub fn weekday(self) -> &'static str {
        const NAMES: [&str; 7] = [
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
        ];
        NAMES[self.days_from_epoch().rem_euclid(7) as usize]
    }

    /// English name of the month.
    pub fn month_name(self) -> &'static str {
        const NAMES: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        NAMES[usize::from(self.month) - 1]
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            let leap =
                (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
            if leap { 29 } else { 28 }
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Journal(Date),
    Regular,
}

/// A markdown file that becomes a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Path relative to the notes root, using `/` separators.
    pub rel: PathBuf,
    /// Normalized page name (not yet validated).
    pub name: String,
    pub kind: SourceKind,
}

/// Lists the notes root's journal and page files in scan order: journals by ascending
/// date (then path), followed by all other pages by ascending path bytes.
pub fn scan(root: &Path, diags: &mut Vec<Diagnostic>) -> Vec<SourceFile> {
    let mut journals = Vec::new();
    let mut pages = Vec::new();

    let mut journal_files = Vec::new();
    collect_markdown(root, Path::new(JOURNALS_DIR), &mut journal_files, diags);
    for rel in journal_files {
        let stem = rel
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(EXTENSION))
            .unwrap_or_default()
            .to_owned();
        match Date::parse_file_stem(&stem) {
            Some(date) => journals.push(SourceFile {
                rel,
                name: date.to_string(),
                kind: SourceKind::Journal(date),
            }),
            None => {
                diags.push(Diagnostic::new(
                    DiagnosticKind::InvalidJournalName,
                    rel.clone(),
                    None,
                    "journal file name is not a valid YYYY-MM-DD (or YYYY_MM_DD) date; treated as a regular page",
                ));
                pages.push(SourceFile {
                    rel,
                    name: normalize_name(&decode_segment(&stem)),
                    kind: SourceKind::Regular,
                });
            }
        }
    }

    let mut page_files = Vec::new();
    collect_markdown(root, Path::new(PAGES_DIR), &mut page_files, diags);
    for rel in page_files {
        if let Some(name) = page_name_from_path(&rel) {
            pages.push(SourceFile {
                rel,
                name,
                kind: SourceKind::Regular,
            });
        }
    }

    if root.join(ROOT_INDEX).is_file() {
        pages.push(SourceFile {
            rel: PathBuf::from(ROOT_INDEX),
            name: ROOT_INDEX_PAGE.to_owned(),
            kind: SourceKind::Regular,
        });
    }

    journals.sort_by(|a, b| {
        let (SourceKind::Journal(da), SourceKind::Journal(db)) = (a.kind, b.kind) else {
            unreachable!("only journals here")
        };
        da.cmp(&db)
            .then_with(|| path_bytes(&a.rel).cmp(path_bytes(&b.rel)))
    });
    pages.sort_by(|a, b| path_bytes(&a.rel).cmp(path_bytes(&b.rel)));
    journals.extend(pages);
    journals
}

/// Byte representation of a relative path, used for deterministic ordering.
pub fn path_bytes(p: &Path) -> &[u8] {
    p.as_os_str().as_encoded_bytes()
}

/// Recursively collects `.md` files under `root/rel`, skipping dot-entries.
/// Paths are relative to `root`. Unreadable directories are reported.
fn collect_markdown(root: &Path, rel: &Path, out: &mut Vec<PathBuf>, diags: &mut Vec<Diagnostic>) {
    let dir = root.join(rel);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            diags.push(Diagnostic::new(
                DiagnosticKind::UnreadableFile,
                rel,
                None,
                format!("cannot read directory: {e}"),
            ));
            return;
        }
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            diags.push(Diagnostic::new(
                DiagnosticKind::UnreadableFile,
                rel.join(&file_name),
                None,
                "file name is not valid UTF-8",
            ));
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let child = rel.join(name);
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // Follow symlinks to decide what the entry is.
        let is_dir = file_type.is_dir() || (file_type.is_symlink() && root.join(&child).is_dir());
        if is_dir {
            collect_markdown(root, &child, out, diags);
        } else if is_markdown(name) {
            out.push(child);
        }
    }
}

/// Reads a source file as UTF-8 text with LF line endings. Problems are reported and
/// yield `None`.
pub fn read_source(root: &Path, rel: &Path, diags: &mut Vec<Diagnostic>) -> Option<String> {
    let bytes = match fs::read(root.join(rel)) {
        Ok(bytes) => bytes,
        Err(e) => {
            diags.push(Diagnostic::new(
                DiagnosticKind::UnreadableFile,
                rel,
                None,
                format!("cannot read file: {e}"),
            ));
            return None;
        }
    };
    match String::from_utf8(bytes) {
        Ok(text) => Some(if text.contains('\r') {
            text.replace("\r\n", "\n")
        } else {
            text
        }),
        Err(_) => {
            diags.push(Diagnostic::new(
                DiagnosticKind::InvalidUtf8,
                rel,
                None,
                "file is not valid UTF-8; skipped",
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn invalid_names() {
        let cases = [
            ("", NameError::Empty),
            ("   ", NameError::Empty),
            ("a\0b", NameError::ContainsNul),
            ("a//b", NameError::EmptySegment),
            ("/a", NameError::EmptySegment),
            ("a/", NameError::EmptySegment),
            (".", NameError::DotSegment),
            ("..", NameError::DotSegment),
            ("../../etc/passwd", NameError::DotSegment),
            ("a/../b", NameError::DotSegment),
            (".hidden", NameError::LeadingDot),
            ("a/.b", NameError::LeadingDot),
        ];
        for (name, err) in cases {
            assert_eq!(validate_name(name), Err(err), "{name:?}");
        }
        for ok in [
            "auth",
            "Project X/Auth Flow",
            "C#",
            "50% done",
            "a.b",
            "con",
        ] {
            assert_eq!(validate_name(ok), Ok(()), "{ok:?}");
        }
    }

    #[test]
    fn spec_encoding_scenarios() {
        assert_eq!(
            page_file_path("C#: a|b?"),
            PathBuf::from("pages/C#%3A a%7Cb%3F.md")
        );
        assert_eq!(
            page_file_path("50% done"),
            PathBuf::from("pages/50%25 done.md")
        );
        assert_eq!(page_file_path("con"), PathBuf::from("pages/%63on.md"));
        assert_eq!(
            page_file_path("con.txt"),
            PathBuf::from("pages/%63on.txt.md")
        );
        assert_eq!(page_file_path("etc."), PathBuf::from("pages/etc%2E.md"));
        assert_eq!(
            page_file_path("Project X/Auth Flow"),
            PathBuf::from("pages/Project X/Auth Flow.md")
        );
        assert_eq!(page_file_path("a /b"), PathBuf::from("pages/a%20/b.md"));
    }

    #[test]
    fn decoding() {
        assert_eq!(decode_segment("50%25 done"), "50% done");
        assert_eq!(decode_segment("%63on"), "con");
        assert_eq!(decode_segment("a%3ab"), "a:b");
        assert_eq!(decode_segment("100%"), "100%");
        assert_eq!(decode_segment("%zz"), "%zz");
        assert_eq!(decode_segment("%FF"), "%FF");
        assert_eq!(decode_segment("a:b"), "a:b");
        assert_eq!(
            page_name_from_path(Path::new("pages/a:b.md")).as_deref(),
            Some("a:b")
        );
        assert_eq!(
            page_name_from_path(Path::new("pages/Project X/Auth Flow.md")).as_deref(),
            Some("Project X/Auth Flow")
        );
    }

    #[test]
    fn name_keys() {
        assert_eq!(NameKey::new("Auth"), NameKey::new(" AUTH "));
        assert_eq!(NameKey::new("Zu\u{308}rich"), NameKey::new("Z\u{fc}rich"));
        assert_ne!(NameKey::new("auth"), NameKey::new("oauth"));
    }

    #[test]
    fn dates() {
        assert_eq!(
            Date::parse("2024-02-29"),
            Some(Date {
                year: 2024,
                month: 2,
                day: 29
            })
        );
        assert_eq!(Date::parse("2000-02-29").map(|d| d.day), Some(29));
        for bad in [
            "2023-02-29",
            "1900-02-29",
            "2026-13-40",
            "2026-00-10",
            "2026-04-31",
            "notes",
            "2026-1-01",
            "+026-01-01",
            "żó-01-01",
        ] {
            assert_eq!(Date::parse(bad), None, "{bad}");
        }
        let d = Date::parse("2026-10-12").unwrap();
        assert_eq!(d.to_string(), "2026-10-12");
        assert_eq!(Date::parse("2026_10_12"), None);
        assert_eq!(Date::parse_file_stem("2026_10_12"), Some(d));
        assert_eq!(Date::parse_file_stem("2026-10-12"), Some(d));
        for bad in ["2026_10-12", "2026-10_12", "2026_13_01", "2026_1_012"] {
            assert_eq!(Date::parse_file_stem(bad), None, "{bad}");
        }
        assert_eq!(
            journal_file_path(d),
            PathBuf::from("journals/2026_10_12.md")
        );
        assert_eq!(
            journal_html_path(d),
            PathBuf::from("journals/2026-10-12.html")
        );
        assert!(Date::parse("2026-09-30").unwrap() < d);
        assert_eq!(d.weekday(), "Monday");
        assert_eq!(d.month_name(), "October");
        assert_eq!(Date::parse("2024-02-29").unwrap().weekday(), "Thursday");
        assert_eq!(Date::parse("1970-01-01").unwrap().weekday(), "Thursday");
        assert_eq!(Date::parse("2000-03-01").unwrap().weekday(), "Wednesday");
    }

    fn valid_name() -> impl Strategy<Value = String> {
        // Segments of arbitrary characters, including ones that must be escaped.
        let segment =
            proptest::string::string_regex(r#"[a-zA-Z0-9 .%:<>"\\|?*#\x01\x7fżółć€😀-]{1,12}"#)
                .unwrap();
        proptest::collection::vec(segment, 1..4)
            .prop_map(|segs| normalize_name(&segs.join("/")))
            .prop_filter("valid names only", |n| validate_name(n).is_ok())
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config {
            // Keep regression seeds next to the source instead of in a top-level directory.
            failure_persistence: Some(Box::new(
                proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"),
            )),
            ..Default::default()
        })]
        #[test]
        fn encoding_round_trips(name in valid_name()) {
            let path = page_file_path(&name);
            prop_assert_eq!(page_name_from_path(&path), Some(name.clone()));
            // Encoded file names contain no characters unsafe on any major platform.
            for component in path.components().skip(1) {
                let s = component.as_os_str().to_str().unwrap();
                let stem = s.strip_suffix(".md").unwrap_or(s);
                prop_assert!(!stem.chars().any(|c| is_unsafe_char(c) && c != '%'));
                prop_assert!(!stem.ends_with('.') && !stem.ends_with(' '));
                prop_assert!(!is_reserved_device_name(stem));
            }
        }
    }
}
