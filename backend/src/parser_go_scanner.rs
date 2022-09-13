// https://golang.org/ref/spec#Lexical_elements

use crate::parser_go_token::{Position, Token};
use phf::{phf_map, Map};
use std::fmt;
use unicode_general_category::{get_general_category, GeneralCategory};

pub type Step<'a> = (Position<'a>, Token, &'a str);

#[derive(Debug)]
pub enum ScannerError {
    HexadecimalNotFound,
    OctalNotFound,
    UnterminatedComment,
    UnterminatedEscapedChar,
    UnterminatedRune,
    UnterminatedString,
    InvalidDirective,
}

impl std::error::Error for ScannerError {}

impl fmt::Display for ScannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scanner error: {:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, ScannerError>;

#[derive(Debug)]
pub struct Scanner<'a> {
    directory: &'a str,
    file: &'a str,
    buffer: &'a str,
    //
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    current_char: Option<char>,
    current_char_len: usize,
    //
    offset: usize,
    line: usize,
    column: usize,
    start_offset: usize,
    start_line: usize,
    start_column: usize,
    //
    hide_column: bool,
    insert_semi: bool,
    pending_line_info: Option<LineInfo<'a>>,
}

type LineInfo<'a> = (Option<&'a str>, usize, Option<usize>, bool);

impl<'a> Scanner<'a> {
    pub fn new(filename: &'a str, buffer: &'a str) -> Self {
        let (directory, file) = filename.rsplit_once('/').unwrap_or(("", filename));
        let mut s = Scanner {
            directory,
            file,
            buffer,
            //
            chars: buffer.chars().peekable(),
            current_char: None,
            current_char_len: 0,
            //
            offset: 0,
            line: 1,
            column: 1,
            start_offset: 0,
            start_line: 1,
            start_column: 1,
            //
            hide_column: false,
            insert_semi: false,
            pending_line_info: None,
        };
        s.next(); // read the first character
        s
    }

    #[allow(clippy::cognitive_complexity)] // Allow complex scan function
    pub fn scan(&mut self) -> Result<Step<'a>> {
        let insert_semi = self.insert_semi;
        self.insert_semi = false;

        while let Some(c) = self.current_char {
            self.reset_start();

            match c {
                ' ' | '\t' | '\r' => {
                    self.next();
                }

                '\n' => {
                    self.next();
                    if insert_semi {
                        return Ok((self.position(), Token::SEMICOLON, "\n"));
                    }
                }

                _ => break,
            }
        }

