use memchr::memmem;
use std::cell::RefCell;
use std::io;
use std::ops::AddAssign;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

thread_local! {
    static CHUNK_BUF: RefCell<[u8; CHUNK_SIZE]> = const { RefCell::new([0u8; CHUNK_SIZE]) };
    static CARRY_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone)]
pub struct StringDelimiter {
    pub open: &'static [u8],
    pub close: &'static [u8],
    pub backslash_escapes: bool,
}

#[derive(Debug, Clone)]
pub struct LangSyntax {
    pub line_comment: Option<&'static [u8]>,
    pub block_comment: Option<(&'static [u8], &'static [u8])>,
    pub single_line_strings: &'static [StringDelimiter],
    pub multiline_strings: &'static [StringDelimiter],
    pub docstring_delimiters: &'static [StringDelimiter],
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub lines: usize,
    pub blanks: usize,
    pub comments: usize,
    pub code: usize,
}

/// Check whether `pos` in `line` is preceded by an odd number of backslashes
/// (i.e. the character at `pos` is escaped).
fn is_escaped(line: &[u8], pos: usize) -> bool {
    line[..pos]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

/// Find the first unescaped occurrence of `close` in `line[start..]`.
/// Returns the absolute position within `line`, or `None`.
fn find_close(line: &[u8], start: usize, close: &[u8], backslash_escapes: bool) -> Option<usize> {
    let mut pos = start;
    while let Some(offset) = memmem::find(&line[pos..], close) {
        let abs = pos + offset;
        if backslash_escapes && is_escaped(line, abs) {
            pos = abs + close.len();
            continue;
        }
        return Some(abs);
    }
    None
}

/// Find the earliest string delimiter opener in `haystack`.
fn find_earliest_opener<'a>(
    haystack: &[u8],
    delimiters: &'a [StringDelimiter],
) -> Option<(usize, &'a StringDelimiter)> {
    delimiters
        .iter()
        .filter_map(|d| memmem::find(haystack, d.open).map(|off| (off, d)))
        .min_by_key(|(off, _)| *off)
}

/// A multiline construct found by `find_next_event`.
enum Event<'a> {
    MultilineString {
        pos: usize,
        delim: &'a StringDelimiter,
    },
    BlockComment {
        pos: usize,
    },
}

/// Scan `seg` for the earliest multiline string opener or block comment opener
/// that is not inside a single-line string. Uses a single memchr pass to find
/// the first "trigger byte" (first byte of any opener), then verifies the full
/// match at that position.
fn find_next_event<'a>(
    seg: &[u8],
    syntax: &'a LangSyntax,
    triggers: &[u8; 4],
    n_triggers: usize,
) -> Option<Event<'a>> {
    if n_triggers == 0 {
        return None;
    }

    let mut pos = 0;
    loop {
        let remaining = &seg[pos..];

        let off = match n_triggers {
            1 => memchr::memchr(triggers[0], remaining)?,
            2 => memchr::memchr2(triggers[0], triggers[1], remaining)?,
            3 => memchr::memchr3(triggers[0], triggers[1], triggers[2], remaining)?,
            _ => {
                // fall back to individual scans.
                let mut min_off = usize::MAX;
                if let Some((s, _)) = syntax.block_comment
                    && let Some(o) = memmem::find(remaining, s)
                    && o < min_off
                {
                    min_off = o;
                }
                if let Some((o, _)) = find_earliest_opener(remaining, syntax.multiline_strings)
                    && o < min_off
                {
                    min_off = o;
                }
                if min_off == usize::MAX {
                    return None;
                }
                min_off
            }
        };

        let candidate = pos + off;
        let tail = &seg[candidate..];

        let mut event = None;
        for delim in syntax.multiline_strings {
            if tail.starts_with(delim.open) {
                event = Some(Event::MultilineString {
                    pos: candidate,
                    delim,
                });
                break;
            }
        }
        if event.is_none()
            && let Some((block_start, _)) = syntax.block_comment
            && tail.starts_with(block_start)
        {
            event = Some(Event::BlockComment { pos: candidate });
        }

        let Some(event) = event else {
            // false trigger match; advance past it
            pos = candidate + 1;
            continue;
        };

        if let Some(skip_to) = string_enclosing(seg, pos, candidate, syntax.single_line_strings) {
            // ignore this opener since it's inside of a string literal
            pos = skip_to;
            continue;
        }

        return Some(event);
    }
}

