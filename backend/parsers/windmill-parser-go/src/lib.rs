#![allow(non_snake_case)] // TODO: switch to parse_* function naming

mod parser_go_ast;
mod parser_go_scanner;
mod parser_go_token;

use itertools::Itertools;

use parser_go_ast::{Decl, Expr};
use parser_go_ast::{FieldList, Ident, StructType};
use parser_go_token::{Position, Token};
use std::fmt;
use windmill_common::error::to_anyhow;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};

pub fn parse_go_sig(code: &str) -> windmill_common::error::Result<MainArgSignature> {
    let filtered_code = filter_non_main(code);
    let file = parse_file("main.go", &filtered_code).map_err(to_anyhow)?;
    if let Some(Decl::FuncDecl(func)) = file.decls.first() {
        let args = func
            .type_
            .params
            .list
            .iter()
            .map(|param| {
                let (otyp, typ) = get_type(param);
                Arg { name: get_name(param), otyp, typ, default: None, has_default: false }
            })
            .collect_vec();
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(windmill_common::error::Error::BadRequest(
            "no main function found".to_string(),
        ))
    }
}

fn get_type(param: &parser_go_ast::Field) -> (Option<String>, Typ) {
    let (otyp, typ) = &param
        .type_
        .as_ref()
        .map(|typ| parse_go_typ(typ))
        .unwrap_or_else(|| (None, Typ::Unknown));
    (otyp.clone(), typ.clone())
}

fn get_name(param: &parser_go_ast::Field) -> String {
    param
        .names
        .as_ref()
        .and_then(|x| x.first().map(|y| y.name.to_string()))
        .unwrap_or_else(|| "".to_string())
}

fn parse_go_typ(typ: &parser_go_ast::Expr) -> (Option<String>, Typ) {
    match typ {
        Expr::Ident(Ident { name, .. }) => (
            Some((*name).to_string()),
            match *name {
                "int" => Typ::Int,
                "int16" => Typ::Int,
                "int32" => Typ::Int,
                "int64" => Typ::Int,
                "string" => Typ::Str(None),
                "bool" => Typ::Bool,
                _ => Typ::Unknown,
            },
        ),
        Expr::ArrayType(array_type) => {
            let (inner_otyp, inner_typ) = parse_go_typ(&*array_type.elt);
            (
                inner_otyp.map(|x| format!("[]{x}")),
                Typ::List(Box::new(inner_typ)),
            )
        }
        Expr::StructType(StructType { fields: Some(FieldList { list, .. }), .. }) => {
            let (otyps, typs): (Vec<String>, Vec<ObjectProperty>) = list
                .iter()
                .map(|field| {
                    let json_tag = field
                        .tag
                        .as_ref()
                        .and_then(|x| x.value.strip_prefix("`json:\""))
                        .and_then(|x| x.strip_suffix("\"`"))
                        .and_then(|x| x.split(',').last().map(|x| x.to_string()));
                    let (otyp, typ) = get_type(field);
                    let name = get_name(field);
                    let key = json_tag.unwrap_or_else(|| name.to_string());
                    (
                        format!("{name} {} `json:\"{key}\"`", otyp_to_string(otyp)),
                        ObjectProperty { key, typ: Box::new(typ) },
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .unzip();
            (
                Some(format!(
                    "struct {{ {} }}",
                    otyps.iter().join("; ").to_string()
                )),
                Typ::Object(typs),
            )
        }
        Expr::InterfaceType(_) => (Some("interface{}".to_string()), Typ::Object(vec![])),
        Expr::MapType(_) => (
            Some("map[string]interface{}".to_string()),
            Typ::Object(vec![]),
        ),
        _ => (None, Typ::Unknown),
    }
}

pub fn otyp_to_string(otyp: Option<String>) -> String {
    otyp.unwrap_or_else(|| "interface{}".to_string())
}

#[cfg(test)]
mod tests {

    use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};

    use super::*;

    #[test]
    fn test_parse_go_sig() -> anyhow::Result<()> {
        let code = r#"

package main

import "fmt"

func main(x int, y string, z bool, l []string, o struct { Name string `json:"name"` }, n interface{}, m map[string]interface{}) {
    fmt.Println("hello world")
}    

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_go_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("int".to_string()),
                        name: "x".to_string(),
                        typ: Typ::Int,
                        has_default: false,
                        default: None
                    },
                    Arg {
                        otyp: Some("string".to_string()),
                        name: "y".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("bool".to_string()),
                        name: "z".to_string(),
                        typ: Typ::Bool,
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("[]string".to_string()),
                        name: "l".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("struct { Name string `json:\"name\"` }".to_string()),
                        name: "o".to_string(),
                        typ: Typ::Object(vec![ObjectProperty {
                            key: "name".to_string(),
                            typ: Box::new(Typ::Str(None))
                        },]),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("interface{}".to_string()),
                        name: "n".to_string(),
                        typ: Typ::Object(vec![]),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("map[string]interface{}".to_string()),
                        name: "m".to_string(),
                        typ: Typ::Object(vec![]),
                        default: None,
                        has_default: false
                    },
                ]
            }
        );

        Ok(())
    }
}