        if let Some(c) = self.current_char {
            match c {
                '+' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::ADD_ASSIGN, ""));
                        }
                        Some('+') => {
                            self.insert_semi = true;
                            self.next();
                            return Ok((self.position(), Token::INC, ""));
                        }
                        _ => return Ok((self.position(), Token::ADD, "")),
                    }
                }

                '-' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::SUB_ASSIGN, ""));
                        }
                        Some('-') => {
                            self.insert_semi = true;
                            self.next();
                            return Ok((self.position(), Token::DEC, ""));
                        }
                        _ => return Ok((self.position(), Token::SUB, "")),
                    }
                }

                '*' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::MUL_ASSIGN, ""));
                        }
                        _ => return Ok((self.position(), Token::MUL, "")),
                    }
                }

                '/' => match self.peek() {
                    Some('=') => {
                        self.next();
                        self.next();
                        return Ok((self.position(), Token::QUO_ASSIGN, ""));
                    }
                    Some('/') => {
                        if insert_semi {
                            return Ok((self.position(), Token::SEMICOLON, "\n"));
                        }
                        return self.scan_line_comment();
                    }
                    Some('*') => {
                        if insert_semi && self.find_line_end() {
                            return Ok((self.position(), Token::SEMICOLON, "\n"));
                        }
                        return self.scan_general_comment();
                    }
                    _ => {
                        self.next();
                        return Ok((self.position(), Token::QUO, ""));
                    }
                },

                '%' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::REM_ASSIGN, ""));
                        }
                        _ => return Ok((self.position(), Token::REM, "")),
                    }
                }

                '&' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::AND_ASSIGN, ""));
                        }
                        Some('&') => {
                            self.next();
                            return Ok((self.position(), Token::LAND, ""));
                        }
                        Some('^') => {
                            self.next();
                            match self.current_char {
                                Some('=') => {
                                    self.next();
                                    return Ok((self.position(), Token::AND_NOT_ASSIGN, ""));
                                }
                                _ => return Ok((self.position(), Token::AND_NOT, "")),
                            }
                        }
                        _ => return Ok((self.position(), Token::AND, "")),
                    }
                }

                '|' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::OR_ASSIGN, ""));
                        }
                        Some('|') => {
                            self.next();
                            return Ok((self.position(), Token::LOR, ""));
                        }
                        _ => return Ok((self.position(), Token::OR, "")),
                    }
                }

                '^' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::XOR_ASSIGN, ""));
                        }
                        _ => return Ok((self.position(), Token::XOR, "")),
                    }
                }

                '<' => {
                    self.next();
                    match self.current_char {
                        Some('<') => {
                            self.next();
                            match self.current_char {
                                Some('=') => {
                                    self.next();
                                    return Ok((self.position(), Token::SHL_ASSIGN, ""));
                                }
                                _ => return Ok((self.position(), Token::SHL, "")),
                            }
                        }
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::LEQ, ""));
                        }
                        Some('-') => {
                            self.next();
                            return Ok((self.position(), Token::ARROW, ""));
                        }
                        _ => return Ok((self.position(), Token::LSS, "")),
                    }
                }

                '>' => {
                    self.next();
                    match self.current_char {
                        Some('>') => {
                            self.next();
                            match self.current_char {
                                Some('=') => {
                                    self.next();
                                    return Ok((self.position(), Token::SHR_ASSIGN, ""));
                                }
                                _ => {
                                    return Ok((self.position(), Token::SHR, ""));
                                }
                            }
                        }
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::GEQ, ""));
                        }
                        _ => return Ok((self.position(), Token::GTR, "")),
                    }
                }

                ':' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::DEFINE, ""));
                        }
                        _ => return Ok((self.position(), Token::COLON, "")),
                    }
                }

                '!' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::NEQ, ""));
                        }
                        _ => return Ok((self.position(), Token::NOT, "")),
                    }
                }

                ',' => {
                    self.next();
                    return Ok((self.position(), Token::COMMA, ""));
                }

                '(' => {
                    self.next();
                    return Ok((self.position(), Token::LPAREN, ""));
                }

                ')' => {
                    self.insert_semi = true;
                    self.next();
                    return Ok((self.position(), Token::RPAREN, ""));
                }

                '[' => {
                    self.next();
                    return Ok((self.position(), Token::LBRACK, ""));
                }

                ']' => {
                    self.insert_semi = true;
                    self.next();
                    return Ok((self.position(), Token::RBRACK, ""));
                }

                '{' => {
                    self.next();
                    return Ok((self.position(), Token::LBRACE, ""));
                }

                '}' => {
                    self.insert_semi = true;
                    self.next();
                    return Ok((self.position(), Token::RBRACE, ""));
                }

                ';' => {
                    self.next();
                    return Ok((self.position(), Token::SEMICOLON, ";"));
                }

                '.' => {
                    self.next();
                    match self.current_char {
                        Some('0'..='9') => return self.scan_int_or_float_or_imag(true),
                        Some('.') => match self.peek() {
                            Some('.') => {
                                self.next();
                                self.next();
                                return Ok((self.position(), Token::ELLIPSIS, ""));
                            }
                            _ => return Ok((self.position(), Token::PERIOD, "")),
                        },
                        _ => return Ok((self.position(), Token::PERIOD, "")),
                    }
                }

                '=' => {
                    self.next();
                    match self.current_char {
                        Some('=') => {
                            self.next();
                            return Ok((self.position(), Token::EQL, ""));
                        }
                        _ => return Ok((self.position(), Token::ASSIGN, "")),
                    }
                }

                '0'..='9' => return self.scan_int_or_float_or_imag(false),
                '\'' => return self.scan_rune(),
                '"' => return self.scan_interpreted_string(),
                '`' => return self.scan_raw_string(),
                _ => return self.scan_pkg_or_keyword_or_ident(),
            };
        }

        self.reset_start();
        if insert_semi {
            Ok((self.position(), Token::SEMICOLON, "\n"))
        } else {
            Ok((self.position(), Token::EOF, ""))
        }
    }

    // https://golang.org/ref/spec#Keywords
    // https://golang.org/ref/spec#Identifiers
    fn scan_pkg_or_keyword_or_ident(&mut self) -> Result<Step<'a>> {
        self.next();

        while let Some(c) = self.current_char {
            if !(is_letter(c) || is_unicode_digit(c)) {
                break;
            }
            self.next()
        }

        let pos = self.position();
        let literal = self.literal();

        if literal.len() > 1 {
            if let Some(&token) = KEYWORDS.get(literal) {
                self.insert_semi = matches!(
                    token,
                    Token::BREAK | Token::CONTINUE | Token::FALLTHROUGH | Token::RETURN
                );
                return Ok((pos, token, literal));
            }
        }

        self.insert_semi = true;
        Ok((pos, Token::IDENT, literal))
    }

    // https://golang.org/ref/spec#Integer_literals
    // https://golang.org/ref/spec#Floating-point_literals
    // https://golang.org/ref/spec#Imaginary_literals
    fn scan_int_or_float_or_imag(&mut self, preceding_dot: bool) -> Result<Step<'a>> {
        self.insert_semi = true;

        let mut token = Token::INT;
        let mut digits = "_0123456789";
        let mut exp = "eE";

        if !preceding_dot {
            if matches!(self.current_char, Some('0')) {
                self.next();
                match self.current_char {
                    Some('b' | 'B') => {
                        digits = "_01";
                        exp = "";
                        self.next();
                    }
                    Some('o' | 'O') => {
                        digits = "_01234567";
                        exp = "";
                        self.next();
                    }
                    Some('x' | 'X') => {
                        digits = "_0123456789abcdefABCDEF";
                        exp = "pP";
                        self.next();
                    }
                    _ => {}
                };
            }

            while let Some(c) = self.current_char {
                if !digits.contains(c) {
                    break;
                }
                self.next();
            }
        }

        if preceding_dot || matches!(self.current_char, Some('.')) {
            token = Token::FLOAT;
            self.next();
            while let Some(c) = self.current_char {
                if !digits.contains(c) {
                    break;
                }
                self.next();
            }
        }

        if !exp.is_empty() {
            if let Some(c) = self.current_char {
                if exp.contains(c) {
                    token = Token::FLOAT;
                    self.next();
                    if matches!(self.current_char, Some('-' | '+')) {
                        self.next();
                    }
                    while let Some(c) = self.current_char {
                        if !matches!(c, '_' | '0'..='9') {
                            break;
                        }
                        self.next();
                    }
                }
            }
        }

        if matches!(self.current_char, Some('i')) {
            token = Token::IMAG;
            self.next();
        }

        Ok((self.position(), token, self.literal()))
    }

    // https://golang.org/ref/spec#Rune_literals
    fn scan_rune(&mut self) -> Result<Step<'a>> {
        self.insert_semi = true;
        self.next();

        match self.current_char {
            Some('\\') => self.require_escaped_char::<'\''>()?,
            Some(_) => self.next(),
            _ => return Err(ScannerError::UnterminatedRune),
        }

        if matches!(self.current_char, Some('\'')) {
            self.next();
            return Ok((self.position(), Token::CHAR, self.literal()));
        }

        Err(ScannerError::UnterminatedRune)
    }

    // https://golang.org/ref/spec#String_literals
    fn scan_interpreted_string(&mut self) -> Result<Step<'a>> {
        self.insert_semi = true;
        self.next();

        while let Some(c) = self.current_char {
            match c {
                '"' => {
                    self.next();
                    return Ok((self.position(), Token::STRING, self.literal()));
                }
                '\\' => self.require_escaped_char::<'"'>()?,
                _ => self.next(),
            }
        }

        Err(ScannerError::UnterminatedString)
    }

    // https://golang.org/ref/spec#String_literals
    fn scan_raw_string(&mut self) -> Result<Step<'a>> {
        self.insert_semi = true;
        self.next();

        while let Some(c) = self.current_char {
            match c {
                '`' => {
                    self.next();
                    return Ok((self.position(), Token::STRING, self.literal()));
                }
                _ => self.next(),
            }
        }

        Err(ScannerError::UnterminatedString)
    }

    // https://golang.org/ref/spec#Comments
    fn scan_general_comment(&mut self) -> Result<Step<'a>> {
        self.next();
        self.next();

        while let Some(c) = self.current_char {
            match c {
                '*' => {
                    self.next();
                    if matches!(self.current_char, Some('/')) {
                        self.next();

                        let pos = self.position();
                        let lit = self.literal();

                        // look for compiler directives
                        self.directive(&lit["/*".len()..lit.len() - "*/".len()], true)?;

                        return Ok((pos, Token::COMMENT, lit));
                    }
                }
                _ => self.next(),
            }
        }

        Err(ScannerError::UnterminatedComment)
    }

    // https://golang.org/ref/spec#Comments
    fn scan_line_comment(&mut self) -> Result<Step<'a>> {
        self.next();
        self.next();

        while let Some(c) = self.current_char {
            if is_newline(c) {
                break;
            }
            self.next();
        }

        let pos = self.position();
        let lit = self.literal();

        // look for compiler directives (at the beginning of line)
        if self.start_column == 1 {
            self.directive(lit["//".len()..].trim_end(), false)?;
        }

        Ok((pos, Token::COMMENT, self.literal()))
    }

    // https://pkg.go.dev/cmd/compile#hdr-Compiler_Directives
    fn directive(&mut self, input: &'a str, immediate: bool) -> Result<()> {
        if let Some(line_directive) = input.strip_prefix("line ") {
            self.pending_line_info = self.parse_line_directive(line_directive)?;
            if immediate {
                self.consume_pending_line_info();
            }
        }
        Ok(())
    }

    fn parse_line_directive(&mut self, line_directive: &'a str) -> Result<Option<LineInfo<'a>>> {
        if let Some((file, line)) = line_directive.rsplit_once(':') {
            let line = line.parse().map_err(|_| ScannerError::InvalidDirective)?;

            if let Some((file, l)) = file.rsplit_once(':') {
                if let Ok(l) = l.parse() {
                    //line :line:col
                    //line filename:line:col
                    /*line :line:col*/
                    /*line filename:line:col*/
                    let file = if !file.is_empty() { Some(file) } else { None };
                    let col = Some(line);
                    let line = l;
                    let hide_column = false;
                    return Ok(Some((file, line, col, hide_column)));
                }
            }

            //line :line
            //line filename:line
            /*line :line*/
            /*line filename:line*/
            Ok(Some((Some(file), line, None, true)))
        } else {
            Ok(None)
        }
    }

    const fn find_line_end(&self) -> bool {
        let buffer = self.buffer.as_bytes();
        let mut in_comment = true;

        let mut i = self.offset;
        let max = self.buffer.len();
        while i < max {
            let c = buffer[i] as char;

            if i < max - 1 {
                let n = buffer[i + 1] as char;

                if !in_comment && c == '/' && n == '/' {
                    return true;
                }

                if c == '/' && n == '*' {
                    i += 2;
                    in_comment = true;
                    continue;
                }

                if c == '*' && n == '/' {
                    i += 2;
                    in_comment = false;
                    continue;
                }
            }

            if is_newline(c) {
                return true;
            }

            if !in_comment && !matches!(c, ' ' | '\t' | '\r') {
                return false;
            }

            i += 1;
        }

        !in_comment
    }

    fn consume_pending_line_info(&mut self) {
        if let Some(line_info) = self.pending_line_info.take() {
            if let Some(file) = line_info.0 {
                self.file = file;
            }

            self.line = line_info.1;

            if let Some(column) = line_info.2 {
                self.column = column;
            }

            self.hide_column = line_info.3;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next(&mut self) {
        self.offset += self.current_char_len;
        self.column += self.current_char_len;
        let last_char = self.current_char;

        self.current_char = self.chars.next();
        if let Some(c) = self.current_char {
            self.current_char_len = c.len_utf8();
            if matches!(last_char, Some('\n')) {
                self.line += 1;
                self.column = 1;
                self.consume_pending_line_info();
            }
        } else {
            self.current_char_len = 0
        }
    }

    const fn position(&self) -> Position<'a> {
        Position {
            directory: self.directory,
            file: self.file,
            offset: self.start_offset,
            line: self.start_line,
            column: if self.hide_column {
                0
            } else {
                self.start_column
            },
        }
    }

    fn reset_start(&mut self) {
        self.start_offset = self.offset;
        self.start_line = self.line;
        self.start_column = self.column;
    }

    fn literal(&self) -> &'a str {
        &self.buffer[self.start_offset..self.offset]
    }

    fn require_escaped_char<const DELIM: char>(&mut self) -> Result<()> {
        self.next();

        let c = self
            .current_char
            .ok_or(ScannerError::UnterminatedEscapedChar)?;

        // TODO: move this to the match when const generics can be referenced in patterns
        if c == DELIM {
            self.next();
            return Ok(());
        }

        match c {
            'a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '\\' => self.next(),
            'x' => {
                self.next();
                self.require_hex_digits::<2>()?
            }
            'u' => {
                self.next();
                self.require_hex_digits::<4>()?;
            }
            'U' => {
                self.next();
                self.require_hex_digits::<8>()?;
            }
            '0'..='7' => self.require_octal_digits::<3>()?,
            _ => return Err(ScannerError::UnterminatedEscapedChar),
        }

        Ok(())
    }

    fn require_octal_digits<const COUNT: usize>(&mut self) -> Result<()> {
        for _ in 0..COUNT {
            let c = self.current_char.ok_or(ScannerError::OctalNotFound)?;

            if !is_octal_digit(c) {
                return Err(ScannerError::OctalNotFound);
            }

            self.next();
        }

        Ok(())
    }

    fn require_hex_digits<const COUNT: usize>(&mut self) -> Result<()> {
        for _ in 0..COUNT {
            let c = self.current_char.ok_or(ScannerError::HexadecimalNotFound)?;

            if !is_hex_digit(c) {
                return Err(ScannerError::HexadecimalNotFound);
            }

            self.next();
        }

        Ok(())
    }
}

