//! A lexical scanner for Swift and Objective-C methods.
//!
//! E1 runs before the parser of E2 exists («до появления инструмента»), so
//! the measurement and the oracle tooling locate methods with this scanner:
//! comments and string literals are blanked, then declarations are found by
//! keyword and their bodies by brace matching. It is deliberately not the
//! tree-sitter parser of `swhy-core`: the oracle must not be produced by the
//! code it later evaluates.

/// Source language of a file, by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLang {
    Swift,
    ObjC,
}

impl SourceLang {
    /// `.swift` is Swift; `.m`, `.mm`, `.h` are Objective-C (docs/TZ.md).
    pub fn of_path(path: &str) -> Option<Self> {
        let extension = path.rsplit_once('.')?.1;
        match extension {
            "swift" => Some(Self::Swift),
            "m" | "mm" | "h" => Some(Self::ObjC),
            _ => None,
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Self::Swift => "swift",
            Self::ObjC => "objc",
        }
    }
}

/// One method found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub lang: SourceLang,
    /// Enclosing types joined with `.`; `<global>` for a free Swift function.
    pub container: String,
    /// Swift: the base name. ObjC: the first piece of the selector.
    pub name: String,
    /// Swift: `load(id:completion:)`. ObjC: `-loadCardsWithId:completion:`.
    pub signature: String,
    /// 1-based line of the `func` keyword or of the `-`/`+` sign.
    pub decl_line: u32,
    /// First line including the doc comments and attributes right above.
    pub start_line: u32,
    /// Last line: the closing brace, or the end of a declaration without body.
    pub end_line: u32,
    pub has_body: bool,
}

impl Method {
    /// Key in the format of docs/TZ.md, «Формат ключа символа».
    pub fn key(&self) -> String {
        format!(
            "{}:{}.{}",
            self.lang.prefix(),
            self.container,
            self.signature
        )
    }

    /// Lines `start_line..=end_line` of `source`.
    pub fn text(&self, source: &str) -> String {
        lines_text(source, self.start_line, self.end_line)
    }
}

/// Lines `start..=end` (1-based) of `source`, each ending with `\n`.
pub fn lines_text(source: &str, start: u32, end: u32) -> String {
    source
        .lines()
        .skip(start as usize - 1)
        .take((end + 1 - start) as usize)
        .flat_map(|line| [line, "\n"])
        .collect()
}

/// Methods of `source` in file order.
pub fn methods(lang: SourceLang, source: &str) -> Vec<Method> {
    match lang {
        SourceLang::Swift => swift_methods(source),
        SourceLang::ObjC => objc_methods(source),
    }
}

