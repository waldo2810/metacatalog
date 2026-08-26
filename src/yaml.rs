// Copyright 2026 The MetaCatalog Authors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! A parser for the subset of YAML the catalog format uses, carrying the source
//! line of every node.
//!
//! The project has a zero-dependency policy, so there is no YAML crate to lean
//! on. The real product of this module is not "YAML to data" — it is a line
//! number on every node, so that validation can say
//! `catalog/warehouse/dim_customer.yml:17: error: ...` and point at the exact
//! key that is wrong. Consumers keep the tree alive for the whole load and walk
//! it directly; discarding it after parsing is what makes line reporting
//! impossible to retrofit.
//!
//! # Supported subset
//!
//! Block mappings, block sequences, nesting by indentation (spaces only), plain
//! scalars, and single- and double-quoted scalars, plus `#` comments.
//!
//! # Rejected, rather than silently misparsed
//!
//! Flow style (`{a: 1}`, `[1, 2]`), anchors and aliases (`&x`, `*x`),
//! multi-document streams (`---`, `...`), tags (`!!str`), block scalars (`|`,
//! `>`), explicit keys (`? a`), directives (`%YAML`), and multi-line quoted
//! scalars. Each fails with [`ErrorKind::UnsupportedFeature`], naming the
//! feature, the file and the line.
//!
//! # Deliberate deviations from YAML 1.1
//!
//! `yes`/`no`/`on`/`off` are strings, not booleans — only `true` and `false`
//! (and their `True`/`TRUE` casings) type as `Bool`. That keeps `nullable: no`
//! meaning what it reads as. Scalars with leading zeros (`007`) stay strings,
//! as do integers too large for an `i64`.
//!
//! # Example
//!
//! ```
//! use std::path::Path;
//! use mc::yaml::{parse_str, Value};
//!
//! let doc = parse_str("table: DimCustomer\n", Path::new("dim_customer.yml"))?;
//! assert_eq!(doc.get("table").and_then(|n| n.as_str()), Some("DimCustomer"));
//! assert_eq!(doc.get_key("table").map(|k| k.line), Some(1));
//! # Ok::<(), mc::yaml::ParseError>(())
//! ```

use std::fmt;
use std::path::{Path, PathBuf};

/// A parsed value together with the 1-based source line it appeared on.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// 1-based line in the source file.
    pub line: usize,
    /// The value itself.
    pub value: Value,
}

/// The value of a [`Node`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `~`, `null`, or an empty value.
    Null,
    /// `true` or `false` only — see the module docs on `yes`/`no`.
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// A block sequence.
    Seq(Vec<Node>),
    /// A block mapping. A `Vec` rather than a map: it preserves source order
    /// and lets duplicate keys be detected and reported with both lines.
    Map(Vec<(Key, Node)>),
}

/// A mapping key, with the position it was written at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The key text, with quotes removed and escapes decoded.
    pub name: String,
    /// 1-based line in the source file.
    pub line: usize,
    /// 1-based column of the key's first character.
    pub col: usize,
}

impl Node {
    /// The value of `key` in this mapping, or `None` if this is not a mapping
    /// or the key is absent.
    pub fn get(&self, key: &str) -> Option<&Node> {
        match &self.value {
            Value::Map(entries) => entries.iter().find(|(k, _)| k.name == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The [`Key`] of `key` in this mapping — the line of the key itself, which
    /// is what a "this field is wrong" message points at. `None` distinguishes
    /// a missing key from one that is present with an empty value.
    pub fn get_key(&self, key: &str) -> Option<&Key> {
        match &self.value {
            Value::Map(entries) => entries.iter().find(|(k, _)| k.name == key).map(|(k, _)| k),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match &self.value {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match &self.value {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Seq(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(Key, Node)]> {
        match &self.value {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }

    /// The node's type as it should appear in a message, e.g. "expected a
    /// mapping, found a string".
    pub fn type_name(&self) -> &'static str {
        match &self.value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Seq(_) => "sequence",
            Value::Map(_) => "mapping",
        }
    }
}

/// What went wrong, so that callers and tests can branch on the cause rather
/// than on message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// A YAML construct this parser deliberately does not support.
    UnsupportedFeature,
    /// A tab in indentation, or a line that lines up with no open block.
    Indentation,
    /// A quoted scalar with no closing quote on its line.
    UnterminatedQuote,
    /// The same key twice in one mapping.
    DuplicateKey,
    /// Anything else malformed.
    Syntax,
    /// The file could not be read, or was not valid UTF-8. Carries line 0.
    Io,
}

/// A parse failure, always naming the file and (except for [`ErrorKind::Io`])
/// the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub file: PathBuf,
    /// 1-based line, or 0 when the failure is not tied to one.
    pub line: usize,
    /// 1-based column, or 0 when the failure is not tied to one.
    pub col: usize,
    pub kind: ErrorKind,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: error: {}", self.file.display(), self.message)
        } else {
            write!(
                f,
                "{}:{}: error: {}",
                self.file.display(),
                self.line,
                self.message
            )
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse `path` into a line-tracked tree.
pub fn parse_file(path: &Path) -> Result<Node, ParseError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(err(
                path,
                ErrorKind::Io,
                0,
                0,
                format!("could not read file: {e}"),
            ));
        }
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            return Err(err(
                path,
                ErrorKind::Io,
                0,
                0,
                "file is not valid UTF-8".to_string(),
            ));
        }
    };
    parse_str(&text, path)
}

/// Parse `text` into a line-tracked tree. `file` is used for error messages
/// only; it is never read.
pub fn parse_str(text: &str, file: &Path) -> Result<Node, ParseError> {
    let lines = scan_lines(text, file)?;
    let indent = match lines.first() {
        Some(line) => line.indent,
        // A file that is empty, blank, or nothing but comments is a null
        // document, not an error.
        None => {
            return Ok(Node {
                line: 1,
                value: Value::Null,
            });
        }
    };
    let mut parser = Parser {
        file,
        lines,
        idx: 0,
        col: indent,
    };
    let node = parser.parse_block(indent)?;
    if let Some(line) = parser.lines.get(parser.idx) {
        return Err(err(
            file,
            ErrorKind::Indentation,
            line.number,
            parser.col + 1,
            format!(
                "inconsistent indentation: this line is indented {} spaces, which lines up with no open block",
                parser.col
            ),
        ));
    }
    Ok(node)
}

// --- the line pre-pass -------------------------------------------------------

/// A significant source line: blank and comment-only lines never make it into
/// the list, and nothing is ever renumbered, so `number` stays the physical
/// line of the original file.
struct Line {
    number: usize,
    /// Leading spaces. Equal to the column the content starts at.
    indent: usize,
    /// The whole line, comment stripped and right-trimmed, indent included.
    /// Held as `char`s so that no byte-offset can ever split a UTF-8 sequence.
    chars: Vec<char>,
}

fn err(file: &Path, kind: ErrorKind, line: usize, col: usize, message: String) -> ParseError {
    ParseError {
        file: file.to_path_buf(),
        line,
        col,
        kind,
        message,
    }
}

fn eof(file: &Path) -> ParseError {
    err(
        file,
        ErrorKind::Syntax,
        0,
        0,
        "unexpected end of file".to_string(),
    )
}

/// Bounds-checked slice of a line into an owned `String`; never panics.
fn slice(chars: &[char], from: usize, to: usize) -> String {
    let from = from.min(chars.len());
    let to = to.min(chars.len()).max(from);
    chars[from..to].iter().collect()
}

fn scan_lines(text: &str, file: &Path) -> Result<Vec<Line>, ParseError> {
    let mut out = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        let number = i + 1;
        let mut raw = raw;
        if number == 1
            && let Some(rest) = raw.strip_prefix('\u{feff}')
        {
            raw = rest;
        }
        if let Some(rest) = raw.strip_suffix('\r') {
            raw = rest;
        }
        // Blank lines carry no indentation to be wrong about.
        if raw.trim().is_empty() {
            continue;
        }
        let mut chars: Vec<char> = raw.chars().collect();
        let mut indent = 0usize;
        loop {
            match chars.get(indent) {
                Some(' ') => indent += 1,
                Some('\t') => {
                    return Err(err(
                        file,
                        ErrorKind::Indentation,
                        number,
                        indent + 1,
                        "tab character in indentation: indent with spaces".to_string(),
                    ));
                }
                _ => break,
            }
        }
        if matches!(chars.get(indent), Some('#')) {
            continue;
        }
        let mut end = strip_comment(&chars, indent);
        while end > indent && matches!(chars.get(end - 1), Some(' ') | Some('\t')) {
            end -= 1;
        }
        if end <= indent {
            continue;
        }
        chars.truncate(end);
        out.push(Line {
            number,
            indent,
            chars,
        });
    }
    Ok(out)
}