impl<'a> IntoIterator for Scanner<'a> {
    type Item = Result<Step<'a>>;
    type IntoIter = IntoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter::new(self)
    }
}

pub struct IntoIter<'a> {
    scanner: Scanner<'a>,
    done: bool,
}

impl<'a> IntoIter<'a> {
    const fn new(scanner: Scanner<'a>) -> Self {
        Self { scanner, done: false }
    }
}

impl<'a> Iterator for IntoIter<'a> {
    type Item = Result<Step<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.scanner.scan() {
            Ok((pos, tok, lit)) => {
                if tok == Token::EOF {
                    self.done = true;
                }
                Some(Ok((pos, tok, lit)))
            }
            Err(err) => {
                self.done = true;
                Some(Err(err))
            }
        }
    }
}

// https://golang.org/ref/spec#Letters_and_digits

fn is_letter(c: char) -> bool {
    c == '_' || is_unicode_letter(c)
}

//const fn is_decimal_digit(c: char) -> bool {
//matches!(c, '0'..='9')
//}

//const fn is_binary_digit(c: char) -> bool {
//matches!(c, '0'..='1')
//}

const fn is_octal_digit(c: char) -> bool {
    matches!(c, '0'..='7')
}

const fn is_hex_digit(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='F' | 'a'..='f')
}