/// If the position `target` in `seg` falls inside a single-line string that
/// opens at or after `start`, returns `Some(end)` where `end` is past the
/// closing delimiter of that string. Otherwise returns `None`.
fn string_enclosing(
    seg: &[u8],
    start: usize,
    target: usize,
    delimiters: &[StringDelimiter],
) -> Option<usize> {
    let mut i = start;
    while i <= target {
        // We use seg[i..target] (exclusive) so openers starting at exactly
        // the target position are not considered — the multiline/block-comment
        // match at that position takes precedence.
        let (earliest_off, delim) = find_earliest_opener(&seg[i..target], delimiters)?;
        let open_abs = i + earliest_off;
        let after_open = open_abs + delim.open.len();
        match find_close(seg, after_open, delim.close, delim.backslash_escapes) {
            Some(close_pos) => {
                let end = close_pos + delim.close.len();
                if target < end {
                    // target is inside this string
                    return Some(end);
                }
                i = end;
            }
            // Unclosed string — target is inside it
            None => return Some(seg.len()),
        }
    }
    None
}

#[derive(Clone, Copy)]
enum State<'a> {
    Normal,
    InBlockComment,
    InMultilineString {
        delim: &'a StringDelimiter,
        is_docstring: bool,
    },
}

enum LineClass {
    Blank,
    Comment,
    Code,
}

struct Counter<'a> {
    state: State<'a>,
    syntax: &'a LangSyntax,
    triggers: [u8; 4],
    n_triggers: usize,
}

impl<'a> Counter<'a> {
    fn new(syntax: &'a LangSyntax) -> Self {
        let mut triggers = [0u8; 4];
        let mut n_triggers = 0usize;
        if let Some((block_start, _)) = syntax.block_comment {
            triggers[0] = block_start[0];
            n_triggers = 1;
        }
        for delim in syntax.multiline_strings {
            let b = delim.open[0];
            if !triggers[..n_triggers].contains(&b) {
                triggers[n_triggers] = b;
                n_triggers += 1;
            }
        }
        Counter {
            state: State::Normal,
            syntax,
            triggers,
            n_triggers,
        }
    }

    /// Classify one line and advance the state machine.
    fn process_line(&mut self, line_bytes: &[u8], counts: &mut Counts) {
        counts.lines += 1;
        match self.classify_and_advance(line_bytes) {
            LineClass::Blank => counts.blanks += 1,
            LineClass::Comment => counts.comments += 1,
            LineClass::Code => counts.code += 1,
        }
    }

    /// Classify the line and update state for the next line.
    fn classify_and_advance(&mut self, line_bytes: &[u8]) -> LineClass {
        if line_bytes.trim_ascii_start().is_empty() {
            return LineClass::Blank;
        }

        let mut has_code = false;
        let mut has_comment = false;
        let mut cursor = 0;
        let mut at_line_start = true;

        loop {
            match self.state {
                State::InBlockComment => {
                    has_comment = true;
                    let (_, block_end) = self.syntax.block_comment.unwrap();
                    match memmem::find(&line_bytes[cursor..], block_end) {
                        None => break,
                        Some(off) => {
                            cursor += off + block_end.len();
                            self.state = State::Normal;
                            at_line_start = false;
                        }
                    }
                }
                State::InMultilineString {
                    delim,
                    is_docstring,
                } => {
                    if is_docstring {
                        has_comment = true;
                    } else {
                        has_code = true;
                    }
                    match find_close(line_bytes, cursor, delim.close, delim.backslash_escapes) {
                        None => break,
                        Some(close_pos) => {
                            cursor = close_pos + delim.close.len();
                            self.state = State::Normal;
                            at_line_start = false;
                        }
                    }
                }
                State::Normal => {
                    let remaining = &line_bytes[cursor..];
                    let trimmed = remaining.trim_ascii_start();
                    if trimmed.is_empty() {
                        break;
                    }

                    // Block comment at start of line. Must precede the line comment check
                    // since in some languages (Lua, Julia) the line comment marker is a
                    // prefix of the block comment start marker.
                    if let Some((block_start, block_end)) = self.syntax.block_comment
                        && trimmed.starts_with(block_start)
                    {
                        has_comment = true;
                        at_line_start = false;
                        let after_open =
                            cursor + (remaining.len() - trimmed.len()) + block_start.len();
                        match memmem::find(&line_bytes[after_open..], block_end) {
                            None => {
                                self.state = State::InBlockComment;
                                break;
                            }
                            Some(off) => {
                                cursor = after_open + off + block_end.len();
                                continue;
                            }
                        }
                    }

                    // Check if line starts with a line comment
                    if let Some(prefix) = self.syntax.line_comment
                        && trimmed.starts_with(prefix)
                    {
                        has_comment = true;
                        break; // line comment consumes the rest of the line
                    }

                    // Check if it's a docstring (multiline string opener as first content on line)
                    if at_line_start {
                        if let Some(delim) = self
                            .syntax
                            .docstring_delimiters
                            .iter()
                            .find(|d| trimmed.starts_with(d.open))
                        {
                            let after_open =
                                cursor + (remaining.len() - trimmed.len()) + delim.open.len();
                            has_comment = true;
                            at_line_start = false;
                            match find_close(
                                line_bytes,
                                after_open,
                                delim.close,
                                delim.backslash_escapes,
                            ) {
                                None => {
                                    self.state = State::InMultilineString {
                                        delim,
                                        is_docstring: true,
                                    };
                                    break;
                                }
                                Some(close_pos) => {
                                    cursor = close_pos + delim.close.len();
                                    continue;
                                }
                            }
                        }
                    }

                    // Scan for the earliest multiline opener (block comment or string)
                    match find_next_event(
                        remaining,
                        self.syntax,
                        &self.triggers,
                        self.n_triggers,
                    ) {
                        None => {
                            has_code = true;
                            break;
                        }
                        Some(Event::BlockComment { pos }) => {
                            if !remaining[..pos].trim_ascii_start().is_empty() {
                                has_code = true;
                            }
                            has_comment = true;
                            at_line_start = false;
                            let (block_start, block_end) = self.syntax.block_comment.unwrap();
                            let after_open = cursor + pos + block_start.len();
                            match memmem::find(&line_bytes[after_open..], block_end) {
                                None => {
                                    self.state = State::InBlockComment;
                                    break;
                                }
                                Some(off) => {
                                    cursor = after_open + off + block_end.len();
                                }
                            }
                        }
                        Some(Event::MultilineString { pos, delim }) => {
                            has_code = true;
                            at_line_start = false;
                            let after_open = cursor + pos + delim.open.len();
                            match find_close(
                                line_bytes,
                                after_open,
                                delim.close,
                                delim.backslash_escapes,
                            ) {
                                None => {
                                    self.state = State::InMultilineString {
                                        delim,
                                        is_docstring: false,
                                    };
                                    break;
                                }
                                Some(close_pos) => {
                                    cursor = close_pos + delim.close.len();
                                }
                            }
                        }
                    }
                }
            }
        }

        if has_code {
            LineClass::Code
        } else if has_comment {
            LineClass::Comment
        } else {
            LineClass::Blank
        }
    }