/// The index at which a `#` comment starts, or `chars.len()` if there is none.
///
/// A `#` opens a comment only at the start of the content or when preceded by
/// whitespace, and never inside a quoted scalar. Quotes are only recognised
/// where a scalar may start, so `don't # note` strips correctly.
fn strip_comment(chars: &[char], start: usize) -> usize {
    let mut i = start;
    let mut token_start = true;
    while let Some(&c) = chars.get(i) {
        match c {
            '\'' | '"' if token_start => match scan_quoted_end(chars, i) {
                Some(end) => {
                    i = end;
                    token_start = false;
                }
                // Unterminated: keep the rest of the line so that the parser
                // reports UnterminatedQuote rather than a truncated scalar.
                None => return chars.len(),
            },
            '#' if i == start || matches!(chars.get(i - 1), Some(' ') | Some('\t')) => return i,
            ' ' | '\t' => i += 1,
            '-' if token_start && matches!(chars.get(i + 1), None | Some(' ')) => i += 1,
            ':' if matches!(chars.get(i + 1), None | Some(' ')) => {
                i += 1;
                token_start = true;
            }
            _ => {
                i += 1;
                token_start = false;
            }
        }
    }
    chars.len()
}

/// The index just past the closing quote of the scalar starting at `start`, or
/// `None` if the line ends first.
fn scan_quoted_end(chars: &[char], start: usize) -> Option<usize> {
    let quote = *chars.get(start)?;
    let mut i = start + 1;
    while let Some(&c) = chars.get(i) {
        if quote == '\'' {
            if c == '\'' {
                if matches!(chars.get(i + 1), Some('\'')) {
                    i += 2;
                    continue;
                }
                return Some(i + 1);
            }
            i += 1;
        } else {
            match c {
                '\\' => i += 2,
                '"' => return Some(i + 1),
                _ => i += 1,
            }
        }
    }
    None
}

