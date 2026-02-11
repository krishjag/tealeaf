//! Lexer for TeaLeaf text format

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Word(String),
    String(String),
    Bytes(Vec<u8>),
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    Null,
    Timestamp(i64, i16),  // Unix milliseconds, timezone offset in minutes
    JsonNumber(String),  // Arbitrary-precision number (raw decimal string)

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
        loop {
            self.skip_whitespace_and_comments();

            let line = self.line;
            let col = self.col;

            if self.pos >= self.input.len() {
                return Ok(Token::new(TokenKind::Eof, line, col));
            }

            let c = match self.current_char() {
                Some(c) => c,
                None => return Ok(Token::new(TokenKind::Eof, line, col)),
            };

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
                ':' => Some(TokenKind::Colon),
                _ => None,
            };

            if let Some(kind) = simple {
                self.advance();
                return Ok(Token::new(kind, line, col));
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

            // Bytes literal: b"hex..."
            if c == 'b' && self.peek_char(1) == Some('"') {
                return self.read_bytes_literal(line, col);
            }

            // String
            if c == '"' {
                return self.read_string(line, col);
            }

            // Timestamp (must check before number - pattern: YYYY-MM-DD...)
            // Validate full date pattern with ASCII digits to prevent
            // parse_iso8601 from slicing into multi-byte characters.
            // Strictly 4-digit years per spec: date = digit{4} "-" digit{2} "-" digit{2}
            if c.is_ascii_digit() {
                let remaining = self.input[self.pos..].as_bytes();
                if remaining.len() >= 10
                   && remaining[0].is_ascii_digit()
                   && remaining[1].is_ascii_digit()
                   && remaining[2].is_ascii_digit()
                   && remaining[3].is_ascii_digit()
                   && remaining[4] == b'-'
                   && remaining[5].is_ascii_digit()
                   && remaining[6].is_ascii_digit()
                   && remaining[7] == b'-'
                   && remaining[8].is_ascii_digit()
                   && remaining[9].is_ascii_digit()
                {
                    return self.read_timestamp(line, col);
                }
            }

            // Negative infinity: -inf
            if c == '-' && self.input[self.pos..].starts_with("-inf") {
                // Make sure it's not a prefix of a longer word like "-info"
                let after = self.input.get(self.pos + 4..self.pos + 5)
                    .and_then(|s| s.chars().next());
                if after.map_or(true, |c| !c.is_alphanumeric() && c != '_') {
                    self.pos += 4;
                    self.col += 4;
                    return Ok(Token::new(TokenKind::Float(f64::NEG_INFINITY), line, col));
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
                    "NaN" => TokenKind::Float(f64::NAN),
                    "inf" => TokenKind::Float(f64::INFINITY),
                    _ => TokenKind::Word(word),
                };
                return Ok(Token::new(kind, line, col));
            }

            // Skip unknown character and loop to try next
            self.advance();
        }
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

    fn read_bytes_literal(&mut self, line: usize, col: usize) -> Result<Token> {
        self.advance(); // skip 'b'
        self.advance(); // skip '"'

        let mut hex = String::new();
        while let Some(c) = self.current_char() {
            if c == '"' {
                self.advance();
                if hex.len() % 2 != 0 {
                    return Err(Error::ParseError(
                        format!("Bytes literal has odd number of hex digits ({})", hex.len())
                    ));
                }
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_|
                        Error::ParseError(format!("Invalid hex pair '{}' in bytes literal", &hex[i..i + 2]))
                    ))
                    .collect::<Result<Vec<u8>>>()?;
                return Ok(Token::new(TokenKind::Bytes(bytes), line, col));
            } else if c.is_ascii_hexdigit() {
                hex.push(c);
                self.advance();
            } else {
                return Err(Error::ParseError(
                    format!("Invalid character '{}' in bytes literal (expected hex digit or '\"')", c)
                ));
            }
        }
        Err(Error::ParseError("Unterminated bytes literal".to_string()))
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
                let lines: Vec<&str> = if lines.len() > 1 && lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines[1..].to_vec()
                } else {
                    lines
                };
                let lines: Vec<&str> = if lines.len() > 1 && lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
                    lines[..lines.len() - 1].to_vec()
                } else {
                    lines
                };

                // Count indent in characters (not bytes) to safely handle
                // multi-byte whitespace like U+0085 (NEXT LINE, 2 bytes).
                let min_indent = lines
                    .iter()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                    .min()
                    .unwrap_or(0);

                let dedented: Vec<&str> = lines
                    .iter()
                    .map(|l| {
                        // Find the byte offset after skipping min_indent characters
                        let byte_off: usize = l.chars().take(min_indent).map(|c| c.len_utf8()).sum();
                        if byte_off <= l.len() { &l[byte_off..] } else { *l }
                    })
                    .collect();

                return Ok(Token::new(TokenKind::String(dedented.join("\n")), line, col));
            }
            self.advance();
        }
        Err(Error::ParseError("Unterminated multiline string".to_string()))
    }

    fn read_timestamp(&mut self, line: usize, col: usize) -> Result<Token> {
        let start = self.pos;

        // Read YYYY-MM-DD (exactly 10 characters)
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
        let (millis, tz_offset) = parse_iso8601(timestamp_str)
            .map_err(|_| Error::ParseError(format!("Invalid timestamp: {}", timestamp_str)))?;

        Ok(Token::new(TokenKind::Timestamp(millis, tz_offset), line, col))
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
            if val.is_finite() {
                Ok(Token::new(TokenKind::Float(val), line, col))
            } else {
                Ok(Token::new(TokenKind::JsonNumber(s.to_string()), line, col))
            }
        } else {
            // Try i64 first, then u64, then preserve as JsonNumber
            match s.parse::<i64>() {
                Ok(val) => Ok(Token::new(TokenKind::Int(val), line, col)),
                Err(_) => match s.parse::<u64>() {
                    Ok(val) => Ok(Token::new(TokenKind::UInt(val), line, col)),
                    Err(_) => Ok(Token::new(TokenKind::JsonNumber(s.to_string()), line, col)),
                }
            }
        }
    }
}

