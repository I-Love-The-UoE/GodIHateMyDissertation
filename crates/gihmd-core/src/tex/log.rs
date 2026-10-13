use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

static FILE_LINE_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<file>(?:[A-Za-z]:)?[^:\n]+?):(?P<line>\d+): (?P<msg>.*)$")
        .unwrap_or_else(|_| never())
});
static BANG_ERROR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^! (?P<msg>.*)$").unwrap_or_else(|_| never()));
static WARNING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<source>LaTeX|Package [\w.-]+|Class [\w.-]+|pdfTeX|LuaTeX|XeTeX|Package|Class) (?P<kind>Warning|Error): (?P<msg>.*?)(?: on input line (?P<line>\d+))?\.?$")
        .unwrap_or_else(|_| never())
});
static BAD_BOX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<kind>Overfull|Underfull) \\[hv]box \((?P<amount>[^)]*)\) (?:in paragraph|detected|has occurred)? ?at lines? (?P<from>\d+)(?:--(?P<to>\d+))?")
        .unwrap_or_else(|_| never())
});
static INPUT_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^l\.(?P<line>\d+)").unwrap_or_else(|_| never()));
static MISSING_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^No file (?P<file>.+)\.$").unwrap_or_else(|_| never()));

#[expect(
    clippy::panic,
    reason = "A constant regex that fails to compile is a programming error."
)]
fn never() -> Regex {
    panic!("Constant regex failed to compile.")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    BadBox,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub message: String,
    pub detail: Option<String>,
    pub log_line: u32,
}

fn resolve(file: &str, source_dirs: &[PathBuf], build_dir: &Path) -> PathBuf {
    let path = Path::new(file.trim().trim_start_matches("./"));

    if path.is_absolute() {
        return path.to_path_buf();
    }

    for dir in source_dirs {
        let candidate = dir.join(path);

        if candidate.is_file() {
            return candidate;
        }
    }

    let in_build = build_dir.join(path);

    if in_build.is_file() {
        return in_build;
    }

    source_dirs
        .first()
        .map_or_else(|| path.to_path_buf(), |dir| dir.join(path))
}

fn looks_like_file(token: &str) -> bool {
    let trimmed = token.trim_end_matches(')');

    trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('.')
}

/// Maintains the TeX file stack from unbalanced parentheses. Reliable once `max_print_line` is
/// raised so that file names are never wrapped.
fn update_stack(line: &str, stack: &mut Vec<String>) {
    let mut index = 0;
    let bytes = line.as_bytes();

    while index < bytes.len() {
        match bytes[index] {
            b'(' => {
                let rest = &line[index + 1..];
                let end = rest
                    .find(|c: char| c == '(' || c == ')' || c.is_whitespace())
                    .unwrap_or(rest.len());
                let token = &rest[..end];

                if looks_like_file(token) {
                    stack.push(token.to_owned());
                }

                index += 1 + end;
            }
            b')' => {
                stack.pop();
                index += 1;
            }
            _ => index += 1,
        }
    }
}

fn parse_line_number(text: &str) -> Option<u32> {
    text.parse().ok()
}

struct Cursor<'a> {
    lines: &'a [&'a str],
    index: usize,
    stack: Vec<String>,
    source_dirs: &'a [PathBuf],
    build_dir: &'a Path,
}

impl Cursor<'_> {
    fn current_file(&self) -> Option<PathBuf> {
        self.stack
            .last()
            .map(|file| resolve(file, self.source_dirs, self.build_dir))
    }

    fn log_line(&self) -> u32 {
        u32::try_from(self.index + 1).unwrap_or(u32::MAX)
    }
}

fn parse_file_line_error(cursor: &Cursor<'_>, line: &str) -> Option<(Diagnostic, usize)> {
    if line.starts_with("l.") {
        return None;
    }

    let capture = FILE_LINE_ERROR.captures(line)?;
    let file = capture.name("file").map_or("", |m| m.as_str());
    let (detail, consumed) = collect_detail(cursor.lines, cursor.index + 1);

    Some((
        Diagnostic {
            severity: Severity::Error,
            file: Some(resolve(file, cursor.source_dirs, cursor.build_dir)),
            line: capture
                .name("line")
                .and_then(|m| parse_line_number(m.as_str())),
            message: capture
                .name("msg")
                .map_or("", |m| m.as_str())
                .trim()
                .to_owned(),
            detail,
            log_line: cursor.log_line(),
        },
        consumed,
    ))
}

fn parse_bang_error(cursor: &Cursor<'_>, line: &str) -> Option<(Diagnostic, usize)> {
    let capture = BANG_ERROR.captures(line)?;
    let (detail, consumed) = collect_detail(cursor.lines, cursor.index + 1);
    let line_number = detail
        .as_deref()
        .and_then(|text| text.lines().find_map(|l| INPUT_LINE.captures(l)))
        .and_then(|c| c.name("line").and_then(|m| parse_line_number(m.as_str())));

    Some((
        Diagnostic {
            severity: Severity::Error,
            file: cursor.current_file(),
            line: line_number,
            message: capture
                .name("msg")
                .map_or("", |m| m.as_str())
                .trim()
                .to_owned(),
            detail,
            log_line: cursor.log_line(),
        },
        consumed,
    ))
}

