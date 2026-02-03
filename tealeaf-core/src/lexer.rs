//! Lexer for TeaLeaf text format

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Word(String),
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Timestamp(i64),  // Unix milliseconds

    // Punctuation
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Colon,
    Comma,
    Eq,
    Question,  // For nullable types (e.g., string?)

    // Special
    Directive(String),
    Tag(String),
    Ref(String),

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Self { kind, line, col }
    }
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = matches!(tok.kind, TokenKind::Eof);
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;

        if self.pos >= self.input.len() {
            return Ok(Token::new(TokenKind::Eof, line, col));
        }

        let c = self.current_char().unwrap();

        // Simple single-char tokens
        let simple = match c {
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            ',' => Some(TokenKind::Comma),
            '=' => Some(TokenKind::Eq),
            '~' => Some(TokenKind::Null),
            '?' => Some(TokenKind::Question),
            _ => None,
        };

        if let Some(kind) = simple {
            self.advance();
            return Ok(Token::new(kind, line, col));
        }

        // Colon - might be a tag
        if c == ':' {
            self.advance();
            if self.current_char().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                let word = self.read_word();
                return Ok(Token::new(TokenKind::Tag(word), line, col));
            }
            return Ok(Token::new(TokenKind::Colon, line, col));
        }

        // Directive
        if c == '@' {
            self.advance();
            let word = self.read_word();
            return Ok(Token::new(TokenKind::Directive(word), line, col));
        }

        // Reference
        if c == '!' {
            self.advance();
            let word = self.read_word();
            return Ok(Token::new(TokenKind::Ref(word), line, col));
        }

        // String
        if c == '"' {
            return self.read_string(line, col);
        }

        // Timestamp (must check before number - pattern: YYYY-MM-DD...)
        if c.is_ascii_digit() {
            let remaining = &self.input[self.pos..];
            if remaining.len() >= 10 &&
               remaining.chars().nth(4) == Some('-') &&
               remaining.chars().nth(7) == Some('-') {
                return self.read_timestamp(line, col);
            }
        }

        // Number
        if c.is_ascii_digit() || (c == '-' && self.peek_char(1).map(|c| c.is_ascii_digit()).unwrap_or(false)) {
            return self.read_number(line, col);
        }

        // Word or keyword
        if c.is_alphabetic() || c == '_' {
            let word = self.read_word();
            let kind = match word.as_str() {
                "true" => TokenKind::Bool(true),
                "false" => TokenKind::Bool(false),
                _ => TokenKind::Word(word),
            };
            return Ok(Token::new(kind, line, col));
        }

        // Skip unknown character
        self.advance();
        self.next_token()
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '#' {
                // Skip comment to end of line
                while let Some(c) = self.current_char() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_word(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn read_string(&mut self, line: usize, col: usize) -> Result<Token> {
        self.advance(); // Skip opening quote

        // Check for multiline
        if self.input[self.pos..].starts_with("\"\"") {
            self.advance();
            self.advance();
            return self.read_multiline_string(line, col);
        }

        let mut value = String::new();
        while let Some(c) = self.current_char() {
            if c == '"' {
                self.advance();
                return Ok(Token::new(TokenKind::String(value), line, col));
            } else if c == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char() {
                    match escaped {
                        'n' => { value.push('\n'); self.advance(); }
                        't' => { value.push('\t'); self.advance(); }
                        'r' => { value.push('\r'); self.advance(); }
                        'b' => { value.push('\u{0008}'); self.advance(); }
                        'f' => { value.push('\u{000C}'); self.advance(); }
                        '"' => { value.push('"'); self.advance(); }
                        '\\' => { value.push('\\'); self.advance(); }
                        'u' => {
                            self.advance(); // skip 'u'
                            let start = self.pos;
                            let mut count = 0;
                            while count < 4 {
                                match self.current_char() {
                                    Some(c) if c.is_ascii_hexdigit() => {
                                        self.advance();
                                        count += 1;
                                    }
                                    _ => break,
                                }
                            }
                            if count != 4 {
                                return Err(Error::ParseError(
                                    "Invalid unicode escape: expected 4 hex digits after \\u".to_string()
                                ));
                            }
                            let hex = &self.input[start..self.pos];
                            let code = u32::from_str_radix(hex, 16).map_err(|_| {
                                Error::ParseError(format!("Invalid unicode escape: \\u{}", hex))
                            })?;
                            let ch = char::from_u32(code).ok_or_else(|| {
                                Error::ParseError(format!("Invalid unicode codepoint: U+{:04X}", code))
                            })?;
                            value.push(ch);
                        }
                        _ => {
                            return Err(Error::ParseError(
                                format!("Invalid escape sequence: \\{}", escaped)
                            ));
                        }
                    }
                }
            } else {
                value.push(c);
                self.advance();
            }
        }
        Err(Error::ParseError("Unterminated string".to_string()))
    }

    fn read_multiline_string(&mut self, line: usize, col: usize) -> Result<Token> {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input[self.pos..].starts_with("\"\"\"") {
                let raw = &self.input[start..self.pos];
                self.advance();
                self.advance();
                self.advance();

                // Dedent
                let lines: Vec<&str> = raw.lines().collect();
                let lines: Vec<&str> = if lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines[1..].to_vec()
                } else {
                    lines
                };
                let lines: Vec<&str> = if lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines[..lines.len() - 1].to_vec()
                } else {
                    lines
                };

                let min_indent = lines
                    .iter()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.len() - l.trim_start().len())
                    .min()
                    .unwrap_or(0);

                let dedented: Vec<&str> = lines
                    .iter()
                    .map(|l| if l.len() >= min_indent { &l[min_indent..] } else { *l })
                    .collect();

                return Ok(Token::new(TokenKind::String(dedented.join("\n")), line, col));
            }
            self.advance();
        }
        Err(Error::ParseError("Unterminated multiline string".to_string()))
    }

    fn read_timestamp(&mut self, line: usize, col: usize) -> Result<Token> {
        let start = self.pos;

        // Read date part: YYYY-MM-DD
        for _ in 0..10 {
            self.advance();
        }

        // Check for time part: THH:MM:SS
        if self.current_char() == Some('T') {
            self.advance();
            // Read HH:MM:SS
            while let Some(c) = self.current_char() {
                if c.is_ascii_digit() || c == ':' {
                    self.advance();
                } else {
                    break;
                }
            }
            // Optional milliseconds .sss
            if self.current_char() == Some('.') {
                self.advance();
                while let Some(c) = self.current_char() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            // Timezone: Z or +HH:MM or -HH:MM
            if self.current_char() == Some('Z') {
                self.advance();
            } else if self.current_char() == Some('+') || self.current_char() == Some('-') {
                self.advance();
                // Read HH:MM
                while let Some(c) = self.current_char() {
                    if c.is_ascii_digit() || c == ':' {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let timestamp_str = &self.input[start..self.pos];
        let millis = parse_iso8601(timestamp_str)
            .map_err(|_| Error::ParseError(format!("Invalid timestamp: {}", timestamp_str)))?;

        Ok(Token::new(TokenKind::Timestamp(millis), line, col))
    }

    fn read_number(&mut self, line: usize, col: usize) -> Result<Token> {
        let start = self.pos;

        // Handle negative
        if self.current_char() == Some('-') {
            self.advance();
        }

        // Hex
        if self.input[self.pos..].starts_with("0x") || self.input[self.pos..].starts_with("0X") {
            self.advance();
            self.advance();
            while let Some(c) = self.current_char() {
                if c.is_ascii_hexdigit() {
                    self.advance();
                } else {
                    break;
                }
            }
            let s = &self.input[start..self.pos];
            let val = if s.starts_with('-') {
                -(i64::from_str_radix(&s[3..], 16).map_err(|_| Error::ParseError(format!("Invalid hex: {}", s)))?)
            } else {
                i64::from_str_radix(&s[2..], 16).map_err(|_| Error::ParseError(format!("Invalid hex: {}", s)))?
            };
            return Ok(Token::new(TokenKind::Int(val), line, col));
        }

        // Binary
        if self.input[self.pos..].starts_with("0b") || self.input[self.pos..].starts_with("0B") {
            self.advance();
            self.advance();
            while let Some(c) = self.current_char() {
                if c == '0' || c == '1' {
                    self.advance();
                } else {
                    break;
                }
            }
            let s = &self.input[start..self.pos];
            let val = if s.starts_with('-') {
                -(i64::from_str_radix(&s[3..], 2).map_err(|_| Error::ParseError(format!("Invalid binary: {}", s)))?)
            } else {
                i64::from_str_radix(&s[2..], 2).map_err(|_| Error::ParseError(format!("Invalid binary: {}", s)))?
            };
            return Ok(Token::new(TokenKind::Int(val), line, col));
        }

        // Regular number
        let mut has_dot = false;
        let mut has_exp = false;
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' && !has_dot && !has_exp {
                has_dot = true;
                self.advance();
            } else if (c == 'e' || c == 'E') && !has_exp {
                has_exp = true;
                self.advance();
                if self.current_char() == Some('+') || self.current_char() == Some('-') {
                    self.advance();
                }
            } else {
                break;
            }
        }

        let s = &self.input[start..self.pos];
        if has_dot || has_exp {
            let val: f64 = s.parse().map_err(|_| Error::ParseError(format!("Invalid float: {}", s)))?;
            Ok(Token::new(TokenKind::Float(val), line, col))
        } else {
            let val: i64 = s.parse().map_err(|_| Error::ParseError(format!("Invalid int: {}", s)))?;
            Ok(Token::new(TokenKind::Int(val), line, col))
        }
    }
}

/// Parse an ISO 8601 timestamp string to Unix milliseconds
fn parse_iso8601(s: &str) -> std::result::Result<i64, ()> {
    // Parse YYYY-MM-DD[THH:MM:SS[.sss][Z|+HH:MM|-HH:MM]]
    let bytes = s.as_bytes();
    if bytes.len() < 10 {
        return Err(());
    }

    let year: i32 = s[0..4].parse().map_err(|_| ())?;
    let month: u32 = s[5..7].parse().map_err(|_| ())?;
    let day: u32 = s[8..10].parse().map_err(|_| ())?;

    let (hour, minute, second, millis, tz_offset_minutes) = if s.len() > 10 && s.as_bytes()[10] == b'T' {
        let time_part = &s[11..];
        let hour: u32 = time_part.get(0..2).ok_or(())?.parse().map_err(|_| ())?;
        let minute: u32 = time_part.get(3..5).ok_or(())?.parse().map_err(|_| ())?;
        let second: u32 = time_part.get(6..8).unwrap_or("00").parse().unwrap_or(0);

        let mut millis = 0i64;
        let mut rest = &time_part[8.min(time_part.len())..];

        // Parse milliseconds
        if rest.starts_with('.') {
            let end = rest[1..].find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len() - 1);
            let ms_str = &rest[1..=end];
            millis = ms_str.parse::<i64>().unwrap_or(0);
            // Normalize to milliseconds (could be 1-6 digits)
            let digits = ms_str.len();
            if digits < 3 {
                millis *= 10i64.pow(3 - digits as u32);
            } else if digits > 3 {
                millis /= 10i64.pow(digits as u32 - 3);
            }
            rest = &rest[end + 1..];
        }

        // Parse timezone
        let tz_offset = if rest.starts_with('Z') {
            0
        } else if rest.starts_with('+') || rest.starts_with('-') {
            let sign = if rest.starts_with('+') { 1 } else { -1 };
            let tz = &rest[1..];
            let tz_hour: i32 = tz.get(0..2).unwrap_or("00").parse().unwrap_or(0);
            let tz_min: i32 = tz.get(3..5).unwrap_or("00").parse().unwrap_or(0);
            sign * (tz_hour * 60 + tz_min)
        } else {
            0 // Assume UTC if no timezone
        };

        (hour, minute, second, millis, tz_offset)
    } else {
        (0, 0, 0, 0, 0)
    };

    // Calculate Unix timestamp
    // Days from epoch (1970-01-01)
    let days = days_from_epoch(year, month, day);
    let seconds = days as i64 * 86400
        + hour as i64 * 3600
        + minute as i64 * 60
        + second as i64
        - tz_offset_minutes as i64 * 60;

    Ok(seconds * 1000 + millis)
}

/// Calculate days from Unix epoch (1970-01-01)
fn days_from_epoch(year: i32, month: u32, day: u32) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (m - 3) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i32 - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("{ } [ ] ( ) : , ~");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LBrace));
        assert!(matches!(tokens[1].kind, TokenKind::RBrace));
        assert!(matches!(tokens[2].kind, TokenKind::LBracket));
        assert!(matches!(tokens[8].kind, TokenKind::Null));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 -17 3.14 0xFF 0b1010");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(42)));
        assert!(matches!(tokens[1].kind, TokenKind::Int(-17)));
        assert!(matches!(tokens[2].kind, TokenKind::Float(f) if (f - 3.14).abs() < 0.001));
        assert!(matches!(tokens[3].kind, TokenKind::Int(255)));
        assert!(matches!(tokens[4].kind, TokenKind::Int(10)));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "hello"));
        assert!(matches!(&tokens[1].kind, TokenKind::String(s) if s == "world\n"));
    }

    #[test]
    fn test_directives() {
        let mut lexer = Lexer::new("@struct @table");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Directive(s) if s == "struct"));
        assert!(matches!(&tokens[1].kind, TokenKind::Directive(s) if s == "table"));
    }

    #[test]
    fn test_references() {
        let mut lexer = Lexer::new("!myref !another_ref");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Ref(s) if s == "myref"));
        assert!(matches!(&tokens[1].kind, TokenKind::Ref(s) if s == "another_ref"));
    }

    #[test]
    fn test_comments_and_references() {
        // # is always a comment
        let mut lexer = Lexer::new("value1 # this is a comment\nvalue2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Word(s) if s == "value1"));
        assert!(matches!(&tokens[1].kind, TokenKind::Word(s) if s == "value2"));
        assert!(matches!(tokens[2].kind, TokenKind::Eof));

        // ! is a reference
        let mut lexer = Lexer::new("value1 !ref value2");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Word(s) if s == "value1"));
        assert!(matches!(&tokens[1].kind, TokenKind::Ref(s) if s == "ref"));
        assert!(matches!(&tokens[2].kind, TokenKind::Word(s) if s == "value2"));
    }

    // -------------------------------------------------------------------------
    // String escape sequences
    // -------------------------------------------------------------------------

    #[test]
    fn test_string_escape_tab() {
        let mut lexer = Lexer::new(r#""\t""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\t"));
    }

    #[test]
    fn test_string_escape_cr() {
        let mut lexer = Lexer::new(r#""\r""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\r"));
    }

    #[test]
    fn test_string_escape_backspace() {
        let mut lexer = Lexer::new(r#""\b""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\u{0008}"));
    }

    #[test]
    fn test_string_escape_formfeed() {
        let mut lexer = Lexer::new(r#""\f""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\u{000C}"));
    }

    #[test]
    fn test_string_escape_backslash() {
        let mut lexer = Lexer::new(r#""\\""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\\"));
    }

    #[test]
    fn test_string_escape_quote() {
        let mut lexer = Lexer::new(r#""\"hello\"""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\"hello\""));
    }

    #[test]
    fn test_string_escape_unicode() {
        let mut lexer = Lexer::new(r#""\u0041""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "A"));
    }

    #[test]
    fn test_string_escape_unicode_emoji_range() {
        // Heart suit: U+2665
        let mut lexer = Lexer::new(r#""\u2665""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s == "\u{2665}"));
    }

    #[test]
    fn test_string_invalid_escape() {
        let mut lexer = Lexer::new(r#""\x""#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Invalid escape sequence"));
    }

    #[test]
    fn test_string_invalid_unicode_short() {
        let mut lexer = Lexer::new(r#""\u00""#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Invalid unicode escape"));
    }

    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new(r#""hello"#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Unterminated string"));
    }

    // -------------------------------------------------------------------------
    // Multiline strings
    // -------------------------------------------------------------------------

    #[test]
    fn test_multiline_string() {
        let input = "\"\"\"
    hello
    world
\"\"\"";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(s) if s.contains("hello") && s.contains("world")));
    }

    #[test]
    fn test_unterminated_multiline_string() {
        let input = "\"\"\"
    hello world";
        let mut lexer = Lexer::new(input);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Unterminated multiline string"));
    }

    // -------------------------------------------------------------------------
    // Timestamps
    // -------------------------------------------------------------------------

    #[test]
    fn test_timestamp_basic() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00Z");
        let tokens = lexer.tokenize().unwrap();
        match &tokens[0].kind {
            TokenKind::Timestamp(ts) => {
                // 2024-01-15T10:30:00Z should be a valid timestamp
                assert!(*ts > 0);
            }
            other => panic!("Expected Timestamp, got {:?}", other),
        }
    }

    #[test]
    fn test_timestamp_with_millis() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00.123Z");
        let tokens = lexer.tokenize().unwrap();
        match &tokens[0].kind {
            TokenKind::Timestamp(ts) => {
                assert_eq!(*ts % 1000, 123); // milliseconds preserved
            }
            other => panic!("Expected Timestamp, got {:?}", other),
        }
    }

    #[test]
    fn test_timestamp_date_only() {
        let mut lexer = Lexer::new("2024-01-15");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Timestamp(_)));
    }

    #[test]
    fn test_timestamp_with_offset() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00+05:30");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Timestamp(_)));
    }

    #[test]
    fn test_timestamp_with_negative_offset() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00-08:00");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Timestamp(_)));
    }

    // -------------------------------------------------------------------------
    // Number edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_scientific_notation() {
        let mut lexer = Lexer::new("1.5e10 2.3E-5 1e+3");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Float(f) if (f - 1.5e10).abs() < 1.0));
        assert!(matches!(tokens[1].kind, TokenKind::Float(f) if (f - 2.3e-5).abs() < 1e-10));
        assert!(matches!(tokens[2].kind, TokenKind::Float(f) if (f - 1e3).abs() < 1.0));
    }

    #[test]
    fn test_binary_literal() {
        let mut lexer = Lexer::new("0b1100 0B1010");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(12)));
        assert!(matches!(tokens[1].kind, TokenKind::Int(10)));
    }

    #[test]
    fn test_hex_uppercase() {
        let mut lexer = Lexer::new("0XDEAD");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(0xDEAD)));
    }

    #[test]
    fn test_negative_number() {
        let mut lexer = Lexer::new("-42 -3.14");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(-42)));
        assert!(matches!(tokens[1].kind, TokenKind::Float(f) if (f - (-3.14)).abs() < 0.001));
    }

    // -------------------------------------------------------------------------
    // Tags and special tokens
    // -------------------------------------------------------------------------

    #[test]
    fn test_tag_token() {
        let mut lexer = Lexer::new(":Circle {radius: 5.0}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Tag(s) if s == "Circle"));
    }

    #[test]
    fn test_colon_without_word() {
        let mut lexer = Lexer::new(": 5");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Colon));
    }

    #[test]
    fn test_question_mark() {
        let mut lexer = Lexer::new("string?");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Word(s) if s == "string"));
        assert!(matches!(tokens[1].kind, TokenKind::Question));
    }

    #[test]
    fn test_equals_token() {
        let mut lexer = Lexer::new("x = 5");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[1].kind, TokenKind::Eq));
    }

    #[test]
    fn test_bool_keywords() {
        let mut lexer = Lexer::new("true false");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Bool(true)));
        assert!(matches!(tokens[1].kind, TokenKind::Bool(false)));
    }

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \n\t  ");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_token_positions() {
        let mut lexer = Lexer::new("hello: 42");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].col, 1);
    }

    #[test]
    fn test_all_brackets() {
        let mut lexer = Lexer::new("() {} []");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
        assert!(matches!(tokens[1].kind, TokenKind::RParen));
        assert!(matches!(tokens[2].kind, TokenKind::LBrace));
        assert!(matches!(tokens[3].kind, TokenKind::RBrace));
        assert!(matches!(tokens[4].kind, TokenKind::LBracket));
        assert!(matches!(tokens[5].kind, TokenKind::RBracket));
    }
}