/// Parse an ISO 8601 timestamp string to Unix milliseconds and timezone offset.
/// Strictly 4-digit years per spec: YYYY-MM-DD[THH:MM[:SS[.sss]][Z|+HH:MM|-HH:MM]]
/// Returns (unix_millis, tz_offset_minutes).
fn parse_iso8601(s: &str) -> std::result::Result<(i64, i16), ()> {
    // Safety: reject any non-ASCII input up front so that byte-position
    // slicing cannot split multi-byte characters.
    if !s.is_ascii() {
        return Err(());
    }

    if s.len() < 10 {
        return Err(());
    }

    let year: i64 = s[0..4].parse().map_err(|_| ())?;
    let month: u32 = s[5..7].parse().map_err(|_| ())?;
    let day: u32 = s[8..10].parse().map_err(|_| ())?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(());
    }

    let time_start = 10;
    let (hour, minute, second, millis, tz_offset_minutes) = if s.len() > time_start && s.as_bytes()[time_start] == b'T' {
        let time_part = &s[time_start + 1..];
        let hour: u32 = time_part.get(0..2).ok_or(())?.parse().map_err(|_| ())?;
        let minute: u32 = time_part.get(3..5).ok_or(())?.parse().map_err(|_| ())?;

        // Determine whether seconds are present or timezone follows directly.
        // After HH:MM (positions 0-4), position 5 tells us:
        //   ':' → seconds at 6..8, rest starts at 8
        //   '+'/'-'/'Z' → no seconds, timezone starts at 5
        //   end of string → no seconds, no timezone
        let (second, rest_start) = if time_part.len() > 5 {
            match time_part.as_bytes()[5] {
                b':' => {
                    let sec: u32 = time_part.get(6..8).ok_or(())?.parse().map_err(|_| ())?;
                    (sec, 8usize)
                }
                b'+' | b'-' | b'Z' => (0u32, 5usize),
                _ => (0u32, time_part.len()),
            }
        } else {
            (0u32, time_part.len())
        };

        // Validate time component ranges
        if hour > 23 || minute > 59 || second > 59 {
            return Err(());
        }

        let mut millis = 0i64;
        let mut rest = &time_part[rest_start.min(time_part.len())..];

        // Parse milliseconds (only first 3 fractional digits matter)
        if rest.starts_with('.') && rest.len() > 1 {
            let end = rest[1..].find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len() - 1);
            if end == 0 {
                return Err(());
            }
            // Cap to 3 digits — we only need millisecond precision and
            // longer strings can overflow i64::pow (e.g. 22 digits → 10^19).
            let frac_digits = end.min(3);
            let ms_str = &rest[1..1 + frac_digits];
            millis = ms_str.parse::<i64>().unwrap_or(0);
            let digits = ms_str.len();
            if digits < 3 {
                millis *= 10i64.pow(3 - digits as u32);
            }
            rest = &rest[end + 1..];
        } else if rest.starts_with('.') {
            // Just a trailing dot with no digits — skip it
            rest = &rest[1..];
        }

        // Parse timezone
        let tz_offset = if rest.starts_with('Z') {
            0i32
        } else if rest.starts_with('+') || rest.starts_with('-') {
            let sign: i32 = if rest.starts_with('+') { 1 } else { -1 };
            let tz = &rest[1..];
            let tz_hour: i32 = tz.get(0..2).ok_or(())?.parse().map_err(|_| ())?;
            // Accept +HH:MM, +HHMM, or +HH (minutes default to 00)
            let tz_min: i32 = if tz.len() >= 4 && tz.as_bytes()[2] == b':' {
                tz.get(3..5).unwrap_or("00").parse().unwrap_or(0)   // +HH:MM
            } else if tz.len() >= 4 && tz.as_bytes()[2] != b':' {
                tz.get(2..4).unwrap_or("00").parse().unwrap_or(0)   // +HHMM
            } else {
                0                                                     // +HH
            };
            if tz_hour > 23 || tz_min > 59 {
                return Err(());
            }
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
    let seconds = days * 86400
        + hour as i64 * 3600
        + minute as i64 * 60
        + second as i64
        - tz_offset_minutes as i64 * 60;

    Ok((seconds * 1000 + millis, tz_offset_minutes as i16))
}

