//! The reference printed by `hermetic help`.
//!
//! Each topic is a plain-text file under `src/help/`. The `pages` topic ends with a
//! worked example whose note files live under `src/help/example/`; they are spliced
//! into the text where the line `@example` stands, so the files the tests build are
//! the bytes the reader sees.

/// Topic names, in the order the full reference prints them.
pub const TOPICS: [&str; 4] = ["commands", "layout", "syntax", "pages"];

const TITLE: &str = "hermetic reference (`hermetic help <topic>` prints one section)\n";

const COMMANDS: &str = include_str!("help/commands.txt");
const LAYOUT: &str = include_str!("help/layout.txt");
const SYNTAX: &str = include_str!("help/syntax.txt");
const PAGES: &str = include_str!("help/pages.txt");

/// The worked example's note files, as paths relative to the notes root.
pub const EXAMPLE: &[(&str, &str)] = &[
    ("index.md", include_str!("help/example/index.md")),
    (
        "journals/2026-10-12.md",
        include_str!("help/example/journals/2026-10-12.md"),
    ),
    ("pages/auth.md", include_str!("help/example/pages/auth.md")),
];

/// The text of one topic, or `None` if there is no such topic.
pub fn topic(name: &str) -> Option<String> {
    Some(match name {
        "commands" => COMMANDS.to_owned(),
        "layout" => LAYOUT.to_owned(),
        "syntax" => SYNTAX.to_owned(),
        "pages" => with_example(PAGES),
        _ => return None,
    })
}

/// The whole reference: a title line and every topic, separated by blank lines.
pub fn reference() -> String {
    let mut out = String::from(TITLE);
    for name in TOPICS {
        out.push('\n');
        out.push_str(&topic(name).expect("listed topic exists"));
    }
    out
}

/// Replaces the `@example` line with each example file, headed by its path and
/// indented under the "Example" heading.
fn with_example(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line == "@example" {
            for (i, (path, content)) in EXAMPLE.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(&format!("        {path}\n"));
                for l in content.lines() {
                    if l.is_empty() {
                        out.push('\n');
                    } else {
                        out.push_str(&format!("          {l}\n"));
                    }
                }
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_has_every_topic_in_order() {
        let r = reference();
        let mut at = 0;
        for name in TOPICS {
            let section = topic(name).unwrap();
            let found = r[at..].find(&section).unwrap_or_else(|| panic!("{name}"));
            at += found + section.len();
        }
    }

    #[test]
    fn unknown_topic() {
        assert_eq!(topic("frobnicate"), None);
        assert_eq!(topic("Syntax"), None);
    }

    #[test]
    fn example_is_spliced_into_pages() {
        let pages = topic("pages").unwrap();
        assert!(!pages.contains("@example"));
        for (path, _) in EXAMPLE {
            assert!(pages.contains(&format!("        {path}\n")), "{path}");
        }
    }
}