fn filter_non_main(code: &str) -> String {
    const FUNC_MAIN: &str = "func main(";

    let mut filtered_code = String::new();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(FUNC_MAIN) {
            filtered_code += FUNC_MAIN;
            remaining += line.strip_prefix(FUNC_MAIN).unwrap();
            remaining += &code_iter.join("\n");
            break;
        }
    }
    if filtered_code.is_empty() {
        return String::new();
    }
    let mut chars = remaining.chars();
    let mut open_parens = 1;

    while let Some(c) = chars.next() {
        if c == '(' {
            open_parens += 1;
        } else if c == ')' {
            open_parens -= 1;
        }
        filtered_code.push(c);
        if open_parens == 0 {
            break;
        }
    }

    filtered_code.push_str("{}");
    return filtered_code;
}

#[derive(Debug)]
pub enum ParserError {
    ScannerError(parser_go_scanner::ScannerError),
    UnexpectedEndOfFile,
    UnexpectedToken,
    UnexpectedTokenAt { at: String, token: Token, literal: String },
}

impl std::error::Error for ParserError {}

impl From<parser_go_scanner::ScannerError> for ParserError {
    fn from(e: parser_go_scanner::ScannerError) -> Self {
        Self::ScannerError(e)
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parser error: {:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, ParserError>;

trait ResultExt<T> {
    fn required(self) -> Result<T>;
}

impl<T> ResultExt<T> for Result<Option<T>> {
    fn required(self) -> Result<T> {
        self.and_then(|node| node.map_or(Err(ParserError::UnexpectedToken), |node| Ok(node)))
    }
}

pub fn parse_file<'a>(filename: &'a str, buffer: &'a str) -> Result<parser_go_ast::File<'a>> {
    let parser_go_scanner = parser_go_scanner::Scanner::new(filename, buffer);
    let mut parser = Parser::new(parser_go_scanner);
    parser.next()?;
    parser.SourceFile().required().map_err(|err| match err {
        ParserError::UnexpectedToken => ParserError::UnexpectedTokenAt {
            at: parser.current_step.0.to_string(),
            token: parser.current_step.1,
            literal: parser.current_step.2.to_owned(),
        },
        err => err,
    })
}

struct Parser<'parser_go_scanner> {
    steps: parser_go_scanner::IntoIter<'parser_go_scanner>,
    current_step: parser_go_scanner::Step<'parser_go_scanner>,
    expr_level: isize,
}

impl<'parser_go_scanner> Parser<'parser_go_scanner> {
    pub fn new(parser_go_scanner: parser_go_scanner::Scanner<'parser_go_scanner>) -> Self {
        Self {
            steps: parser_go_scanner.into_iter(),
            current_step: (Position::default(), Token::EOF, ""),
            expr_level: -1,
        }
    }

    // SourceFile = PackageClause ";" { ImportDecl ";" } { TopLevelDecl ";" } .
    fn SourceFile(&mut self) -> Result<Option<parser_go_ast::File<'parser_go_scanner>>> {
        let mut out = parser_go_ast::File { decls: vec![] };

        while let Some(top_level_decl) = self.TopLevelDecl()? {
            self.token(Token::SEMICOLON).required()?;
            out.decls.push(top_level_decl);
        }

        self.token(Token::EOF).required()?;

        Ok(Some(out))
    }

    // TopLevelDecl = Declaration | FunctionDecl | MethodDecl .
    fn TopLevelDecl(&mut self) -> Result<Option<parser_go_ast::Decl<'parser_go_scanner>>> {
        use Token::*;
        Ok(match self.current_step.1 {
            FUNC => Some(parser_go_ast::Decl::FuncDecl(
                self.FunctionDecl_or_MethodDecl().required()?,
            )),
            _ => None,
        })
    }

    // IdentifierList = identifier { "," identifier } .
    fn IdentifierList(&mut self) -> Result<Option<Vec<parser_go_ast::Ident<'parser_go_scanner>>>> {
        let mut out = match self.identifier()? {
            Some(v) => vec![v],
            None => return Ok(None),
        };

        while self.token(Token::COMMA)?.is_some() {
            out.push(self.identifier().required()?);
        }