/// Calculate days from Unix epoch (1970-01-01)
fn days_from_epoch(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (m - 3) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
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
            TokenKind::Timestamp(ts, _tz) => {
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
            TokenKind::Timestamp(ts, _tz) => {
                assert_eq!(*ts % 1000, 123); // milliseconds preserved
            }
            other => panic!("Expected Timestamp, got {:?}", other),
        }
    }

    #[test]
    fn test_timestamp_date_only() {
        let mut lexer = Lexer::new("2024-01-15");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Timestamp(_, _)));
    }

    #[test]
    fn test_timestamp_with_offset() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00+05:30");
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Timestamp(_, tz) = tokens[0].kind { assert_eq!(tz, 330); }
        else { panic!("expected timestamp"); }
    }

    #[test]
    fn test_timestamp_with_negative_offset() {
        let mut lexer = Lexer::new("2024-01-15T10:30:00-08:00");
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Timestamp(_, tz) = tokens[0].kind { assert_eq!(tz, -480); }
        else { panic!("expected timestamp"); }
    }

    #[test]
    fn test_timestamp_offset_formats() {
        // +HH:MM (standard)
        let mut lexer = Lexer::new("2024-01-15T10:30:00+05:30");
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Timestamp(_, tz) = tokens[0].kind { assert_eq!(tz, 330); }
        else { panic!("expected timestamp"); }

        // +HHMM (compact, no colon)
        let mut lexer = Lexer::new("2024-01-15T10:30:00+0530");
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Timestamp(_, tz) = tokens[0].kind { assert_eq!(tz, 330); }
        else { panic!("expected timestamp for +HHMM"); }

        // +HH (hour-only, minutes default to 00)
        let mut lexer = Lexer::new("2024-01-15T10:30:00+05");
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::Timestamp(_, tz) = tokens[0].kind { assert_eq!(tz, 300); }
        else { panic!("expected timestamp for +HH"); }
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
    fn test_colon_then_word() {
        // `:Circle` is now lexed as Colon + Word("Circle"), not Tag("Circle")
        let mut lexer = Lexer::new(":Circle {radius: 5.0}");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Colon));
        assert!(matches!(&tokens[1].kind, TokenKind::Word(s) if s == "Circle"));
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

    // -------------------------------------------------------------------------
    // Bytes literals
    // -------------------------------------------------------------------------

    #[test]
    fn test_bytes_literal_basic() {
        let mut lexer = Lexer::new(r#"b"48656c6c6f""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Bytes(b) if b == &[0x48, 0x65, 0x6c, 0x6c, 0x6f]));
    }

    #[test]
    fn test_bytes_literal_empty() {
        let mut lexer = Lexer::new(r#"b"""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Bytes(b) if b.is_empty()));
    }

    #[test]
    fn test_bytes_literal_uppercase() {
        let mut lexer = Lexer::new(r#"b"CAFEF00D""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Bytes(b) if b == &[0xca, 0xfe, 0xf0, 0x0d]));
    }

    #[test]
    fn test_bytes_literal_mixed_case() {
        let mut lexer = Lexer::new(r#"b"CaFe""#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Bytes(b) if b == &[0xca, 0xfe]));
    }

    #[test]
    fn test_bytes_literal_odd_length_error() {
        let mut lexer = Lexer::new(r#"b"abc""#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("odd number of hex digits"), "Error: {}", err);
    }

    #[test]
    fn test_bytes_literal_invalid_char_error() {
        let mut lexer = Lexer::new(r#"b"xyz""#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Invalid character"), "Error: {}", err);
    }

    #[test]
    fn test_bytes_literal_unterminated_error() {
        let mut lexer = Lexer::new(r#"b"cafe"#);
        let err = lexer.tokenize().unwrap_err();
        assert!(err.to_string().contains("Unterminated bytes literal"), "Error: {}", err);
    }

    #[test]
    fn test_bytes_literal_does_not_conflict_with_word() {
        // "bar" should parse as a word, not a bytes literal
        let mut lexer = Lexer::new("bar baz");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Word(w) if w == "bar"));
        assert!(matches!(&tokens[1].kind, TokenKind::Word(w) if w == "baz"));
    }

    // -------------------------------------------------------------------------
    // Fuzz regression tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fuzz_crash_unknown_chars_no_stack_overflow() {
        // Regression: fuzz_parse crash-e42e7ae2f5127519e7e60e87d1cbfbc2a5bf878d
        // Many consecutive unknown Unicode characters caused stack overflow
        // via recursive next_token() calls.
        let input = "\"0B\u{10}\u{3}#\"0BP\u{07FE}-----\u{061D}\u{07FE}\u{07FE}-----\u{061D}\u{3}#\"0B\u{10}\u{3}#\"0BP\u{07FE}-----\u{061D}\u{07FE}\u{07FE}-----\u{061D}\u{07FE}";
        let mut lexer = Lexer::new(input);
        // Should not stack overflow — may return Ok or Err, but must not crash
        let _ = lexer.tokenize();
    }

    #[test]
    fn test_fuzz_crash_timestamp_non_ascii_date() {
        // Regression: fuzz_parse crash-e5a60511db30059b55e7d7215b710fc36ec75dfb
        // Input "3313-32-$Ң..." matched timestamp heuristic at positions 4,7
        // but non-ASCII chars at positions 8-9 caused parse_iso8601 to panic
        // on byte slice `s[8..10]` cutting through multi-byte character Ң.
        let input = "02)3313-32-$\u{04A2}\u{1}\0\05";
        let mut lexer = Lexer::new(input);
        let _ = lexer.tokenize();
    }

    #[test]
    fn test_fuzz_crash_backslash_timestamp_non_ascii() {
        // Regression: fuzz_parse crash-785c8b3fbc203fc7279523e1eb5c57b2341de7ea
        // Backslashes + date pattern with non-ASCII Ԭ chars in date positions
        let input = "\\\\\u{1}\0\0\n\\\\\\\\\\\\)3313-32-\\\u{052D}\u{052D}:{Y:{Y\\\\\\\\\\\\\\\\\\\\\\3m\u{00AC}m\u{00C2}5\0\05";
        let mut lexer = Lexer::new(input);
        let _ = lexer.tokenize();
    }

    #[test]
    fn test_fuzz_crash_large_repeated_date_pattern() {
        // Regression: fuzz_parse crash-8684aafa13348eaeacbbd9a69ae6e02a57bc681e
        // 645-byte input with repeated date-like "3313-333-3332)" patterns
        // and non-ASCII chars interspersed. Must not panic.
        let input = "\"18]\")\"\"\"　]\t;=1]　]　　3333-333-3332)3313-33--33331333-333313T33302)3313-333-3333)3313-333-333-3332)33-133-3-333313;-3333)3333313T33302)3313-333-3333)3313-33332)33-3333)3333313T33302)3313-333-3333)3313-333-333-323)33-\t\n\t313T33302)3333-333-3332)3313-33--33331333-333313T33302)";
        let mut lexer = Lexer::new(input);
        let _ = lexer.tokenize();
    }

    #[test]
    fn test_fuzz_parse_iso8601_non_ascii_rejected() {
        // Verify parse_iso8601 rejects non-ASCII input gracefully
        assert!(parse_iso8601("2024-01-15T10:30:00Z").is_ok());
        assert!(parse_iso8601("3313-32-$\u{04A2}").is_err());
        assert!(parse_iso8601("2024-01-\u{052D}5").is_err());
        assert!(parse_iso8601("").is_err());
        assert!(parse_iso8601("short").is_err());
        // Month/day zero must be rejected (day-1 underflows u32 in days_from_epoch)
        assert!(parse_iso8601("2024-00-15T10:30:00Z").is_err());
        assert!(parse_iso8601("2024-01-00T10:30:00Z").is_err());
        assert!(parse_iso8601("2024-13-15T10:30:00Z").is_err());
        assert!(parse_iso8601("2024-00-00T10:30:00Z").is_err());
    }

    #[test]
    fn test_fuzz_timestamp_trailing_dot() {
        // Timestamp ending with just a dot and no fractional digits
        // Should return an error (not panic) since ".Z" has no digits after dot
        let mut lexer = Lexer::new("2024-01-15T10:30:00.Z");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_fuzz_crash_timestamp_long_fractional_no_overflow() {
        // Regression: fuzz_parse crash-bc25426e70a60ec5649726a4aa65e9f6776c90fb
        // Timestamp with 22 fractional digits caused 10i64.pow(19) overflow.
        // parse_iso8601 now caps fractional parsing to 3 digits.
        // Bogus dates parse without panic (no range validation):
        let _ = parse_iso8601("3230-32-33T33016656.6563311111111111111112");
        // Valid timestamp with many fractional digits should not overflow
        let result = parse_iso8601("2024-01-15T10:30:00.123456789012345678901234567890Z");
        assert!(result.is_ok());
        // Should parse as 123 ms (first 3 digits only)
        assert_eq!(result.unwrap().0 % 1000, 123);
    }

    #[test]
    fn test_fuzz_crash_bc25426e_full_parse_no_panic() {
        // Regression: crash-bc25426e — must not panic through TeaLeaf::parse
        let input = "\x00\x00\x00\x00\x00\x00\x00O\x00\x00\x00\x00\x00\x00\x00\x00\x0030-3\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x003232,\x00\x00\x001\x00\x00O\x00\x00\x00\x00\x00\x00\x00\x00\x0030-3\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x003232,\x00\x00\x00111111112\x00\n\x00\x00\x00\x00\x00\x003,3230-32-33T33016656.6563311111111111111112\x00\n\x00\x00\x00\x00\x00\x003,3230-32-33T33016656.65633111111111113323!:g";
        let _ = crate::TeaLeaf::parse(input); // Must not panic
    }

    #[test]
    fn test_fuzz_crash_multiline_multibyte_whitespace_dedent() {
        // Regression: fuzz_parse crash-834ac7a271d94cf87372e9a91a9137e81ff9316a
        // Multiline string with mixed whitespace: \u{0B} (1 byte) and \u{0085} (2 bytes).
        // Old byte-based dedent sliced at byte offset 1 into the 2-byte U+0085,
        // panicking on invalid character boundary.
        let input = "*\0\"\"\"\u{0B}J\n\n\n\u{0085}\u{0B}J\n\n\n\n\n\n\n\n\"\"\" \0\n\n\n\n\n\"\"\" \0\0";
        let mut lexer = Lexer::new(input);
        let _ = lexer.tokenize(); // Must not panic
    }

    #[test]
    fn test_multiline_string_multibyte_indent() {
        // Verify dedent works correctly with multi-byte whitespace characters
        // Both lines have 1 whitespace character of indent, but different byte widths
        let input = "\"\"\"\n\u{0085}A\n\u{0B}B\n\"\"\"";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        match &tokens[0].kind {
            TokenKind::String(s) => {
                assert_eq!(s, "A\nB", "Both lines should be dedented by 1 character");
            }
            other => panic!("Expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_many_unknown_chars_no_stack_overflow() {
        // Thousands of consecutive unknown characters should not stack overflow
        let input: String = std::iter::repeat('\u{07FE}').take(10_000).collect();
        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize().unwrap();
        // All unknown chars skipped, only Eof remains
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }
}