/// `source` with comments and the contents of string literals replaced by
/// spaces. Newlines stay, so offsets and line numbers do not change.
pub fn mask(lang: SourceLang, source: &str) -> Vec<u8> {
    let src = source.as_bytes();
    let mut out = src.to_vec();
    let mut i = 0;
    while i < src.len() {
        let end = match src[i] {
            b'/' if src.get(i + 1) == Some(&b'/') => memchr(src, i, b'\n').unwrap_or(src.len()),
            b'/' if src.get(i + 1) == Some(&b'*') => block_comment_end(src, i),
            b'"' => string_end(lang, src, i, 0),
            b'#' if lang == SourceLang::Swift => {
                let hashes = src[i..].iter().take_while(|&&b| b == b'#').count();
                if src.get(i + hashes) == Some(&b'"') {
                    string_end(lang, src, i + hashes, hashes)
                } else {
                    i += hashes;
                    continue;
                }
            }
            b'\'' if lang == SourceLang::ObjC => char_literal_end(src, i),
            _ => {
                i += 1;
                continue;
            }
        };
        for byte in &mut out[i..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        i = end;
    }
    out
}

fn memchr(src: &[u8], from: usize, needle: u8) -> Option<usize> {
    src[from..]
        .iter()
        .position(|&b| b == needle)
        .map(|at| from + at)
}

/// End of the block comment that starts at `start`; Swift comments nest.
fn block_comment_end(src: &[u8], start: usize) -> usize {
    let mut depth = 0usize;
    let mut i = start;
    while i + 1 < src.len() {
        if src[i] == b'/' && src[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if src[i] == b'*' && src[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    src.len()
}

fn char_literal_end(src: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < src.len() && i < start + 8 {
        match src[i] {
            b'\\' => i += 2,
            b'\'' => return i + 1,
            b'\n' => break,
            _ => i += 1,
        }
    }
    start + 1
}

/// End of the string literal whose opening quote is at `quote`. `hashes` is
/// the number of `#` of a Swift raw string.
fn string_end(lang: SourceLang, src: &[u8], quote: usize, hashes: usize) -> usize {
    let swift = lang == SourceLang::Swift;
    let multiline = swift && src[quote..].starts_with(b"\"\"\"");
    let mut i = quote + if multiline { 3 } else { 1 };
    let closes = |at: usize| {
        let quotes = if multiline { 3 } else { 1 };
        src[at..].starts_with(&b"\"\"\""[..quotes])
            && src[at + quotes..].iter().take(hashes).all(|&b| b == b'#')
            && src.len() >= at + quotes + hashes
    };
    while i < src.len() {
        match src[i] {
            b'"' if closes(i) => return i + if multiline { 3 } else { 1 } + hashes,
            b'\n' if !multiline => return i,
            b'\\' => {
                let escape = i + 1;
                let has_hashes = src[escape..].iter().take(hashes).all(|&b| b == b'#')
                    && src.len() >= escape + hashes;
                if swift && has_hashes && src.get(escape + hashes) == Some(&b'(') {
                    i = interpolation_end(lang, src, escape + hashes);
                } else if hashes == 0 {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    src.len()
}

/// End of the interpolation `\( … )` whose opening parenthesis is at `open`.
fn interpolation_end(lang: SourceLang, src: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while i < src.len() {
        match src[i] {
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return i;
                }
            }
            b'"' => i = string_end(lang, src, i, 0),
            _ => i += 1,
        }
    }
    src.len()
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

fn skip_ws(src: &[u8], mut i: usize) -> usize {
    while i < src.len() && src[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn ident_end(src: &[u8], mut i: usize) -> usize {
    while i < src.len() && is_ident(src[i]) {
        i += 1;
    }
    i
}

/// Index of the bracket that closes the one at `open`, on masked text.
fn matching(src: &[u8], open: usize) -> Option<usize> {
    let (left, right) = match src[open] {
        b'(' => (b'(', b')'),
        b'{' => (b'{', b'}'),
        b'[' => (b'[', b']'),
        _ => return None,
    };
    let mut depth = 0usize;
    for (at, &byte) in src.iter().enumerate().skip(open) {
        if byte == left {
            depth += 1;
        } else if byte == right {
            depth -= 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }
    None
}

/// Offsets of line starts, for offset → 1-based line conversion.
struct Lines(Vec<usize>);

impl Lines {
    fn new(src: &[u8]) -> Self {
        let mut starts = vec![0];
        starts.extend(
            src.iter()
                .enumerate()
                .filter(|&(_, &b)| b == b'\n')
                .map(|(i, _)| i + 1),
        );
        Self(starts)
    }

    fn line_of(&self, offset: usize) -> u32 {
        self.0.partition_point(|&start| start <= offset) as u32
    }
}

/// First line of the run of comments and attributes right above
/// `decl_line`. A blank line or a `// MARK` line ends the run.
fn leading_start(source: &str, decl_line: u32) -> u32 {
    let lines: Vec<&str> = source.lines().collect();
    let mut start = decl_line as usize - 1;
    let mut in_block = false;
    while start > 0 {
        let above = lines[start - 1].trim();
        if in_block || above.ends_with("*/") {
            in_block = !above.starts_with("/*");
        } else if above.starts_with("// MARK")
            || !(above.starts_with("//") || above.starts_with('@'))
        {
            break;
        }
        start -= 1;
    }
    start as u32 + 1
}

const SWIFT_TYPE_KEYWORDS: [&str; 6] =
    ["class", "struct", "enum", "extension", "actor", "protocol"];

/// Brace ranges of Swift type declarations: (open, close, name).
fn swift_types(masked: &[u8]) -> Vec<(usize, usize, String)> {
    let mut types = Vec::new();
    let mut i = 0;
    while i < masked.len() {
        if !is_ident(masked[i]) {
            i += 1;
            continue;
        }
        let end = ident_end(masked, i);
        let word = &masked[i..end];
        let preceded_by_dot = i > 0 && masked[i - 1] == b'.';
        if !preceded_by_dot && SWIFT_TYPE_KEYWORDS.iter().any(|k| k.as_bytes() == word) {
            let name_start = skip_ws(masked, end);
            let mut name_end = name_start;
            while name_end < masked.len()
                && (is_ident(masked[name_end]) || masked[name_end] == b'.')
            {
                name_end += 1;
            }
            let name = String::from_utf8_lossy(&masked[name_start..name_end]).into_owned();
            let is_member_keyword =
                ["func", "var", "let", "subscript", "init"].contains(&name.as_str());
            if !name.is_empty() && !is_member_keyword {
                // The body is the first `{` before any `}` or `;`.
                let open = masked[name_end..]
                    .iter()
                    .position(|&b| matches!(b, b'{' | b'}' | b';'))
                    .map(|at| name_end + at)
                    .filter(|&at| masked[at] == b'{');
                if let Some((open, close)) =
                    open.and_then(|open| Some((open, matching(masked, open)?)))
                {
                    types.push((open, close, name));
                }
            }
        }
        i = end;
    }
    types
}

/// Words that start a new member: a declaration without `{` before one of
/// them at the start of a line has no body (a protocol requirement).
const SWIFT_MEMBER_STARTS: [&str; 22] = [
    "func",
    "var",
    "let",
    "init",
    "deinit",
    "subscript",
    "static",
    "class",
    "struct",
    "enum",
    "extension",
    "protocol",
    "private",
    "fileprivate",
    "internal",
    "public",
    "open",
    "mutating",
    "nonmutating",
    "associatedtype",
    "typealias",
    "case",
];

fn swift_methods(source: &str) -> Vec<Method> {
    let masked = mask(SourceLang::Swift, source);
    let lines = Lines::new(&masked);
    let types = swift_types(&masked);
    let mut found = Vec::new();
    let mut i = 0;
    while i < masked.len() {
        if !is_ident(masked[i]) {
            i += 1;
            continue;
        }
        let word_end = ident_end(masked.as_slice(), i);
        if &masked[i..word_end] != b"func" || (i > 0 && matches!(masked[i - 1], b'.' | b'`')) {
            i = word_end;
            continue;
        }
        let keyword = i;
        i = word_end;
        let name_start = skip_ws(&masked, word_end);
        let name_end = if masked.get(name_start) == Some(&b'`') {
            memchr(&masked, name_start + 1, b'`').map_or(name_start, |at| at + 1)
        } else if masked.get(name_start).is_some_and(|&b| is_ident(b)) {
            ident_end(&masked, name_start)
        } else {
            // Operator function: everything up to `(` or `<`.
            masked[name_start..]
                .iter()
                .position(|&b| b == b'(' || b.is_ascii_whitespace())
                .map_or(name_start, |at| name_start + at)
        };
        if name_end == name_start {
            continue;
        }
        let name = String::from_utf8_lossy(&masked[name_start..name_end]).into_owned();
        // Generic parameters, then the parameter list.
        let mut at = skip_ws(&masked, name_end);
        if masked.get(at) == Some(&b'<') {
            let mut depth = 0usize;
            while at < masked.len() {
                match masked[at] {
                    b'<' => depth += 1,
                    b'>' if at > 0 && masked[at - 1] != b'-' => {
                        depth -= 1;
                        if depth == 0 {
                            at += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                at += 1;
            }
            at = skip_ws(&masked, at);
        }
        if masked.get(at) != Some(&b'(') {
            continue;
        }
        let Some(params_close) = matching(&masked, at) else {
            continue;
        };
        let labels = swift_labels(&masked[at + 1..params_close]);
        let (end, has_body) = swift_body_end(&masked, params_close + 1);
        let container: Vec<&str> = types
            .iter()
            .filter(|(open, close, _)| *open < keyword && keyword < *close)
            .map(|(_, _, name)| name.as_str())
            .collect();
        let decl_line = lines.line_of(keyword);
        found.push(Method {
            lang: SourceLang::Swift,
            container: if container.is_empty() {
                "<global>".to_string()
            } else {
                container.join(".")
            },
            signature: format!("{name}({labels})"),
            name,
            decl_line,
            start_line: leading_start(source, decl_line),
            end_line: lines.line_of(end),
            has_body,
        });
    }
    found
}

/// `id:completion:` from the masked text between the parentheses.
fn swift_labels(params: &[u8]) -> String {
    let mut labels = String::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut pieces = Vec::new();
    for (at, &byte) in params.iter().enumerate() {
        match byte {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b'>' if at > 0 && params[at - 1] != b'-' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                pieces.push(&params[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    pieces.push(&params[start..]);
    for piece in pieces {
        // Attributes such as `@escaping` belong to the type, after the colon;
        // an attribute before the label (`@ViewBuilder content:`) is skipped.
        let mut at = skip_ws(piece, 0);
        while piece.get(at) == Some(&b'@') {
            at = skip_ws(piece, ident_end(piece, at + 1));
            if piece.get(at) == Some(&b'(') {
                at = skip_ws(
                    piece,
                    matching(piece, at).map_or(piece.len(), |close| close + 1),
                );
            }
        }
        let end = ident_end(piece, at);
        if end > at {
            labels.push_str(&String::from_utf8_lossy(&piece[at..end]));
            labels.push(':');
        }
    }
    labels
}

/// Offset of the last byte of the declaration that continues at `from`
/// (right after the parameter list) and whether it has a body.
fn swift_body_end(masked: &[u8], from: usize) -> (usize, bool) {
    let mut at = from;
    let mut last_code = from - 1;
    while at < masked.len() {
        match masked[at] {
            b'{' => {
                return match matching(masked, at) {
                    Some(close) => (close, true),
                    None => (masked.len() - 1, true),
                };
            }
            b'}' | b';' => break,
            b'\n' => {
                let next = skip_ws(masked, at);
                let word = &masked[next..ident_end(masked, next)];
                let starts_member = SWIFT_MEMBER_STARTS.iter().any(|k| k.as_bytes() == word)
                    || matches!(masked.get(next), Some(b'@' | b'#' | b'}') | None);
                if starts_member {
                    break;
                }
                at = next;
            }
            byte => {
                if !byte.is_ascii_whitespace() {
                    last_code = at;
                }
                at += 1;
            }
        }
    }
    (last_code, false)
}

fn objc_methods(source: &str) -> Vec<Method> {
    let masked = mask(SourceLang::ObjC, source);
    let lines = Lines::new(&masked);
    // Containers: (offset, name) of every @interface / @implementation /
    // @protocol; the category is not part of the name.
    let mut containers: Vec<(usize, String)> = Vec::new();
    for keyword in ["@interface", "@implementation", "@protocol"] {
        let mut from = 0;
        while let Some(at) = find(&masked, from, keyword.as_bytes()) {
            from = at + keyword.len();
            let name_start = skip_ws(&masked, from);
            let name_end = ident_end(&masked, name_start);
            if name_end > name_start {
                let name = String::from_utf8_lossy(&masked[name_start..name_end]).into_owned();
                containers.push((at, name));
            }
        }
    }
    containers.sort();

    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    let mut line_start = true;
    while i < masked.len() {
        let byte = masked[i];
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'-' | b'+' if line_start && depth == 0 => {
                if let Some((method, end)) = objc_method_at(source, &masked, &lines, &containers, i)
                {
                    found.push(method);
                    i = end + 1;
                    line_start = false;
                    continue;
                }
            }
            _ => {}
        }
        if byte == b'\n' {
            line_start = true;
        } else if !byte.is_ascii_whitespace() {
            line_start = false;
        }
        i += 1;
    }
    found
}

fn find(haystack: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| from + at)
}

/// Parses the ObjC method whose sign is at `sign`; returns it with the
/// offset of its last byte.
fn objc_method_at(
    source: &str,
    masked: &[u8],
    lines: &Lines,
    containers: &[(usize, String)],
    sign: usize,
) -> Option<(Method, usize)> {
    let mut at = skip_ws(masked, sign + 1);
    if masked.get(at) != Some(&b'(') {
        return None;
    }
    at = skip_ws(masked, matching(masked, at)? + 1);
    let mut selector = String::new();
    let mut first_piece = String::new();
    loop {
        let piece_end = ident_end(masked, at);
        let piece = String::from_utf8_lossy(&masked[at..piece_end]).into_owned();
        let after = skip_ws(masked, piece_end);
        if masked.get(after) == Some(&b':') {
            if selector.is_empty() {
                first_piece.clone_from(&piece);
            }
            selector.push_str(&piece);
            selector.push(':');
            // `(type)name`, the type is optional.
            at = skip_ws(masked, after + 1);
            if masked.get(at) == Some(&b'(') {
                at = skip_ws(masked, matching(masked, at)? + 1);
            }
            at = skip_ws(masked, ident_end(masked, at));
        } else {
            if selector.is_empty() {
                if piece.is_empty() {
                    return None;
                }
                first_piece.clone_from(&piece);
                selector = piece;
                at = after;
            }
            break;
        }
        if matches!(masked.get(at), Some(b'{' | b';') | None) {
            break;
        }
        if !masked.get(at).is_some_and(|&b| is_ident(b) || b == b':') {
            break;
        }
    }
    // Skip trailing macros and variadic markers up to the body or `;`.
    let stop = masked[at..]
        .iter()
        .position(|&b| matches!(b, b'{' | b';' | b'}' | b'@'))
        .map(|offset| at + offset)?;
    let (end, has_body) = match masked[stop] {
        b'{' => (matching(masked, stop)?, true),
        b';' => (stop, false),
        _ => return None,
    };
    let container = containers
        .iter()
        .rev()
        .find(|(offset, _)| *offset < sign)
        .map_or("<global>", |(_, name)| name.as_str());
    let decl_line = lines.line_of(sign);
    let method = Method {
        lang: SourceLang::ObjC,
        container: container.to_string(),
        name: first_piece,
        signature: format!("{}{selector}", masked[sign] as char),
        decl_line,
        start_line: leading_start(source, decl_line),
        end_line: lines.line_of(end),
        has_body,
    };
    Some((method, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(lang: SourceLang, source: &str) -> Vec<String> {
        methods(lang, source).iter().map(Method::key).collect()
    }

    const SWIFT: &str = r##"import Foundation

/* a block comment with func fake() { */
final class CardViewModel {
    let text = "func alsoFake() { \(items.map { "\($0)" }) }"
    let raw = #"func raw() { "quoted" }"#

    /// Reloads.
    /// - Note: doc comment.
    @objc
    func reloadCards() {
        if cards.isEmpty { return } // func inComment()
    }

    func load(id: String, completion: @escaping ([Card]) -> Void) {
        completion([])
    }

    func load(_ id: String) {}

    static func make<T: Decodable>(from data: Data, as type: T.Type = T.self) -> T? { nil }

    enum State {
        case idle
        mutating func reset() {
            self = .idle
        }
    }

    class func shared() -> CardViewModel { CardViewModel() }
}

extension CardViewModel.State {
    func describe() -> String {
        """
        func inMultiline() {
        """
    }
}

protocol Loader {
    func load(id: String)
    func cancel()
}

func formatAmount(_ value: Int) -> String {
    return "\(value)"
}
"##;

    #[test]
    fn swift_keys_follow_the_format_of_the_spec() {
        assert_eq!(
            keys(SourceLang::Swift, SWIFT),
            [
                "swift:CardViewModel.reloadCards()",
                "swift:CardViewModel.load(id:completion:)",
                "swift:CardViewModel.load(_:)",
                "swift:CardViewModel.make(from:as:)",
                "swift:CardViewModel.State.reset()",
                "swift:CardViewModel.shared()",
                "swift:CardViewModel.State.describe()",
                "swift:Loader.load(id:)",
                "swift:Loader.cancel()",
                "swift:<global>.formatAmount(_:)",
            ]
        );
    }

    #[test]
    fn swift_ranges_include_doc_comments_and_attributes() {
        let found = methods(SourceLang::Swift, SWIFT);
        let reload = &found[0];
        assert_eq!(
            (
                reload.start_line,
                reload.decl_line,
                reload.end_line,
                reload.has_body
            ),
            (8, 11, 13, true)
        );
        assert_eq!(
            reload.text(SWIFT),
            "    /// Reloads.\n    /// - Note: doc comment.\n    @objc\n    func reloadCards() {\n        if cards.isEmpty { return } // func inComment()\n    }\n"
        );
        let one_line = &found[2];
        assert_eq!((one_line.start_line, one_line.end_line), (19, 19));
        let requirement = &found[7];
        assert_eq!(
            (
                requirement.decl_line,
                requirement.end_line,
                requirement.has_body
            ),
            (42, 42, false)
        );
        let global = &found[9];
        assert_eq!((global.decl_line, global.end_line), (46, 48));
    }

    const OBJC: &str = r#"#import "CardsController.h"

@implementation CardsController

// Loads the cards.
- (void)loadCardsWithId:(NSString *)id completion:(void (^)(NSArray *))completion {
    NSString *s = @"- (void)fake { }";
    if (id.length == 0) { completion(@[]); return; }
    char c = '{';
}

+ (instancetype)sharedInstance
{
    return nil;
}

- (NSInteger)count { return 0; }

@end

@implementation CardsController (Network)

- (void)cancel:(BOOL)animated {
}

@end
"#;

    #[test]
    fn objc_keys_use_the_selector_and_drop_the_category() {
        assert_eq!(
            keys(SourceLang::ObjC, OBJC),
            [
                "objc:CardsController.-loadCardsWithId:completion:",
                "objc:CardsController.+sharedInstance",
                "objc:CardsController.-count",
                "objc:CardsController.-cancel:",
            ]
        );
        let found = methods(SourceLang::ObjC, OBJC);
        assert_eq!(found[0].name, "loadCardsWithId");
        assert_eq!(
            (found[0].start_line, found[0].decl_line, found[0].end_line),
            (5, 6, 10)
        );
        assert_eq!((found[1].decl_line, found[1].end_line), (12, 15));
        assert!(found.iter().all(|method| method.has_body));
    }

    #[test]
    fn objc_header_declarations_have_no_body() {
        let header = "@interface OldMath : NSObject\n- (NSInteger)addValue:(NSInteger)a to:(NSInteger)b;\n- (void)reset NS_SWIFT_NAME(reset());\n@end\n";
        let found = methods(SourceLang::ObjC, header);
        assert_eq!(
            found.iter().map(Method::key).collect::<Vec<_>>(),
            ["objc:OldMath.-addValue:to:", "objc:OldMath.-reset"]
        );
        assert!(found.iter().all(|method| !method.has_body));
        assert_eq!((found[0].decl_line, found[0].end_line), (2, 2));
    }

    #[test]
    fn mask_keeps_length_and_newlines() {
        let source = "let a = \"x\\\"y\" // c\n/* b\n */ let b = 1\n";
        let masked = mask(SourceLang::Swift, source);
        assert_eq!(masked.len(), source.len());
        assert_eq!(
            String::from_utf8(masked).unwrap(),
            "let a =            \n    \n    let b = 1\n"
        );
    }

    #[test]
    fn language_is_taken_from_the_extension() {
        assert_eq!(SourceLang::of_path("A/B.swift"), Some(SourceLang::Swift));
        assert_eq!(SourceLang::of_path("A/B.m"), Some(SourceLang::ObjC));
        assert_eq!(SourceLang::of_path("A/B.mm"), Some(SourceLang::ObjC));
        assert_eq!(SourceLang::of_path("A/B.h"), Some(SourceLang::ObjC));
        assert_eq!(SourceLang::of_path("A/B.md"), None);
        assert_eq!(SourceLang::of_path("Makefile"), None);
    }
}