        Ok(Some(out))
    }

    // ExpressionList = Expression { "," Expression } .
    fn ExpressionList(&mut self) -> Result<Option<Vec<parser_go_ast::Expr<'parser_go_scanner>>>> {
        let mut out = match self.Expression()? {
            Some(v) => vec![v],
            None => return Ok(None),
        };

        while self.token(Token::COMMA)?.is_some() {
            out.push(self.Expression().required()?);
        }

        Ok(Some(out))
    }

    // Expression = UnaryExpr | Expression binary_op Expression .
    fn Expression(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let unary_expr = match self.UnaryExpr()? {
            Some(v) => v,
            None => return Ok(None),
        };

        self.expression(unary_expr, Token::lowest_precedence())
    }

    // https://en.wikipedia.org/wiki/Operator-precedence_parser
    fn expression(
        &mut self,
        mut lhs: parser_go_ast::Expr<'parser_go_scanner>,
        min_precedence: u8,
    ) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        while let Some(op) = self.get_binary_op(min_precedence)? {
            self.next()?;

            let mut rhs = self.UnaryExpr().required()?;
            while self.get_binary_op(op.1.precedence() + 1)?.is_some() {
                rhs = self.expression(rhs, op.1.precedence() + 1).required()?;
            }

            lhs = parser_go_ast::Expr::BinaryExpr(parser_go_ast::BinaryExpr {
                x: Box::new(lhs),
                op_pos: op.0,
                op: op.1,
                y: Box::new(rhs),
            });
        }

        Ok(Some(lhs))
    }

    // UnaryExpr = PrimaryExpr | unary_op UnaryExpr .
    fn UnaryExpr(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        if let Some(op) = self.unary_op()? {
            let x = Box::new(self.UnaryExpr().required()?);
            let expr = if op.1 == Token::MUL {
                parser_go_ast::Expr::StarExpr(parser_go_ast::StarExpr { star: op.0, x })
            } else {
                parser_go_ast::Expr::UnaryExpr(parser_go_ast::UnaryExpr {
                    op: op.1,
                    op_pos: op.0,
                    x,
                })
            };
            return Ok(Some(expr));
        }

        self.PrimaryExpr()
    }

    // PrimaryExpr =
    //         Operand |
    //         Conversion |
    //         MethodExpr |
    //         PrimaryExpr Selector |
    //         PrimaryExpr Index |
    //         PrimaryExpr Slice |
    //         PrimaryExpr TypeAssertion |
    //         PrimaryExpr Arguments .
    fn PrimaryExpr(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let mut primary_expr = match self.Operand()? {
            Some(v) => v,
            None => return Ok(None),
        };

        loop {
            match self.current_step.1 {
                Token::PERIOD => {
                    primary_expr = self.Selector_or_TypeAssertion(primary_expr).required()?;
                }
                Token::LBRACK => {
                    primary_expr = self.Index_or_Slice(primary_expr).required()?;
                }
                Token::LPAREN => {
                    primary_expr = self.Arguments(primary_expr).required()?;
                }
                Token::LBRACE if self.expr_level >= 0 => {
                    unimplemented!("composite literal");
                }
                _ => break,
            }
        }

        Ok(Some(primary_expr))
    }

    // Selector      = "." identifier .
    // TypeAssertion = "." "(" Type ")" .
    fn Selector_or_TypeAssertion(
        &mut self,
        x: parser_go_ast::Expr<'parser_go_scanner>,
    ) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        if self.token(Token::PERIOD)?.is_none() {
            return Ok(None);
        }

        if let Some(lparen) = self.token(Token::LPAREN)? {
            let type_ = self.Type().required()?;
            let rparen = self.token(Token::RPAREN).required()?;
            return Ok(Some(parser_go_ast::Expr::TypeAssertExpr(
                parser_go_ast::TypeAssertExpr {
                    x: Box::new(x),
                    lparen: lparen.0,
                    type_: Box::new(type_),
                    rparen: rparen.0,
                },
            )));
        }

        Ok(Some(parser_go_ast::Expr::SelectorExpr(
            parser_go_ast::SelectorExpr { x: Box::new(x), sel: self.identifier().required()? },
        )))
    }

    // Index = "[" Expression "]" .
    // Slice = "[" [ Expression ] ":" [ Expression ] "]" |
    //         "[" [ Expression ] ":" Expression ":" Expression "]" .
    fn Index_or_Slice(
        &mut self,
        x: parser_go_ast::Expr<'parser_go_scanner>,
    ) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let lbrack = match self.token(Token::LBRACK)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let low = if let Some(low) = self.Expression()? {
            if let Some(rbrack) = self.token(Token::RBRACK)? {
                return Ok(Some(parser_go_ast::Expr::IndexExpr(
                    parser_go_ast::IndexExpr {
                        x: Box::new(x),
                        lbrack: lbrack.0,
                        index: Box::new(low),
                        rbrack: rbrack.0,
                    },
                )));
            }
            Some(low)
        } else {
            None
        };

        self.token(Token::COLON).required()?;

        let high = if let Some(high) = self.Expression()? {
            if self.token(Token::COLON)?.is_some() {
                let max = self.Expression().required()?;
                let rbrack = self.token(Token::RBRACK).required()?;
                return Ok(Some(parser_go_ast::Expr::SliceExpr(
                    parser_go_ast::SliceExpr {
                        x: Box::new(x),
                        lbrack: lbrack.0,
                        low: low.map(Box::new),
                        high: Some(Box::new(high)),
                        max: Some(Box::new(max)),
                        slice3: true,
                        rbrack: rbrack.0,
                    },
                )));
            }
            Some(high)
        } else {
            None
        };
        let rbrack = self.token(Token::RBRACK).required()?;

        Ok(Some(parser_go_ast::Expr::SliceExpr(
            parser_go_ast::SliceExpr {
                x: Box::new(x),
                lbrack: lbrack.0,
                low: low.map(Box::new),
                high: high.map(Box::new),
                max: None,
                slice3: false,
                rbrack: rbrack.0,
            },
        )))
    }

    // Arguments = "(" [ ( ExpressionList | Type [ "," ExpressionList ] ) [ "..." ] [ "," ] ] ")" .
    fn Arguments(
        &mut self,
        x: parser_go_ast::Expr<'parser_go_scanner>,
    ) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let lparen = match self.token(Token::LPAREN)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let mut args = if let Some(exprs) = self.ExpressionList()? {
            exprs
        } else if let Some(type_) = self.Type()? {
            vec![type_]
        } else {
            vec![]
        };

        if self.token(Token::COMMA)?.is_some() {
            let mut exprs = self.ExpressionList().required()?;
            args.append(&mut exprs);
        }

        let ellipsis = if !args.is_empty() {
            let ellipsis = self.token(Token::ELLIPSIS)?;
            self.token(Token::COMMA)?;
            ellipsis
        } else {
            None
        };

        let rparen = self.token(Token::RPAREN).required()?;

        Ok(Some(parser_go_ast::Expr::CallExpr(
            parser_go_ast::CallExpr {
                fun: Box::new(x),
                lparen: lparen.0,
                args: Some(args),
                ellipsis: ellipsis.map(|(pos, _, _)| pos),
                rparen: rparen.0,
            },
        )))
    }

    // Operand = Literal | OperandName | "(" Expression ")" .
    // Literal = BasicLit | CompositeLit | FunctionLit .
    // OperandName = identifier | QualifiedIdent .
    fn Operand(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        use Token::*;
        Ok(match self.current_step.1 {
            IDENT => Some(parser_go_ast::Expr::Ident(self.identifier().required()?)),
            INT | FLOAT | IMAG | CHAR | STRING => {
                Some(parser_go_ast::Expr::BasicLit(self.BasicLit().required()?))
            }
            LPAREN => {
                let lparen = self.token(Token::LPAREN).required()?;
                let expr = self.Expression().required()?;
                let rparen = self.token(Token::RPAREN).required()?;
                return Ok(Some(parser_go_ast::Expr::ParenExpr(
                    parser_go_ast::ParenExpr {
                        lparen: lparen.0,
                        x: Box::new(expr),
                        rparen: rparen.0,
                    },
                )));
            }
            FUNC => Some(parser_go_ast::Expr::FuncLit(self.FunctionLit().required()?)),
            _ => self.CompositeLit()?.map(parser_go_ast::Expr::CompositeLit),
        })
    }

    // CompositeLit = LiteralType LiteralValue .
    // LiteralValue = "{" [ ElementList [ "," ] ] "}" .
    // ElementList  = KeyedElement { "," KeyedElement } .
    fn CompositeLit(&mut self) -> Result<Option<parser_go_ast::CompositeLit<'parser_go_scanner>>> {
        let type_ = match self.LiteralType()? {
            Some(v) => v,
            None => return Ok(None),
        };

        let lbrace = self.token(Token::LBRACE).required()?;

        let mut elts = self.KeyedElement()?.map(|elt| vec![elt]);
        if let Some(elts) = elts.as_mut() {
            while self.token(Token::COMMA)?.is_some() {
                if let Some(k) = self.KeyedElement()? {
                    elts.push(k);
                } else {
                    break;
                }
            }
        }

        let rbrace = self.token(Token::RBRACE).required()?;

        Ok(Some(parser_go_ast::CompositeLit {
            type_: Box::new(type_),
            lbrace: lbrace.0,
            elts,
            rbrace: rbrace.0,
            incomplete: false,
        }))
    }

    // LiteralType = StructType | ArrayType | "[" "..." "]" ElementType |
    //               SliceType | MapType | TypeName .
    fn LiteralType(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        Ok(match self.current_step.1 {
            Token::STRUCT => Some(parser_go_ast::Expr::StructType(
                self.StructType().required()?,
            )),
            Token::LBRACK => Some(parser_go_ast::Expr::ArrayType(
                self.ArrayType_or_SliceType::<true>().required()?,
            )),
            Token::MAP => Some(parser_go_ast::Expr::MapType(self.MapType().required()?)),
            Token::IDENT => Some(self.TypeName().required()?),
            _ => None,
        })
    }

    // KeyedElement = [ Key ":" ] Element .
    // Key          = FieldName | Expression | LiteralValue .
    // FieldName    = identifier .
    // Element      = Expression | LiteralValue .
    fn KeyedElement(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let key = match self.Expression()? {
            Some(v) => v,
            None => return Ok(None),
        };

        if let Some(colon) = self.token(Token::COLON)? {
            let value = self.Expression().required()?;
            return Ok(Some(parser_go_ast::Expr::KeyValueExpr(
                parser_go_ast::KeyValueExpr {
                    key: Box::new(key),
                    colon: colon.0,
                    value: Box::new(value),
                },
            )));
        }

        Ok(Some(key))
    }

    // FunctionLit = "func" Signature FunctionBody .
    fn FunctionLit(&mut self) -> Result<Option<parser_go_ast::FuncLit<'parser_go_scanner>>> {
        let func = match self.token(Token::FUNC)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let type_ = self.Signature(Some(func.0)).required()?;
        let body = self.FunctionBody().required()?;

        Ok(Some(parser_go_ast::FuncLit { type_, body }))
    }

    // BasicLit = int_lit | float_lit | imaginary_lit | rune_lit | string_lit .
    fn BasicLit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        Ok(match self.current_step.1 {
            Token::INT => Some(self.int_lit().required()?),
            Token::FLOAT => Some(self.float_lit().required()?),
            Token::IMAG => Some(self.imaginary_lit().required()?),
            Token::CHAR => Some(self.rune_lit().required()?),
            Token::STRING => Some(self.string_lit().required()?),
            _ => None,
        })
    }

    // Type = TypeName | TypeLit | "(" Type ")" .
    fn Type(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        if self.token(Token::LPAREN)?.is_some() {
            let type_ = self.Type().required()?;
            self.token(Token::RPAREN).required()?;
            return Ok(Some(type_));
        }

        if let Some(type_name) = self.TypeName()? {
            return Ok(Some(type_name));
        }

        if let Some(type_lit) = self.TypeLit()? {
            return Ok(Some(type_lit));
        }

        Ok(None)
    }

    // TypeName = identifier | QualifiedIdent .
    fn TypeName(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        self.identifier_or_QualifiedIdent()
    }

    // TypeLit = ArrayType | StructType | PointerType | FunctionType | InterfaceType |
    //           SliceType | MapType | ChannelType .
    fn TypeLit(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        Ok(match self.current_step.1 {
            Token::LBRACK => Some(parser_go_ast::Expr::ArrayType(
                self.ArrayType_or_SliceType::<false>().required()?,
            )),
            Token::STRUCT => Some(parser_go_ast::Expr::StructType(
                self.StructType().required()?,
            )),
            Token::MUL => Some(parser_go_ast::Expr::StarExpr(
                self.PointerType().required()?,
            )),
            // TODO: FunctionType
            Token::INTERFACE => Some(parser_go_ast::Expr::InterfaceType(
                self.InterfaceType().required()?,
            )),
            Token::MAP => Some(parser_go_ast::Expr::MapType(self.MapType().required()?)),
            Token::CHAN => Some(parser_go_ast::Expr::ChanType(
                self.ChannelType().required()?,
            )),
            _ => None,
        })
    }

    // ArrayType   = "[" ArrayLength "]" ElementType .
    // ArrayLength = Expression .
    // SliceType   = "[" "]" ElementType .
    fn ArrayType_or_SliceType<const ELLIPSIS: bool>(
        &mut self,
    ) -> Result<Option<parser_go_ast::ArrayType<'parser_go_scanner>>> {
        let lbrack = match self.token(Token::LBRACK)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let len = if ELLIPSIS {
            if let Some(ellipsis) = self.token(Token::ELLIPSIS)? {
                Some(parser_go_ast::Expr::Ellipsis(parser_go_ast::Ellipsis {
                    ellipsis: ellipsis.0,
                    elt: None,
                }))
            } else {
                self.Expression()?
            }
        } else {
            self.Expression()?
        };

        self.token(Token::RBRACK).required()?;

        let element_type = self.ElementType().required()?;

        Ok(Some(parser_go_ast::ArrayType {
            lbrack: lbrack.0,
            len: len.map(Box::new),
            elt: Box::new(element_type),
        }))
    }

    // MapType = "map" "[" KeyType "]" ElementType .
    fn MapType(&mut self) -> Result<Option<parser_go_ast::MapType<'parser_go_scanner>>> {
        let map = match self.token(Token::MAP)? {
            Some(v) => v,
            None => return Ok(None),
        };
        self.token(Token::LBRACK).required()?;
        let key_type = self.KeyType().required()?;
        self.token(Token::RBRACK).required()?;
        let element_type = self.ElementType().required()?;

        Ok(Some(parser_go_ast::MapType {
            map: map.0,
            key: Box::new(key_type),
            value: Box::new(element_type),
        }))
    }

    // KeyType = Type .
    fn KeyType(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        self.Type()
    }

    // ChannelType = ( "chan" | "chan" "<-" | "<-" "chan" ) ElementType .
    fn ChannelType(&mut self) -> Result<Option<parser_go_ast::ChanType<'parser_go_scanner>>> {
        if let Some(chan) = self.token(Token::CHAN)? {
            if let Some(arrow) = self.token(Token::ARROW)? {
                let value = Box::new(self.ElementType().required()?);
                return Ok(Some(parser_go_ast::ChanType {
                    begin: chan.0,
                    arrow: Some(arrow.0),
                    dir: parser_go_ast::ChanDir::SEND as u8,
                    value,
                }));
            }

            let value = Box::new(self.ElementType().required()?);
            return Ok(Some(parser_go_ast::ChanType {
                begin: chan.0,
                arrow: None,
                dir: parser_go_ast::ChanDir::SEND as u8 | parser_go_ast::ChanDir::RECV as u8,
                value,
            }));
        }

        if let Some(arrow) = self.token(Token::ARROW)? {
            self.token(Token::CHAN).required()?;
            let value = Box::new(self.ElementType().required()?);
            return Ok(Some(parser_go_ast::ChanType {
                begin: arrow.0,
                arrow: None,
                dir: parser_go_ast::ChanDir::RECV as u8,
                value,
            }));
        }

        Ok(None)
    }

    // ElementType = Type .
    fn ElementType(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        self.Type()
    }

    // PointerType = "*" BaseType .
    fn PointerType(&mut self) -> Result<Option<parser_go_ast::StarExpr<'parser_go_scanner>>> {
        let star = match self.token(Token::MUL)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let x = Box::new(self.BaseType().required()?);
        Ok(Some(parser_go_ast::StarExpr { star: star.0, x }))
    }

    // BaseType = Type .
    fn BaseType(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        self.Type()
    }

    // InterfaceType = "interface" "{" { ( MethodSpec | InterfaceTypeName ) ";" } "}" .
    // MethodSpec    = MethodName Signature .
    fn InterfaceType(
        &mut self,
    ) -> Result<Option<parser_go_ast::InterfaceType<'parser_go_scanner>>> {
        let interface = match self.token(Token::INTERFACE)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let lbrace = self.token(Token::LBRACE).required()?;

        let mut fields = vec![];
        loop {
            if let Some(method_spec) = self.MethodName()? {
                if let Some(signature) = self.Signature(None)? {
                    self.token(Token::SEMICOLON).required()?;
                    fields.push(parser_go_ast::Field {
                        doc: None,
                        names: Some(vec![method_spec]),
                        type_: Some(parser_go_ast::Expr::FuncType(signature)),
                        tag: None,
                        comment: None,
                    });
                    continue;
                }

                fields.push(parser_go_ast::Field {
                    doc: None,
                    names: None,
                    type_: Some(parser_go_ast::Expr::Ident(method_spec)),
                    tag: None,
                    comment: None,
                });
                if self.token(Token::SEMICOLON)?.is_none() {
                    break;
                }
                continue;
            };

            if let Some(interface_type_name) = self.InterfaceTypeName()? {
                fields.push(parser_go_ast::Field {
                    doc: None,
                    names: None,
                    type_: Some(interface_type_name),
                    tag: None,
                    comment: None,
                });
                if self.token(Token::SEMICOLON)?.is_none() {
                    break;
                }
                continue;
            }

            break;
        }

        let rbrace = self.token(Token::RBRACE).required()?;

        Ok(Some(parser_go_ast::InterfaceType {
            interface: interface.0,
            methods: Some(parser_go_ast::FieldList {
                opening: Some(lbrace.0),
                list: fields,
                closing: Some(rbrace.0),
            }),
            incomplete: false,
        }))
    }

    // MethodName = identifier .
    fn MethodName(&mut self) -> Result<Option<parser_go_ast::Ident<'parser_go_scanner>>> {
        self.identifier()
    }

    // InterfaceTypeName = TypeName .
    fn InterfaceTypeName(&mut self) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        self.TypeName()
    }

    // StructType = "struct" "{" { FieldDecl ";" } "}" .
    fn StructType(&mut self) -> Result<Option<parser_go_ast::StructType<'parser_go_scanner>>> {
        let struct_ = match self.token(Token::STRUCT)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let lbrace = self.token(Token::LBRACE).required()?;

        let mut fields = vec![];
        while let Some(field_decl) = self.FieldDecl()? {
            fields.push(field_decl);
            if self.token(Token::SEMICOLON)?.is_none() {
                break;
            }
        }

        let rbrace = self.token(Token::RBRACE).required()?;

        Ok(Some(parser_go_ast::StructType {
            struct_: struct_.0,
            fields: Some(parser_go_ast::FieldList {
                opening: Some(lbrace.0),
                list: fields,
                closing: Some(rbrace.0),
            }),
            incomplete: false,
        }))
    }

    // FieldDecl     = (IdentifierList Type | EmbeddedField) [ Tag ] .
    // EmbeddedField = [ "*" ] TypeName .
    fn FieldDecl(&mut self) -> Result<Option<parser_go_ast::Field<'parser_go_scanner>>> {
        if let Some(star) = self.token(Token::MUL)? {
            let type_name = Box::new(self.TypeName().required()?);
            let tag = self.Tag()?;
            return Ok(Some(parser_go_ast::Field {
                doc: None,
                type_: Some(parser_go_ast::Expr::StarExpr(parser_go_ast::StarExpr {
                    star: star.0,
                    x: type_name,
                })),
                names: None,
                tag,
                comment: None,
            }));
        };

        if let Some(names) = self.IdentifierList()? {
            if let Some(type_) = self.Type()? {
                let tag = self.Tag()?;
                return Ok(Some(parser_go_ast::Field {
                    doc: None,
                    names: Some(names),
                    type_: Some(type_),
                    tag,
                    comment: None,
                }));
            }

            if names.len() == 1 {
                let name = names.into_iter().next().unwrap();
                let tag = self.Tag()?;
                return Ok(Some(parser_go_ast::Field {
                    doc: None,
                    type_: Some(parser_go_ast::Expr::Ident(name)),
                    names: None,
                    tag,
                    comment: None,
                }));
            }

            return Err(ParserError::UnexpectedToken);
        }

        if let Some(type_) = self.TypeName()? {
            let tag = self.Tag()?;
            return Ok(Some(parser_go_ast::Field {
                doc: None,
                type_: Some(type_),
                names: None,
                tag,
                comment: None,
            }));
        }

        Ok(None)
    }

    // Tag = string_lit .
    fn Tag(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.string_lit()
    }

    // Signature = Parameters [ Result ] .
    fn Signature(
        &mut self,
        func: Option<Position<'parser_go_scanner>>,
    ) -> Result<Option<parser_go_ast::FuncType<'parser_go_scanner>>> {
        let params = match self.Parameters()? {
            Some(v) => v,
            None => return Ok(None),
        };
        let results = self.Result()?;

        Ok(Some(parser_go_ast::FuncType { func, params, results }))
    }

    // Result = Parameters | Type .
    fn Result(&mut self) -> Result<Option<parser_go_ast::FieldList<'parser_go_scanner>>> {
        if let Some(parameters) = self.Parameters()? {
            Ok(Some(parameters))
        } else if let Some(type_) = self.Type()? {
            Ok(Some(parser_go_ast::FieldList {
                opening: None,
                list: vec![parser_go_ast::Field {
                    doc: None,
                    names: None,
                    tag: None,
                    type_: Some(type_),
                    comment: None,
                }],
                closing: None,
            }))
        } else {
            Ok(None)
        }
    }

    // Parameters = "(" [ ParameterList [ "," ] ] ")" .
    fn Parameters(&mut self) -> Result<Option<parser_go_ast::FieldList<'parser_go_scanner>>> {
        let lparen = match self.token(Token::LPAREN)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let list = self
            .ParameterList()?
            .map(|list| {
                let _ = self.token(Token::COMMA);
                list
            })
            .unwrap_or_default();
        let rparen = self.token(Token::RPAREN).required()?;

        Ok(Some(parser_go_ast::FieldList {
            opening: Some(lparen.0),
            list,
            closing: Some(rparen.0),
        }))
    }

    // ParameterList = ParameterDecl { "," ParameterDecl } .
    // ParameterDecl = [ IdentifierList ] [ "..." ] Type .
    fn ParameterList(&mut self) -> Result<Option<Vec<parser_go_ast::Field<'parser_go_scanner>>>> {
        let idents = match self.IdentifierList()? {
            Some(v) => v,
            None => return Ok(None),
        };
        let type_ = self.Type()?;

        // If no type can be found, then the idents are types, e.g.: (bool, bool)
        if type_.is_none() {
            return Ok(Some(
                idents
                    .into_iter()
                    .map(|ident| parser_go_ast::Field {
                        doc: None,
                        names: None,
                        type_: Some(parser_go_ast::Expr::Ident(ident)),
                        tag: None,
                        comment: None,
                    })
                    .collect(),
            ));
        }

        // If a type can be found, then we expect idents + types: (a, b bool, c bool, d bool)

        let mut fields = vec![parser_go_ast::Field {
            comment: None,
            type_,
            tag: None,
            names: Some(idents),
            doc: None,
        }];

        while self.token(Token::COMMA)?.is_some() {
            let idents = self.IdentifierList().required()?;
            let ellipsis = self.token(Token::ELLIPSIS)?;
            let type_ = self.Type().required()?;

            if let Some(ellipsis) = ellipsis {
                fields.push(parser_go_ast::Field {
                    comment: None,
                    type_: Some(parser_go_ast::Expr::Ellipsis(parser_go_ast::Ellipsis {
                        ellipsis: ellipsis.0,
                        elt: Some(Box::new(type_)),
                    })),
                    tag: None,
                    names: Some(idents),
                    doc: None,
                });
                return Ok(Some(fields));
            }

            fields.push(parser_go_ast::Field {
                comment: None,
                type_: Some(type_),
                tag: None,
                names: Some(idents),
                doc: None,
            });
        }

        Ok(Some(fields))
    }

    // FunctionBody = Block .
    fn FunctionBody(&mut self) -> Result<Option<parser_go_ast::BlockStmt<'parser_go_scanner>>> {
        self.Block()
    }

    // Block         = "{" StatementList "}" .
    // StatementList = { Statement ";" } .
    fn Block(&mut self) -> Result<Option<parser_go_ast::BlockStmt<'parser_go_scanner>>> {
        let lbrace = match self.token(Token::LBRACE)? {
            Some(v) => v,
            None => return Ok(None),
        };

        let list = vec![];

        let rbrace = self.token(Token::RBRACE).required()?;

        Ok(Some(parser_go_ast::BlockStmt {
            lbrace: lbrace.0,
            list,
            rbrace: rbrace.0,
        }))
    }

    // Receiver = Parameters .
    fn Receiver(&mut self) -> Result<Option<parser_go_ast::FieldList<'parser_go_scanner>>> {
        self.Parameters()
    }

    // identifier | QualifiedIdent
    // QualifiedIdent = PackageName "." identifier .
    // PackageName    = identifier .
    fn identifier_or_QualifiedIdent(
        &mut self,
    ) -> Result<Option<parser_go_ast::Expr<'parser_go_scanner>>> {
        let ident = match self.identifier()? {
            Some(v) => v,
            None => return Ok(None),
        };

        if self.token(Token::PERIOD)?.is_some() {
            let sel = self.identifier().required()?;
            return Ok(Some(parser_go_ast::Expr::SelectorExpr(
                parser_go_ast::SelectorExpr { x: Box::new(parser_go_ast::Expr::Ident(ident)), sel },
            )));
        }

        Ok(Some(parser_go_ast::Expr::Ident(ident)))
    }

    // FunctionDecl | MethodDecl
    // FunctionDecl = "func" FunctionName Signature [ FunctionBody ] .
    // MethodDecl   = "func" Receiver MethodName Signature [ FunctionBody ] .
    // FunctionName = identifier .
    // MethodName   = identifier .
    fn FunctionDecl_or_MethodDecl(
        &mut self,
    ) -> Result<Option<parser_go_ast::FuncDecl<'parser_go_scanner>>> {
        let func = match self.token(Token::FUNC)? {
            Some(v) => v,
            None => return Ok(None),
        };
        let recv = self.Receiver()?;
        let name = self.identifier().required()?;
        let type_ = self.Signature(Some(func.0)).required()?;
        let body = self.FunctionBody()?;

        Ok(Some(parser_go_ast::FuncDecl {
            doc: None,
            recv,
            name,
            type_,
            body,
        }))
    }

    // unary_op = "+" | "-" | "!" | "^" | "*" | "&" | "<-" .
    fn unary_op(&mut self) -> Result<Option<parser_go_scanner::Step<'parser_go_scanner>>> {
        use Token::*;
        Ok(match self.current_step {
            step @ (_, ADD | SUB | NOT | MUL | XOR | AND | ARROW, _) => {
                self.next()?;
                Some(step)
            }
            _ => None,
        })
    }

    // binary_op = "||" | "&&" | rel_op | add_op | mul_op .
    // rel_op    = "==" | "!=" | "<" | "<=" | ">" | ">=" .
    // add_op    = "+" | "-" | "|" | "^" .
    // mul_op    = "*" | "/" | "%" | "<<" | ">>" | "&" | "&^" .
    fn get_binary_op(
        &mut self,
        min_precedence: u8,
    ) -> Result<Option<parser_go_scanner::Step<'parser_go_scanner>>> {
        use Token::*;
        Ok(match self.current_step {
            step @ (_,
                /* binary_op */
                LOR | LAND |
                /* rel_op */
                EQL | NEQ | LSS | LEQ | GTR | GEQ |
                /* add_op */
                ADD | SUB | OR | XOR |
                /* mul_op */
                MUL | QUO | REM | SHL | SHR | AND | AND_NOT
            , _) if step.1.precedence() >= min_precedence => {
                Some(step)
            }
            _ => None,
        })
    }

    fn identifier(&mut self) -> Result<Option<parser_go_ast::Ident<'parser_go_scanner>>> {
        self.token(Token::IDENT)?
            .map_or(Ok(None), |(name_pos, _, name)| {
                Ok(Some(parser_go_ast::Ident { name_pos, name, obj: None }))
            })
    }

    fn int_lit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.token(Token::INT)?
            .map_or(Ok(None), |(value_pos, kind, value)| {
                Ok(Some(parser_go_ast::BasicLit { value_pos, kind, value }))
            })
    }

    fn float_lit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.token(Token::FLOAT)?
            .map_or(Ok(None), |(value_pos, kind, value)| {
                Ok(Some(parser_go_ast::BasicLit { value_pos, kind, value }))
            })
    }

    fn imaginary_lit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.token(Token::IMAG)?
            .map_or(Ok(None), |(value_pos, kind, value)| {
                Ok(Some(parser_go_ast::BasicLit { value_pos, kind, value }))
            })
    }

    fn rune_lit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.token(Token::CHAR)?
            .map_or(Ok(None), |(value_pos, kind, value)| {
                Ok(Some(parser_go_ast::BasicLit { value_pos, kind, value }))
            })
    }

    fn string_lit(&mut self) -> Result<Option<parser_go_ast::BasicLit<'parser_go_scanner>>> {
        self.token(Token::STRING)?
            .map_or(Ok(None), |(value_pos, kind, value)| {
                Ok(Some(parser_go_ast::BasicLit { value_pos, kind, value }))
            })
    }

    /// Returns the current step and advances to the next one, but only if it matches the expected
    /// token. [`Parser::next`] is automatically called for you.
    fn token(
        &mut self,
        expected: Token,
    ) -> Result<Option<parser_go_scanner::Step<'parser_go_scanner>>> {
        Ok(match self.current_step {
            step @ (_, tok, _) if tok == expected => {
                if expected != Token::EOF {
                    self.next()?;
                }
                Some(step)
            }
            _ => None,
        })
    }

    /// Advances to the next token. Skips all the comment tokens.
    fn next(&mut self) -> Result<()> {
        if let Some(step) = self
            .steps
            .find(|step| !matches!(step, Ok((_, Token::COMMENT, _))))
        {
            self.current_step = step?;
            return Ok(());
        }
        Err(ParserError::UnexpectedEndOfFile)
    }
}