/// The index of the `:` that ends a mapping key starting at `from`, or `None`
/// if this is not a `key: value` line.
fn find_key_sep(chars: &[char], from: usize) -> Option<usize> {
    let mut i = from;
    if matches!(chars.get(from), Some('\'') | Some('"')) {
        i = scan_quoted_end(chars, from)?;
        while matches!(chars.get(i), Some(' ')) {
            i += 1;
        }
        return match chars.get(i) {
            Some(':') if matches!(chars.get(i + 1), None | Some(' ')) => Some(i),
            _ => None,
        };
    }
    // A plain key cannot contain ": ", so the first one ends it.
    while let Some(&c) = chars.get(i) {
        if c == ':' && matches!(chars.get(i + 1), None | Some(' ')) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Reject the constructs this parser deliberately does not support, before any
/// attempt is made to parse them as something else.
fn check_unsupported(
    file: &Path,
    number: usize,
    chars: &[char],
    col: usize,
) -> Result<(), ParseError> {
    let feature = match chars.get(col) {
        Some('{') => Some("flow mappings ({...})"),
        Some('[') => Some("flow sequences ([...])"),
        Some('&') => Some("anchors (&name)"),
        Some('*') => Some("aliases (*name)"),
        Some('!') => Some("tags (!name)"),
        Some('|') => Some("block scalars (|)"),
        Some('>') => Some("block scalars (>)"),
        Some('?') if matches!(chars.get(col + 1), None | Some(' ')) => {
            Some("explicit keys (? key)")
        }
        Some('%') if col == 0 => Some("directives (%YAML)"),
        _ => {
            let rest = slice(chars, col, chars.len());
            if rest == "---" || rest.starts_with("--- ") {
                Some("multi-document streams (---)")
            } else if rest == "..." || rest.starts_with("... ") {
                Some("multi-document streams (...)")
            } else {
                None
            }
        }
    };
    match feature {
        Some(feature) => Err(err(
            file,
            ErrorKind::UnsupportedFeature,
            number,
            col + 1,
            format!("unsupported YAML feature: {feature}"),
        )),
        None => Ok(()),
    }
}

// --- scalars -----------------------------------------------------------------

/// Type a plain (unquoted) scalar. Quoted scalars never come through here —
/// they are always strings.
fn infer_scalar(text: &str) -> Value {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    if let Some(i) = parse_int(text) {
        return Value::Int(i);
    }
    if let Some(f) = parse_float(text) {
        return Value::Float(f);
    }
    Value::Str(text.to_string())
}

/// Optional sign then ASCII digits, with no leading zero beyond `0` itself, so
/// that codes like `007` survive as strings. Values too large for an `i64` fall
/// through to `Str` rather than being silently rounded.
fn parse_int(text: &str) -> Option<i64> {
    let digits = match text.strip_prefix('-').or_else(|| text.strip_prefix('+')) {
        Some(rest) => rest,
        None => text,
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    match text.strip_prefix('+') {
        Some(rest) => rest.parse::<i64>().ok(),
        None => text.parse::<i64>().ok(),
    }
}

/// Decimal or exponent form only — a bare digit string is an integer, and
/// anything else (`1.2.3`, `2026-08-22`) is a string.
fn parse_float(text: &str) -> Option<f64> {
    let b = text.as_bytes();
    let mut i = 0usize;
    if matches!(b.first(), Some(b'+') | Some(b'-')) {
        i = 1;
    }
    let mut digits = 0usize;
    while matches!(b.get(i), Some(c) if c.is_ascii_digit()) {
        i += 1;
        digits += 1;
    }
    let mut has_dot = false;
    if matches!(b.get(i), Some(b'.')) {
        has_dot = true;
        i += 1;
        while matches!(b.get(i), Some(c) if c.is_ascii_digit()) {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    let mut has_exp = false;
    if matches!(b.get(i), Some(b'e') | Some(b'E')) {
        has_exp = true;
        i += 1;
        if matches!(b.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let mut exp_digits = 0usize;
        while matches!(b.get(i), Some(c) if c.is_ascii_digit()) {
            i += 1;
            exp_digits += 1;
        }
        if exp_digits == 0 {
            return None;
        }
    }
    if i != b.len() || (!has_dot && !has_exp) {
        return None;
    }
    text.parse::<f64>().ok()
}

/// Decode the quoted scalar starting at `start`, returning it and the index
/// just past its closing quote.
fn decode_quoted(
    file: &Path,
    number: usize,
    chars: &[char],
    start: usize,
) -> Result<(String, usize), ParseError> {
    let quote = match chars.get(start) {
        Some(q) => *q,
        None => return Err(eof(file)),
    };
    let mut out = String::new();
    let mut i = start + 1;
    while let Some(&c) = chars.get(i) {
        if quote == '\'' {
            if c == '\'' {
                // '' is the only escape in a single-quoted scalar.
                if matches!(chars.get(i + 1), Some('\'')) {
                    out.push('\'');
                    i += 2;
                    continue;
                }
                return Ok((out, i + 1));
            }
            out.push(c);
            i += 1;
            continue;
        }
        match c {
            '"' => return Ok((out, i + 1)),
            '\\' => {
                let escape = match chars.get(i + 1) {
                    Some(e) => *e,
                    None => break,
                };
                match escape {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '0' => out.push('\0'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'u' => {
                        let (decoded, next) = decode_unicode(file, number, chars, i)?;
                        out.push(decoded);
                        i = next;
                        continue;
                    }
                    _ => {
                        return Err(err(
                            file,
                            ErrorKind::Syntax,
                            number,
                            i + 1,
                            format!(
                                "unsupported escape sequence \\{escape} in a double-quoted scalar"
                            ),
                        ));
                    }
                }
                i += 2;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Err(err(
        file,
        ErrorKind::UnterminatedQuote,
        number,
        start + 1,
        format!(
            "unterminated {} scalar: quoted scalars must close on the line they open",
            if quote == '\'' {
                "single-quoted"
            } else {
                "double-quoted"
            }
        ),
    ))
}

fn hex4(chars: &[char], at: usize) -> Option<u32> {
    let mut value = 0u32;
    for k in 0..4 {
        let digit = (*chars.get(at + k)?).to_digit(16)?;
        value = value * 16 + digit;
    }
    Some(value)
}

/// Decode `\uXXXX` at `i` (where `chars[i]` is the backslash), pairing
/// surrogates, and return the char and the index just past the escape.
fn decode_unicode(
    file: &Path,
    number: usize,
    chars: &[char],
    i: usize,
) -> Result<(char, usize), ParseError> {
    let bad = |msg: &str| {
        err(
            file,
            ErrorKind::Syntax,
            number,
            i + 1,
            format!("invalid \\u escape: {msg}"),
        )
    };
    let high = match hex4(chars, i + 2) {
        Some(v) => v,
        None => return Err(bad("expected four hexadecimal digits")),
    };
    if (0xD800..0xDC00).contains(&high) {
        // A high surrogate is only valid as the first half of a pair.
        if matches!(chars.get(i + 6), Some('\\'))
            && matches!(chars.get(i + 7), Some('u'))
            && let Some(low) = hex4(chars, i + 8)
            && (0xDC00..0xE000).contains(&low)
        {
            let cp = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            return match char::from_u32(cp) {
                Some(c) => Ok((c, i + 12)),
                None => Err(bad("not a Unicode scalar value")),
            };
        }
        return Err(bad("unpaired surrogate"));
    }
    if (0xDC00..0xE000).contains(&high) {
        return Err(bad("unpaired surrogate"));
    }
    match char::from_u32(high) {
        Some(c) => Ok((c, i + 6)),
        None => Err(bad("not a Unicode scalar value")),
    }
}

// --- recursive descent -------------------------------------------------------

/// The cursor is a line plus a column within it. Because indentation is spaces
/// only, that column *is* the effective indent of whatever starts there — which
/// is how `- name: CustomerKey` works: after the dash is consumed the cursor
/// sits at column 4, and the mapping it opens continues on later lines indented
/// to exactly column 4.
struct Parser<'a> {
    file: &'a Path,
    lines: Vec<Line>,
    idx: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    fn line_number(&self) -> usize {
        self.lines.get(self.idx).map_or(0, |l| l.number)
    }

    fn advance_line(&mut self) {
        self.idx += 1;
        self.col = self.lines.get(self.idx).map_or(0, |l| l.indent);
    }

    /// A `-` indicator: a dash followed by a space or by the end of the line.
    /// `-5` is a scalar, not a sequence item.
    fn at_dash(&self) -> bool {
        match self.lines.get(self.idx) {
            Some(l) => {
                matches!(l.chars.get(self.col), Some('-'))
                    && matches!(l.chars.get(self.col + 1), None | Some(' '))
            }
            None => false,
        }
    }

    fn reject_unsupported(&self) -> Result<(), ParseError> {
        match self.lines.get(self.idx) {
            Some(l) => check_unsupported(self.file, l.number, &l.chars, self.col),
            None => Ok(()),
        }
    }

    fn indentation_error(&self, number: usize, expected: usize) -> ParseError {
        err(
            self.file,
            ErrorKind::Indentation,
            number,
            self.col + 1,
            format!(
                "inconsistent indentation: this line is indented {} spaces, but its block is indented {expected}",
                self.col
            ),
        )
    }

    /// Parse whatever block starts at the cursor, at column `indent`.
    fn parse_block(&mut self, indent: usize) -> Result<Node, ParseError> {
        self.reject_unsupported()?;
        if self.at_dash() {
            return self.parse_seq(indent);
        }
        let is_mapping = match self.lines.get(self.idx) {
            Some(l) => find_key_sep(&l.chars, self.col).is_some(),
            None => return Err(eof(self.file)),
        };
        if is_mapping {
            self.parse_map(indent)
        } else {
            let node = self.parse_inline_value()?;
            self.advance_line();
            Ok(node)
        }
    }

    fn parse_map(&mut self, indent: usize) -> Result<Node, ParseError> {
        let start_line = self.line_number();
        let mut entries: Vec<(Key, Node)> = Vec::new();
        loop {
            let number = match self.lines.get(self.idx) {
                None => break,
                Some(l) => {
                    if self.col < indent {
                        break;
                    }
                    if self.col > indent {
                        return Err(self.indentation_error(l.number, indent));
                    }
                    l.number
                }
            };
            let before = (self.idx, self.col);
            self.reject_unsupported()?;
            if self.at_dash() {
                return Err(err(
                    self.file,
                    ErrorKind::Syntax,
                    number,
                    self.col + 1,
                    "expected a mapping key, found a sequence item".to_string(),
                ));
            }
            let key = self.parse_key()?;
            if let Some((first, _)) = entries.iter().find(|(k, _)| k.name == key.name) {
                return Err(err(
                    self.file,
                    ErrorKind::DuplicateKey,
                    key.line,
                    key.col,
                    format!(
                        "duplicate mapping key '{}': first defined on line {}",
                        key.name, first.line
                    ),
                ));
            }
            let value = self.parse_key_value(&key, indent)?;
            entries.push((key, value));
            if (self.idx, self.col) == before {
                return Err(err(
                    self.file,
                    ErrorKind::Syntax,
                    number,
                    self.col + 1,
                    "could not make progress parsing this mapping".to_string(),
                ));
            }
        }
        Ok(Node {
            line: start_line,
            value: Value::Map(entries),
        })
    }

    /// The value of a mapping entry: the rest of the key's line if there is
    /// one, otherwise the block indented under it, otherwise null.
    fn parse_key_value(&mut self, key: &Key, indent: usize) -> Result<Node, ParseError> {
        let has_inline = match self.lines.get(self.idx) {
            Some(l) => self.col < l.chars.len(),
            None => false,
        };
        if has_inline {
            let node = self.parse_inline_value()?;
            self.advance_line();
            return Ok(node);
        }
        self.advance_line();
        match self.lines.get(self.idx) {
            // A block indented under the key.
            Some(_) if self.col > indent => self.parse_block(self.col),
            // A block sequence may also sit at its key's own indent — common
            // YAML style, and unambiguous, since a mapping entry can never have
            // a sequence item as a sibling.
            Some(_) if self.col == indent && self.at_dash() => self.parse_block(self.col),
            _ => Ok(Node {
                line: key.line,
                value: Value::Null,
            }),
        }
    }

    /// Read the key at the cursor and leave the cursor on its value.
    fn parse_key(&mut self) -> Result<Key, ParseError> {
        let file = self.file;
        let col = self.col;
        let (name, number, next) = {
            let line = match self.lines.get(self.idx) {
                Some(l) => l,
                None => return Err(eof(file)),
            };
            let chars = &line.chars;
            let sep = match find_key_sep(chars, col) {
                Some(sep) => sep,
                None => {
                    return Err(err(
                        file,
                        ErrorKind::Syntax,
                        line.number,
                        col + 1,
                        "expected ':' after a mapping key".to_string(),
                    ));
                }
            };
            let name = if matches!(chars.get(col), Some('\'') | Some('"')) {
                decode_quoted(file, line.number, chars, col)?.0
            } else {
                slice(chars, col, sep).trim_end().to_string()
            };
            if name.is_empty() {
                return Err(err(
                    file,
                    ErrorKind::Syntax,
                    line.number,
                    col + 1,
                    "mapping key is empty".to_string(),
                ));
            }
            let mut next = sep + 1;
            while matches!(chars.get(next), Some(' ')) {
                next += 1;
            }
            (name, line.number, next)
        };
        self.col = next;
        Ok(Key {
            name,
            line: number,
            col: col + 1,
        })
    }

    /// A scalar occupying the rest of the current line. The cursor is left
    /// where it was; the caller advances.
    fn parse_inline_value(&mut self) -> Result<Node, ParseError> {
        let file = self.file;
        let col = self.col;
        let line = match self.lines.get(self.idx) {
            Some(l) => l,
            None => return Err(eof(file)),
        };
        let number = line.number;
        let chars = &line.chars;
        check_unsupported(file, number, chars, col)?;
        if matches!(chars.get(col), Some('-')) && matches!(chars.get(col + 1), None | Some(' ')) {
            return Err(err(
                file,
                ErrorKind::Syntax,
                number,
                col + 1,
                "a block sequence cannot start on the same line as its key".to_string(),
            ));
        }
        let value = if matches!(chars.get(col), Some('\'') | Some('"')) {
            let (text, end) = decode_quoted(file, number, chars, col)?;
            let mut after = end;
            while matches!(chars.get(after), Some(' ')) {
                after += 1;
            }
            if after < chars.len() {
                return Err(err(
                    file,
                    ErrorKind::Syntax,
                    number,
                    after + 1,
                    "unexpected text after a quoted scalar".to_string(),
                ));
            }
            // Quoted scalars are never type-inferred.
            Value::Str(text)
        } else {
            infer_scalar(slice(chars, col, chars.len()).trim_end())
        };
        Ok(Node {
            line: number,
            value,
        })
    }

    fn parse_seq(&mut self, indent: usize) -> Result<Node, ParseError> {
        let start_line = self.line_number();
        let mut items: Vec<Node> = Vec::new();
        loop {
            let number = match self.lines.get(self.idx) {
                None => break,
                Some(l) => {
                    if self.col < indent {
                        break;
                    }
                    if self.col > indent {
                        return Err(self.indentation_error(l.number, indent));
                    }
                    l.number
                }
            };
            let before = (self.idx, self.col);
            self.reject_unsupported()?;
            // Anything else at this indent belongs to the enclosing block.
            if !self.at_dash() {
                break;
            }
            let dash_col = self.col;
            self.col += 1;
            while matches!(
                self.lines.get(self.idx).and_then(|l| l.chars.get(self.col)),
                Some(' ')
            ) {
                self.col += 1;
            }
            let at_end = self
                .lines
                .get(self.idx)
                .is_none_or(|l| self.col >= l.chars.len());
            let node = if at_end {
                // A bare `-`: the item is the block indented under the dash.
                self.advance_line();
                match self.lines.get(self.idx) {
                    Some(_) if self.col > dash_col => self.parse_block(self.col)?,
                    _ => Node {
                        line: number,
                        value: Value::Null,
                    },
                }
            } else {
                self.parse_block(self.col)?
            };
            items.push(node);
            if (self.idx, self.col) == before {
                return Err(err(
                    self.file,
                    ErrorKind::Syntax,
                    number,
                    self.col + 1,
                    "could not make progress parsing this sequence".to_string(),
                ));
            }
        }
        Ok(Node {
            line: start_line,
            value: Value::Seq(items),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The MOD-01 example, verbatim, shared with every other story's fixtures.
    const FIXTURE: &str = include_str!("../tests/fixtures/catalog/dim_customer.yml");

    fn parse(text: &str) -> Result<Node, ParseError> {
        parse_str(text, Path::new("test.yml"))
    }

    fn ok(text: &str) -> Node {
        match parse(text) {
            Ok(node) => node,
            Err(e) => panic!("expected {text:?} to parse, got: {e}"),
        }
    }

    fn bad(text: &str) -> ParseError {
        match parse(text) {
            Ok(node) => panic!("expected {text:?} to fail, got: {node:?}"),
            Err(e) => e,
        }
    }

    /// Compare two trees ignoring every line and column, so that a commented
    /// document can be checked against an uncommented one.
    fn same_shape(a: &Node, b: &Node) -> bool {
        match (&a.value, &b.value) {
            (Value::Map(x), Value::Map(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y.iter())
                        .all(|((ka, va), (kb, vb))| ka.name == kb.name && same_shape(va, vb))
            }
            (Value::Seq(x), Value::Seq(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(m, n)| same_shape(m, n))
            }
            (x, y) => x == y,
        }
    }

    // --- AC1/AC3: the tree, and a line on every node -------------------------

    #[test]
    fn fixture_parses_with_the_source_line_on_every_node() {
        let doc = ok(FIXTURE);
        assert_eq!(doc.type_name(), "mapping");
        assert_eq!(doc.line, 2);

        for (key, text, line, col) in [
            ("layer", "dwh", 2, 1),
            ("namespace", "core", 3, 1),
            ("table", "DimCustomer", 4, 1),
            ("description", "Conformed customer dimension", 5, 1),
            ("owner", "data-platform", 6, 1),
        ] {
            let node = match doc.get(key) {
                Some(node) => node,
                None => panic!("missing key {key}"),
            };
            assert_eq!(node.as_str(), Some(text), "{key}");
            assert_eq!(node.line, line, "{key} value line");
            assert_eq!(
                doc.get_key(key).map(|k| (k.line, k.col)),
                Some((line, col)),
                "{key} key"
            );
        }

        assert_eq!(
            doc.get_key("columns").map(|k| (k.line, k.col)),
            Some((7, 1))
        );
        let columns = match doc.get("columns") {
            Some(node) => {
                assert_eq!(node.line, 8, "the sequence starts on its first item's line");
                match node.as_seq() {
                    Some(items) => items,
                    None => panic!("columns is not a sequence"),
                }
            }
            None => panic!("missing columns"),
        };
        assert_eq!(columns.len(), 2);

        // Two levels deep: the item's own line, and each of its keys' lines.
        assert_eq!(columns[0].line, 8);
        assert_eq!(
            columns[0].get("name").map(|n| (n.as_str(), n.line)),
            Some((Some("CustomerKey"), 8))
        );
        assert_eq!(
            columns[0].get("type").map(|n| (n.as_str(), n.line)),
            Some((Some("int"), 9))
        );
        assert_eq!(
            columns[0].get("description").map(|n| (n.as_str(), n.line)),
            Some((Some("Surrogate key"), 10))
        );
        assert_eq!(
            columns[0].get_key("name").map(|k| (k.line, k.col)),
            Some((8, 5))
        );
        assert_eq!(
            columns[0].get_key("type").map(|k| (k.line, k.col)),
            Some((9, 5))
        );
        assert_eq!(
            columns[0].get_key("description").map(|k| (k.line, k.col)),
            Some((10, 5))
        );

        assert_eq!(columns[1].line, 11);
        assert_eq!(
            columns[1].get("name").map(|n| (n.as_str(), n.line)),
            Some((Some("FullName"), 11))
        );
        assert_eq!(
            columns[1].get("type").map(|n| (n.as_str(), n.line)),
            Some((Some("nvarchar(200)"), 12))
        );
        assert_eq!(columns[1].get("description"), None);
    }

    #[test]
    fn parse_file_reads_the_fixture_from_disk() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/catalog/dim_customer.yml"
        ));
        let from_disk = match parse_file(path) {
            Ok(node) => node,
            Err(e) => panic!("fixture did not parse: {e}"),
        };
        assert!(same_shape(&from_disk, &ok(FIXTURE)));
        assert_eq!(
            from_disk
                .get("columns")
                .and_then(|n| n.as_seq())
                .map(|s| s.len()),
            Some(2)
        );
    }

    #[test]
    fn parse_file_reports_a_missing_file_without_panicking() {
        let e = match parse_file(Path::new("no/such/file.yml")) {
            Ok(node) => panic!("expected a failure, got {node:?}"),
            Err(e) => e,
        };
        assert_eq!(e.kind, ErrorKind::Io);
        assert!(
            e.to_string()
                .starts_with("no/such/file.yml: error: could not read file"),
            "{e}"
        );
    }

    // --- AC4: comments ------------------------------------------------------

    /// The fixture again, with a header block, an inline comment on a key, an
    /// indented comment inside the sequence, and blank lines.
    const COMMENTED: &str = "\
# catalog/warehouse/dim_customer.yml
#
# The conformed customer dimension.

layer: dwh              # the storage layer
namespace: core
table: DimCustomer
description: Conformed customer dimension
owner: data-platform

# The columns follow.
columns:
    # the surrogate key
  - name: CustomerKey
    type: int
    description: Surrogate key
  - name: FullName    # the display name
    type: nvarchar(200)
";

    #[test]
    fn comments_and_blank_lines_change_neither_the_tree_nor_the_numbering() {
        let doc = ok(COMMENTED);
        assert!(same_shape(&doc, &ok(FIXTURE)), "comments changed the tree");

        // Line numbers are the physical lines of *this* document: dropping the
        // comment and blank lines must not renumber anything after them.
        assert_eq!(
            doc.get("layer").map(|n| (n.as_str(), n.line)),
            Some((Some("dwh"), 5))
        );
        assert_eq!(doc.get("owner").map(|n| n.line), Some(9));
        assert_eq!(doc.get_key("columns").map(|k| k.line), Some(12));
        let columns = match doc.get("columns").and_then(|n| n.as_seq()) {
            Some(items) => items,
            None => panic!("columns is not a sequence"),
        };
        assert_eq!(columns[0].line, 14);
        assert_eq!(columns[0].get("type").map(|n| n.line), Some(15));
        assert_eq!(columns[0].get("description").map(|n| n.line), Some(16));
        assert_eq!(columns[1].line, 17);
        assert_eq!(
            columns[1].get("name").and_then(|n| n.as_str()),
            Some("FullName")
        );
        assert_eq!(columns[1].get("type").map(|n| n.line), Some(18));
    }

    #[test]
    fn a_hash_only_opens_a_comment_where_yaml_says_it_does() {
        for (src, expected) in [
            ("k: a # note\n", "a"),
            ("k: a#b\n", "a#b"),
            ("k: don't # note\n", "don't"),
            ("k: \"a # b\"\n", "a # b"),
            ("k: 'a # b' # note\n", "a # b"),
        ] {
            assert_eq!(
                ok(src).get("k").and_then(|n| n.as_str()),
                Some(expected),
                "{src:?}"
            );
        }
    }

    // --- AC2: unsupported constructs ----------------------------------------

    #[test]
    fn unsupported_constructs_are_rejected_by_name() {
        for (src, feature) in [
            ("a: {b: 1}\n", "flow mapping"),
            ("a: [1, 2]\n", "flow sequence"),
            ("{a: 1}\n", "flow mapping"),
            ("- [1]\n", "flow sequence"),
            ("a: &anchor 1\n", "anchor"),
            ("a: *anchor\n", "alias"),
            ("a: !!str 1\n", "tag"),
            ("---\na: 1\n", "multi-document"),
            ("a: 1\n...\n", "multi-document"),
            ("a: |\n  text\n", "block scalar"),
            ("a: >\n  text\n", "block scalar"),
            ("? a\n: 1\n", "explicit key"),
            ("%YAML 1.2\na: 1\n", "directive"),
        ] {
            let e = bad(src);
            assert_eq!(e.kind, ErrorKind::UnsupportedFeature, "{src:?}");
            assert!(e.message.contains(feature), "{src:?} said: {}", e.message);
            assert!(
                e.message.contains("unsupported YAML feature"),
                "{src:?}: {}",
                e.message
            );
        }
    }

    #[test]
    fn an_unsupported_construct_names_its_line() {
        let e = bad("a: 1\nb: 2\nc: [3]\n");
        assert_eq!(e.kind, ErrorKind::UnsupportedFeature);
        assert_eq!(e.line, 3);
        assert_eq!(
            e.to_string(),
            "test.yml:3: error: unsupported YAML feature: flow sequences ([...])"
        );
    }

    #[test]
    fn a_quoted_scalar_that_only_looks_like_flow_style_is_fine() {
        assert_eq!(
            ok("a: \"{b: 1}\"\n").get("a").and_then(|n| n.as_str()),
            Some("{b: 1}")
        );
    }

    // --- AC6: malformed input -----------------------------------------------

    #[test]
    fn a_tab_in_indentation_is_rejected() {
        let e = bad("a:\n\tb: 1\n");
        assert_eq!(e.kind, ErrorKind::Indentation);
        assert_eq!(e.line, 2);
        assert!(e.message.contains("tab"), "{}", e.message);
        // A tab *after* the indentation is just a character.
        assert_eq!(
            ok("a: x\ty\n").get("a").and_then(|n| n.as_str()),
            Some("x\ty")
        );
    }

    #[test]
    fn siblings_at_inconsistent_indentation_are_rejected() {
        for (src, line) in [
            ("a:\n  b: 1\n   c: 2\n", 3),
            ("a:\n    b: 1\n  c: 2\n", 3),
            ("a: 1\n  b: 2\n", 2),
            ("- 1\n  - 2\n", 2),
        ] {
            let e = bad(src);
            assert_eq!(
                e.kind,
                ErrorKind::Indentation,
                "{src:?} said: {}",
                e.message
            );
            assert_eq!(e.line, line, "{src:?} said: {}", e.message);
        }
    }

    #[test]
    fn unterminated_quotes_are_rejected() {
        for src in [
            "a: 'oops\n",
            "a: \"oops\n",
            "a: 'multi\n  line'\n",
            "'key: 1\n",
        ] {
            let e = bad(src);
            assert_eq!(
                e.kind,
                ErrorKind::UnterminatedQuote,
                "{src:?} said: {}",
                e.message
            );
            assert_eq!(e.line, 1, "{src:?}");
        }
    }

    #[test]
    fn duplicate_keys_name_the_file_and_both_lines() {
        let e = bad("name: a\ntype: int\nname: b\n");
        assert_eq!(e.kind, ErrorKind::DuplicateKey);
        assert_eq!(e.line, 3);
        assert_eq!(
            e.to_string(),
            "test.yml:3: error: duplicate mapping key 'name': first defined on line 1"
        );

        // The same check MOD-01 AC4 needs: a column declared twice in one file.
        let e = bad("columns:\n  - name: a\n    type: int\n    name: b\n");
        assert_eq!(e.kind, ErrorKind::DuplicateKey);
        assert_eq!(e.line, 4);
        assert!(e.message.contains("line 2"), "{}", e.message);

        // Duplicates are per mapping, not per file.
        assert!(parse("columns:\n  - name: a\n  - name: b\n").is_ok());
    }

    #[test]
    fn other_malformed_input_is_rejected_clearly() {
        for (src, kind, line) in [
            ("a: 1\nb\n", ErrorKind::Syntax, 2),
            (": 1\n", ErrorKind::Syntax, 1),
            ("a: 'x' junk\n", ErrorKind::Syntax, 1),
            ("a: \"\\q\"\n", ErrorKind::Syntax, 1),
            ("a: \"\\uZZZZ\"\n", ErrorKind::Syntax, 1),
            ("a: \"\\ud800\"\n", ErrorKind::Syntax, 1),
            ("a: - 1\n", ErrorKind::Syntax, 1),
            ("a: 1\n- 2\n", ErrorKind::Syntax, 2),
        ] {
            let e = bad(src);
            assert_eq!(e.kind, kind, "{src:?} said: {}", e.message);
            assert_eq!(e.line, line, "{src:?} said: {}", e.message);
            assert!(
                e.to_string()
                    .starts_with(&format!("test.yml:{line}: error: ")),
                "{e}"
            );
        }
    }

    #[test]
    fn malformed_input_never_panics() {
        // Every prefix of the fixture, which lands mid-token in every position.
        let chars: Vec<char> = FIXTURE.chars().collect();
        for n in 0..chars.len() {
            let text: String = chars[..n].iter().collect();
            let _ = parse(&text);
        }
        for src in [
            "\"",
            "'",
            ":",
            "-",
            "- -",
            "a:\n\n  \n",
            "\u{feff}",
            "\\",
            "a: \"\\u\"",
            "a: \"\\ud800\\u0041\"",
            "  \n\ta: 1\n",
            "a: '''",
            "- : -",
            "??",
            "k: \u{1f600} # e",
            "\u{1f600}: 1",
        ] {
            let _ = parse(src);
        }
    }

    // --- AC5: scalar typing --------------------------------------------------

    #[test]
    fn plain_scalars_are_typed() {
        let cases: &[(&str, Value)] = &[
            // The values that must survive as strings.
            ("nvarchar(200)", Value::Str("nvarchar(200)".to_string())),
            ("2026-08-22", Value::Str("2026-08-22".to_string())),
            ("1.2.3", Value::Str("1.2.3".to_string())),
            ("007", Value::Str("007".to_string())),
            ("data-platform", Value::Str("data-platform".to_string())),
            (
                "99999999999999999999",
                Value::Str("99999999999999999999".to_string()),
            ),
            // Deliberately not booleans: the Norway problem.
            ("no", Value::Str("no".to_string())),
            ("yes", Value::Str("yes".to_string())),
            ("on", Value::Str("on".to_string())),
            ("off", Value::Str("off".to_string())),
            // Booleans.
            ("true", Value::Bool(true)),
            ("True", Value::Bool(true)),
            ("TRUE", Value::Bool(true)),
            ("false", Value::Bool(false)),
            ("False", Value::Bool(false)),
            ("FALSE", Value::Bool(false)),
            // Integers.
            ("0", Value::Int(0)),
            ("42", Value::Int(42)),
            ("-5", Value::Int(-5)),
            ("+7", Value::Int(7)),
            // Floats.
            ("2.75", Value::Float(2.75)),
            ("-0.5", Value::Float(-0.5)),
            ("1e9", Value::Float(1e9)),
            ("-2.5e-3", Value::Float(-2.5e-3)),
            // Null.
            ("~", Value::Null),
            ("null", Value::Null),
            ("Null", Value::Null),
            ("NULL", Value::Null),
        ];
        for (text, expected) in cases {
            let doc = ok(&format!("k: {text}\n"));
            assert_eq!(doc.get("k").map(|n| &n.value), Some(expected), "{text}");
        }
    }

    #[test]
    fn quoted_scalars_are_always_strings() {
        for (src, expected) in [
            ("k: 'true'\n", "true"),
            ("k: \"42\"\n", "42"),
            ("k: '007'\n", "007"),
            ("k: \"~\"\n", "~"),
            ("k: \"\"\n", ""),
            ("k: ''\n", ""),
            ("k: 'it''s'\n", "it's"),
        ] {
            assert_eq!(
                ok(src).get("k").and_then(|n| n.as_str()),
                Some(expected),
                "{src:?}"
            );
        }
    }

    #[test]
    fn double_quoted_escapes_are_decoded() {
        let doc = ok(concat!(r#"k: "a\nb\tc\"d\\e\/f\u00e9\u0041""#, "\n"));
        assert_eq!(
            doc.get("k").and_then(|n| n.as_str()),
            Some("a\nb\tc\"d\\e/féA")
        );
        let doc = ok(concat!(r#"k: "\ud83d\ude00""#, "\n"));
        assert_eq!(doc.get("k").and_then(|n| n.as_str()), Some("\u{1f600}"));
    }

    // --- edges ---------------------------------------------------------------

    #[test]
    fn empty_and_comment_only_documents_are_null() {
        for src in [
            "",
            "\n",
            "\n\n   \n",
            "# just a comment\n",
            "# one\n\n  # two\n",
        ] {
            let doc = ok(src);
            assert_eq!(doc.value, Value::Null, "{src:?}");
            assert_eq!(doc.line, 1, "{src:?}");
        }
    }

    #[test]
    fn a_key_with_no_value_is_null_at_its_own_line() {
        let doc = ok("layer: dwh\ntable:\ncolumns:\n");
        assert_eq!(
            doc.get("table").map(|n| (&n.value, n.line)),
            Some((&Value::Null, 2))
        );
        assert_eq!(
            doc.get("columns").map(|n| (&n.value, n.line)),
            Some((&Value::Null, 3))
        );
        // Present-with-no-value is not the same as missing.
        assert!(doc.get_key("table").is_some());
        assert!(doc.get_key("namespace").is_none());
        assert!(doc.get("namespace").is_none());
    }

    #[test]
    fn crlf_bom_and_trailing_whitespace_are_handled() {
        let doc = ok("\u{feff}layer: dwh\r\ncount: 1   \r\ntable: DimCustomer\r\n");
        assert_eq!(doc.get("layer").and_then(|n| n.as_str()), Some("dwh"));
        assert_eq!(doc.get("count").map(|n| &n.value), Some(&Value::Int(1)));
        assert_eq!(doc.get("table").map(|n| n.line), Some(3));
        // The BOM must not shift the first key's column.
        assert_eq!(doc.get_key("layer").map(|k| (k.line, k.col)), Some((1, 1)));
    }

    #[test]
    fn quoted_keys_are_decoded() {
        let doc = ok("'a b': 1\n\"c:d\": 2\n'it''s' : 3\n");
        assert_eq!(doc.get("a b").map(|n| &n.value), Some(&Value::Int(1)));
        assert_eq!(doc.get("c:d").map(|n| &n.value), Some(&Value::Int(2)));
        assert_eq!(doc.get("it's").map(|n| &n.value), Some(&Value::Int(3)));
        assert_eq!(doc.get_key("a b").map(|k| (k.line, k.col)), Some((1, 1)));
    }

    #[test]
    fn a_block_sequence_may_sit_at_its_key_indent() {
        let doc = ok("columns:\n- name: a\n- name: b\nowner: x\n");
        let columns = match doc.get("columns").and_then(|n| n.as_seq()) {
            Some(items) => items,
            None => panic!("columns is not a sequence"),
        };
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[1].get("name").and_then(|n| n.as_str()), Some("b"));
        assert_eq!(columns[1].line, 3);
        assert_eq!(doc.get("owner").and_then(|n| n.as_str()), Some("x"));
        assert_eq!(doc.get_key("owner").map(|k| k.line), Some(4));
    }

    #[test]
    fn sequences_of_mappings_of_sequences_nest() {
        let doc = ok("a:\n  - b:\n      - 1\n      - 2\n  - c: x\n");
        let outer = match doc.get("a").and_then(|n| n.as_seq()) {
            Some(items) => items,
            None => panic!("a is not a sequence"),
        };
        assert_eq!(outer.len(), 2);
        let inner = match outer[0].get("b").and_then(|n| n.as_seq()) {
            Some(items) => items,
            None => panic!("b is not a sequence"),
        };
        assert_eq!(
            inner
                .iter()
                .map(|n| (n.value.clone(), n.line))
                .collect::<Vec<_>>(),
            vec![(Value::Int(1), 3), (Value::Int(2), 4)]
        );
        assert_eq!(outer[1].get("c").and_then(|n| n.as_str()), Some("x"));
        assert_eq!(outer[1].line, 5);
    }

    #[test]
    fn nested_and_empty_sequence_items() {
        let doc = ok("- - 1\n  - 2\n");
        let outer = match doc.as_seq() {
            Some(items) => items,
            None => panic!("not a sequence"),
        };
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].as_seq().map(|s| s.len()), Some(2));

        let doc = ok("-\n- a\n");
        let items = match doc.as_seq() {
            Some(items) => items,
            None => panic!("not a sequence"),
        };
        assert_eq!(items[0].value, Value::Null);
        assert_eq!(items[0].line, 1);
        assert_eq!(items[1].as_str(), Some("a"));

        let doc = ok("-\n    a: 1\n");
        let items = match doc.as_seq() {
            Some(items) => items,
            None => panic!("not a sequence"),
        };
        assert_eq!(items[0].get("a").map(|n| &n.value), Some(&Value::Int(1)));
    }

    #[test]
    fn accessors_report_the_node_type() {
        let doc = ok("m:\n  a: 1\ns:\n  - 1\nn: ~\nb: true\ni: 1\nf: 1.5\nt: text\n");
        for (key, name) in [
            ("m", "mapping"),
            ("s", "sequence"),
            ("n", "null"),
            ("b", "boolean"),
            ("i", "integer"),
            ("f", "float"),
            ("t", "string"),
        ] {
            assert_eq!(doc.get(key).map(|n| n.type_name()), Some(name), "{key}");
        }
        assert_eq!(doc.get("b").and_then(|n| n.as_bool()), Some(true));
        assert_eq!(doc.get("i").and_then(|n| n.as_int()), Some(1));
        assert_eq!(doc.get("f").and_then(|n| n.as_float()), Some(1.5));
        assert_eq!(doc.get("t").and_then(|n| n.as_str()), Some("text"));
        assert_eq!(
            doc.get("m").and_then(|n| n.as_map()).map(|m| m.len()),
            Some(1)
        );
        assert!(doc.get("n").map(|n| n.is_null()).unwrap_or(false));
        // Wrong-type accessors return None rather than panicking.
        assert_eq!(doc.get("m").and_then(|n| n.as_str()), None);
        assert_eq!(doc.get("t").and_then(|n| n.as_seq()), None);
        assert_eq!(doc.get("t").and_then(|n| n.get("x")), None);
    }

    #[test]
    fn a_document_may_be_a_bare_scalar_or_sequence() {
        assert_eq!(ok("just text\n").as_str(), Some("just text"));
        assert_eq!(ok("- 1\n- 2\n").as_seq().map(|s| s.len()), Some(2));
        // A value on the line after its key is still that key's value.
        assert_eq!(
            ok("a:\n  text\n").get("a").and_then(|n| n.as_str()),
            Some("text")
        );
    }
}