    /// Count lines by reading `reader` in chunks. Returns `(counts, total_bytes)`.
    ///
    /// Reads in `CHUNK_SIZE` blocks and uses `memchr_iter` to scan for newlines.
    /// The `carry` buffer stores the partial line at the end of the chunk.
    fn run<R: io::Read>(&mut self, mut reader: R) -> (Counts, u64) {
        CHUNK_BUF.with(|chunk_cell| {
            CARRY_BUF.with(|carry_cell| {
                let mut buf = chunk_cell.borrow_mut();
                let mut carry = carry_cell.borrow_mut();
                carry.clear();

                let mut counts = Counts::default();
                let mut total_bytes: u64 = 0;

                loop {
                    let n = match reader.read(&mut buf[..]) {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    };
                    total_bytes += n as u64;
                    let chunk = &buf[..n];

                    let mut prev = 0;
                    for nl in memchr::memchr_iter(b'\n', chunk) {
                        let segment = &chunk[prev..nl];
                        if carry.is_empty() {
                            self.process_line(segment, &mut counts);
                        } else {
                            carry.extend_from_slice(segment);
                            self.process_line(&carry, &mut counts);
                            carry.clear();
                        }
                        prev = nl + 1;
                    }
                    carry.extend_from_slice(&chunk[prev..]);
                }

                if !carry.is_empty() {
                    self.process_line(&carry, &mut counts);
                }

                (counts, total_bytes)
            })
        })
    }
}

#[cfg(test)]
pub fn count_lines_from_bytes(content: &[u8], syntax: &LangSyntax) -> Counts {
    Counter::new(syntax).run(content).0
}

pub fn count_lines_from_reader<R: io::Read>(reader: R, syntax: &LangSyntax) -> (Counts, u64) {
    Counter::new(syntax).run(reader)
}

impl AddAssign for Counts {
    fn add_assign(&mut self, other: Counts) {
        self.lines += other.lines;
        self.blanks += other.blanks;
        self.comments += other.comments;
        self.code += other.code;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_case {
        ($name:ident, $file:expr, $path:expr, $lines:expr, $blanks:expr, $comments:expr, $code:expr) => {
            #[test]
            fn $name() {
                let content = include_str!($path);
                let registry = crate::languages::LanguageRegistry::new();
                let (_, syntax) = registry
                    .get_language($file)
                    .expect("no language found for extension");
                let body = content.splitn(4, '\n').nth(3).unwrap_or("");
                assert_eq!(
                    count_lines_from_bytes(body.as_bytes(), syntax),
                    Counts {
                        lines: $lines,
                        blanks: $blanks,
                        comments: $comments,
                        code: $code
                    },
                );
            }
        };
    }

    include!(concat!(env!("OUT_DIR"), "/count_tests.rs"));
}
