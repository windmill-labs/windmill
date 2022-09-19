// https://cs.opensource.google/go/go/+/refs/tags/go1.17.2:src/go/token/token.go

#![allow(non_camel_case_types)] // For consistency with the Go tokens

use std::fmt;

#[derive(Clone, Copy, Debug, Default)]
pub struct Position<'a> {
    pub directory: &'a str,
    pub file: &'a str,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl<'a> fmt::Display for Position<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.file.is_empty() {
            write!(f, ":{}:{}", self.line, self.column)
        } else if self.file.starts_with('/') {
            write!(f, "{}:{}:{}", self.file, self.line, self.column)
        } else {
            write!(
                f,
                "{}/{}:{}:{}",
                self.directory, self.file, self.line, self.column
            )
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Token {
    EOF,
    COMMENT,

    IDENT,  // main
    INT,    // 12345
    FLOAT,  // 123.45
    IMAG,   // 123.45i
    CHAR,   // 'a'
    STRING, // "abc"

    ADD, // +
    SUB, // -
    MUL, // *
    QUO, // /
    REM, // %

    AND,     // &
    OR,      // |
    XOR,     // ^
    SHL,     // <<
    SHR,     // >>
    AND_NOT, // &^

    ADD_ASSIGN, // +=
    SUB_ASSIGN, // -=
    MUL_ASSIGN, // *=
    QUO_ASSIGN, // /=
    REM_ASSIGN, // %=

    AND_ASSIGN,     // &=
    OR_ASSIGN,      // |=
    XOR_ASSIGN,     // ^=
    SHL_ASSIGN,     // <<=
    SHR_ASSIGN,     // >>=
    AND_NOT_ASSIGN, // &^=

    LAND,  // &&
    LOR,   // ||
    ARROW, // <-
    INC,   // ++
    DEC,   // --

    EQL,    // ==
    LSS,    // <
    GTR,    // >
    ASSIGN, // =
    NOT,    // !

    NEQ,      // !=
    LEQ,      // <=
    GEQ,      // >=
    DEFINE,   // :=
    ELLIPSIS, // ...

    LPAREN, // (
    LBRACK, // [
    LBRACE, // {
    COMMA,  // ,
    PERIOD, // .

    RPAREN,    // )
    RBRACK,    // ]
    RBRACE,    // }
    SEMICOLON, // ;
    COLON,     // :

    BREAK,
    CASE,
    CHAN,
    CONST,
    CONTINUE,

    DEFAULT,
    DEFER,
    ELSE,
    FALLTHROUGH,
    FOR,

    FUNC,
    GO,
    GOTO,
    IF,
    IMPORT,

    INTERFACE,
    MAP,
    PACKAGE,
    RANGE,
    RETURN,

    SELECT,
    STRUCT,
    SWITCH,
    TYPE,
    VAR,
}

impl Token {
    pub const fn is_assign_op(&self) -> bool {
        use Token::*;
        matches!(
            self,
            ADD_ASSIGN
                | SUB_ASSIGN
                | MUL_ASSIGN
                | QUO_ASSIGN
                | REM_ASSIGN
                | AND_ASSIGN
                | OR_ASSIGN
                | XOR_ASSIGN
                | SHL_ASSIGN
                | SHR_ASSIGN
                | AND_NOT_ASSIGN
        )
    }

    // https://go.dev/ref/spec#Operator_precedence
    pub fn precedence(&self) -> u8 {
        use Token::*;
        match self {
            MUL | QUO | REM | SHL | SHR | AND | AND_NOT => 5,
            ADD | SUB | OR | XOR => 4,
            EQL | NEQ | LSS | LEQ | GTR | GEQ => 3,
            LAND => 2,
            LOR => 1,
            _ => unreachable!(
                "precedence() is only supported for binary operators, called with: {:?}",
                self
            ),
        }
    }

    pub const fn lowest_precedence() -> u8 {
        0
    }
}

impl From<&Token> for &'static str {
    fn from(token: &Token) -> Self {
        use Token::*;

        match token {
            EOF => "EOF",
            COMMENT => "COMMENT",

            IDENT => "IDENT",
            INT => "INT",
            FLOAT => "FLOAT",
            IMAG => "IMAG",
            CHAR => "CHAR",
            STRING => "STRING",

            ADD => "+",
            SUB => "-",
            MUL => "*",
            QUO => "/",
            REM => "%",

            AND => "&",
            OR => "|",
            XOR => "^",
            SHL => "<<",
            SHR => ">>",
            AND_NOT => "&^",

            ADD_ASSIGN => "+=",
            SUB_ASSIGN => "-=",
            MUL_ASSIGN => "*=",
            QUO_ASSIGN => "/=",
            REM_ASSIGN => "%=",

            AND_ASSIGN => "&=",
            OR_ASSIGN => "|=",
            XOR_ASSIGN => "^=",
            SHL_ASSIGN => "<<=",
            SHR_ASSIGN => ">>=",
            AND_NOT_ASSIGN => "&^=",

            LAND => "&&",
            LOR => "||",
            ARROW => "<-",
            INC => "++",
            DEC => "--",

            EQL => "==",
            LSS => "<",
            GTR => ">",
            ASSIGN => "=",
            NOT => "!",

            NEQ => "!=",
            LEQ => "<=",
            GEQ => ">=",
            DEFINE => ":=",
            ELLIPSIS => "...",

            LPAREN => "(",
            LBRACK => "[",
            LBRACE => "{",
            COMMA => ",",
            PERIOD => ".",

            RPAREN => ")",
            RBRACK => "]",
            RBRACE => "}",
            SEMICOLON => ";",
            COLON => ":",

            BREAK => "break",
            CASE => "case",
            CHAN => "chan",
            CONST => "const",
            CONTINUE => "continue",

            DEFAULT => "default",
            DEFER => "defer",
            ELSE => "else",
            FALLTHROUGH => "fallthrough",
            FOR => "for",

            FUNC => "func",
            GO => "go",
            GOTO => "goto",
            IF => "if",
            IMPORT => "import",

            INTERFACE => "interface",
            MAP => "map",
            PACKAGE => "package",
            RANGE => "range",
            RETURN => "return",

            SELECT => "select",
            STRUCT => "struct",
            SWITCH => "switch",
            TYPE => "type",
            VAR => "var",
        }
    }
}