// https://golang.org/ref/spec#Characters

const fn is_newline(c: char) -> bool {
    c == '\n'
}

//const fn is_unicode_char(c: char) -> bool {
//c != '\n'
//}

fn is_unicode_letter(c: char) -> bool {
    matches!(
        get_general_category(c),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
    )
}

fn is_unicode_digit(c: char) -> bool {
    get_general_category(c) == GeneralCategory::DecimalNumber
}

// https://golang.org/ref/spec#Keywords

static KEYWORDS: Map<&'static str, Token> = phf_map! {
  "break" => Token::BREAK,
  "case" => Token::CASE,
  "chan" => Token::CHAN,
  "const" => Token::CONST,
  "continue" => Token::CONTINUE,

  "default" => Token::DEFAULT,
  "defer" => Token::DEFER,
  "else" => Token::ELSE,
  "fallthrough" => Token::FALLTHROUGH,
  "for" => Token::FOR,

  "func" => Token::FUNC,
  "go" => Token::GO,
  "goto" => Token::GOTO,
  "if" => Token::IF,
  "import" => Token::IMPORT,

  "interface" => Token::INTERFACE,
  "map" => Token::MAP,
  "package" => Token::PACKAGE,
  "range" => Token::RANGE,
  "return" => Token::RETURN,

  "select" => Token::SELECT,
  "struct" => Token::STRUCT,
  "switch" => Token::SWITCH,
  "type" => Token::TYPE,
  "var" => Token::VAR,
};

#[cfg(test)]
mod tests {
    use super::Scanner;

    #[test] // fuzz
    fn it_should_return_an_error_on_missing_line_number() {
        let input = "/*line :*/";
        let mut out: Vec<_> = Scanner::new(file!(), input).into_iter().collect();
        assert!(out.pop().unwrap().is_err());
    }
}