fn is_warning_continuation(line: &str) -> bool {
    line.starts_with('(')
        && !line.contains(".tex")
        && line
            .split_once(')')
            .is_some_and(|(_, rest)| rest.starts_with(' '))
}

fn parse_warning(cursor: &Cursor<'_>, line: &str) -> Option<(Diagnostic, usize)> {
    let capture = WARNING.captures(line)?;
    let mut message = capture
        .name("msg")
        .map_or("", |m| m.as_str())
        .trim()
        .to_owned();
    let mut line_number = capture
        .name("line")
        .and_then(|m| parse_line_number(m.as_str()));
    let mut consumed = 0;

    while let Some(next) = cursor.lines.get(cursor.index + 1 + consumed)
        && is_warning_continuation(next)
    {
        let continuation = next.split_once(')').map_or("", |(_, rest)| rest).trim();
        message.push(' ');
        message.push_str(continuation.trim_end_matches('.'));

        if let Some(position) = continuation.find("on input line ") {
            line_number = parse_line_number(continuation[position + 14..].trim_end_matches('.'));
        }

        consumed += 1;
    }

    let severity = if capture.name("kind").is_some_and(|m| m.as_str() == "Error") {
        Severity::Error
    } else {
        Severity::Warning
    };

    Some((
        Diagnostic {
            severity,
            file: cursor.current_file(),
            line: line_number,
            message,
            detail: None,
            log_line: cursor.log_line(),
        },
        consumed,
    ))
}

fn parse_bad_box(cursor: &Cursor<'_>, line: &str) -> Option<Diagnostic> {
    let capture = BAD_BOX.captures(line)?;
    let from = capture
        .name("from")
        .and_then(|m| parse_line_number(m.as_str()));
    let to = capture
        .name("to")
        .and_then(|m| parse_line_number(m.as_str()));
    let range = match (from, to) {
        (Some(from), Some(to)) if to != from => format!("lines {from}--{to}"),
        (Some(from), _) => format!("line {from}"),
        _ => String::new(),
    };

    Some(Diagnostic {
        severity: Severity::BadBox,
        file: cursor.current_file(),
        line: from,
        message: format!(
            "{} box ({}) at {range}",
            capture.name("kind").map_or("", |m| m.as_str()),
            capture.name("amount").map_or("", |m| m.as_str()),
        ),
        detail: None,
        log_line: cursor.log_line(),
    })
}

fn parse_missing_file(cursor: &Cursor<'_>, line: &str) -> Option<Diagnostic> {
    let capture = MISSING_FILE.captures(line)?;

    Some(Diagnostic {
        severity: Severity::Info,
        file: cursor.current_file(),
        line: None,
        message: format!(
            "No file {}",
            capture.name("file").map_or("", |m| m.as_str())
        ),
        detail: None,
        log_line: cursor.log_line(),
    })
}

pub fn parse_log(log: &str, source_dirs: &[PathBuf], build_dir: &Path) -> Vec<Diagnostic> {
    let lines: Vec<&str> = log.lines().collect();
    let mut cursor = Cursor {
        lines: &lines,
        index: 0,
        stack: Vec::new(),
        source_dirs,
        build_dir,
    };
    let mut diagnostics = Vec::new();

    while let Some(line) = lines.get(cursor.index) {
        let multi = parse_file_line_error(&cursor, line)
            .or_else(|| parse_bang_error(&cursor, line))
            .or_else(|| parse_warning(&cursor, line));

        if let Some((diagnostic, consumed)) = multi {
            diagnostics.push(diagnostic);
            cursor.index += 1 + consumed;

            continue;
        }

        if let Some(diagnostic) =
            parse_bad_box(&cursor, line).or_else(|| parse_missing_file(&cursor, line))
        {
            diagnostics.push(diagnostic);
        }

        update_stack(line, &mut cursor.stack);
        cursor.index += 1;
    }

    diagnostics.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then(a.log_line.cmp(&b.log_line))
    });
    diagnostics.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.message == b.message);

    diagnostics
}

fn collect_detail(lines: &[&str], start: usize) -> (Option<String>, usize) {
    let mut detail = Vec::new();
    let mut consumed = 0;

    while let Some(line) = lines.get(start + consumed) {
        if line.trim().is_empty() && !detail.is_empty() {
            break;
        }

        if FILE_LINE_ERROR.is_match(line) && !line.starts_with("l.") {
            break;
        }

        if consumed > 8 {
            break;
        }

        detail.push((*line).to_owned());
        consumed += 1;

        if line.starts_with("l.") {
            break;
        }
    }

    let text = detail.join("\n");

    ((!text.trim().is_empty()).then_some(text), consumed)
}
