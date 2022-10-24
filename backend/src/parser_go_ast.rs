#![allow(clippy::large_enum_variant)] // TODO: we allow large enum variant for now, let's profile properly to see if we want to box.

use crate::parser_go_token::{Position, Token};
use std::collections::BTreeMap;

// https://pkg.go.dev/go/ast#CommentGroup
#[derive(Debug)]
pub struct CommentGroup {
    // List []*Comment // len(List) > 0
}

// https://pkg.go.dev/go/ast#FieldList
#[derive(Debug)]
pub struct FieldList<'a> {
    pub opening: Option<Position<'a>>, // position of opening parenthesis/brace, if any
    pub list: Vec<Field<'a>>,          // field list; or nil
    pub closing: Option<Position<'a>>, // position of closing parenthesis/brace, if any
}

// https://pkg.go.dev/go/ast#Field
#[derive(Debug)]
pub struct Field<'a> {
    pub doc: Option<CommentGroup>,     // associated documentation; or nil
    pub names: Option<Vec<Ident<'a>>>, // field/method/(type) parameter names, or type "type"; or nil
    pub type_: Option<Expr<'a>>,       // field/method/parameter type, type list type; or nil
    pub tag: Option<BasicLit<'a>>,     // field tag; or nil
    pub comment: Option<CommentGroup>, // line comments; or nil
}

// https://pkg.go.dev/go/ast#File
#[derive(Debug)]
pub struct File<'a> {
    // package name
    pub decls: Vec<Decl<'a>>, // top-level declarations; or nil // list of all comments in the source file
}

// https://pkg.go.dev/go/ast#FuncDecl
#[derive(Debug)]
pub struct FuncDecl<'a> {
    pub doc: Option<CommentGroup>,   // associated documentation; or nil
    pub recv: Option<FieldList<'a>>, // receiver (methods); or nil (functions)
    pub name: Ident<'a>,             // function/method name
    pub type_: FuncType<'a>, // function signature: type and value parameters, results, and position of "func" keyword
    pub body: Option<BlockStmt<'a>>, // function body; or nil for external (non-Go) function
}

// https://pkg.go.dev/go/ast#BlockStmt
#[derive(Debug)]
pub struct BlockStmt<'a> {
    pub lbrace: Position<'a>, // position of "{"
    pub list: Vec<Stmt>,
    pub rbrace: Position<'a>, // position of "}", if any (may be absent due to syntax error)
}

// https://pkg.go.dev/go/ast#FuncType
#[derive(Debug)]
pub struct FuncType<'a> {
    pub func: Option<Position<'a>>, // position of "func" keyword (token.NoPos if there is no "func")
    pub params: FieldList<'a>,      // (incoming) parameters; non-nil
    pub results: Option<FieldList<'a>>, // (outgoing) results; or nil
}

// https://pkg.go.dev/go/ast#Ident
#[derive(Debug)]
pub struct Ident<'a> {
    pub name_pos: Position<'a>,       // identifier position
    pub name: &'a str,                // identifier name
    pub obj: Option<Box<Object<'a>>>, // denoted object; or nil
}

// https://pkg.go.dev/go/ast#ValueSpec
#[derive(Debug)]
pub struct ValueSpec<'a> {
    pub doc: Option<CommentGroup>,     // associated documentation; or nil
    pub names: Vec<Ident<'a>>,         // value names (len(Names) > 0)
    pub type_: Option<Expr<'a>>,       // value type; or nil
    pub values: Option<Vec<Expr<'a>>>, // initial values; or nil
    pub comment: Option<CommentGroup>, // line comments; or nil
}

// https://pkg.go.dev/go/ast#BasicLit
#[derive(Debug)]
pub struct BasicLit<'a> {
    pub value_pos: Position<'a>, // literal position
    pub kind: Token,             // token.INT, token.FLOAT, token.IMAG, token.CHAR, or token.STRING
    pub value: &'a str, // literal string; e.g. 42, 0x7f, 3.14, 1e-9, 2.4i, 'a', '\x7f', "foo" or `\m\n\o`
}

// https://pkg.go.dev/go/ast#Object
#[derive(Debug)]
pub struct Object<'a> {
    pub kind: ObjKind,
    pub name: &'a str,         // declared name
    pub decl: Option<ObjDecl>, // corresponding Field, XxxSpec, FuncDecl, LabeledStmt, AssignStmt, Scope; or nil
    pub data: Option<usize>,   // object-specific data; or nil
    pub type_: Option<()>,     // placeholder for type information; may be nil
}

// https://pkg.go.dev/go/ast#Ellipsis
#[derive(Debug)]
pub struct Ellipsis<'a> {
    pub ellipsis: Position<'a>,     // position of "..."
    pub elt: Option<Box<Expr<'a>>>, // ellipsis element type (parameter lists only); or nil
}

// https://pkg.go.dev/go/ast#Ellipsis
#[derive(Debug)]
pub struct TypeAssertExpr<'a> {
    pub x: Box<Expr<'a>>,     // expression
    pub lparen: Position<'a>, // position of "("
    pub type_: Box<Expr<'a>>, // asserted type; nil means type switch X.(type)
    pub rparen: Position<'a>, // position of ")"
}

// https://pkg.go.dev/go/ast#SliceExpr
#[derive(Debug)]
pub struct SliceExpr<'a> {
    pub x: Box<Expr<'a>>,            // expression
    pub lbrack: Position<'a>,        // position of "["
    pub low: Option<Box<Expr<'a>>>,  // begin of slice range; or nil
    pub high: Option<Box<Expr<'a>>>, // end of slice range; or nil
    pub max: Option<Box<Expr<'a>>>,  // maximum capacity of slice; or nil
    pub slice3: bool,                // true if 3-index slice (2 colons present)
    pub rbrack: Position<'a>,        // position of "]"
}

// https://pkg.go.dev/go/ast#ObjKind
#[derive(Debug)]
pub enum ObjKind {}

#[derive(Debug)]
pub enum ObjDecl {}

// https://pkg.go.dev/go/ast#Decl
#[derive(Debug)]
pub enum Decl<'a> {
    FuncDecl(FuncDecl<'a>),
}

// https://pkg.go.dev/go/ast#Scope
#[derive(Debug)]
pub struct Scope<'a> {
    pub outer: Option<Box<Scope<'a>>>,
    pub objects: BTreeMap<&'a str, Object<'a>>,
}

// https://pkg.go.dev/go/ast#GenDecl
#[derive(Debug)]
pub struct GenDecl<'a> {
    pub doc: Option<CommentGroup>,    // associated documentation; or nil
    pub tok_pos: Position<'a>,        // position of Tok
    pub tok: Token,                   // IMPORT, CONST, TYPE, or VAR
    pub lparen: Option<Position<'a>>, // position of '(', if any
    pub specs: Vec<Spec>,
    pub rparen: Option<Position<'a>>, // position of ')', if any
}

// https://pkg.go.dev/go/ast#AssignStmt
#[derive(Debug)]
pub struct AssignStmt<'a> {
    pub lhs: Vec<Expr<'a>>,
    pub tok_pos: Position<'a>, // position of Tok
    pub tok: Token,            // assignment token, DEFINE
    pub rhs: Vec<Expr<'a>>,
}

// https://pkg.go.dev/go/ast#BinaryExpr
#[derive(Debug)]
pub struct BinaryExpr<'a> {
    pub x: Box<Expr<'a>>,     // left operand
    pub op_pos: Position<'a>, // position of Op
    pub op: Token,            // operator
    pub y: Box<Expr<'a>>,     // right operand
}

// https://pkg.go.dev/go/ast#ReturnStmt
#[derive(Debug)]
pub struct ReturnStmt<'a> {
    pub return_: Position<'a>,  // position of "return" keyword
    pub results: Vec<Expr<'a>>, // result expressions; or nil
}

// https://pkg.go.dev/go/ast#TypeSpec
#[derive(Debug)]
pub struct TypeSpec<'a> {
    pub doc: Option<CommentGroup>,     // associated documentation; or nil
    pub name: Option<Ident<'a>>,       // type name
    pub assign: Option<Position<'a>>,  // position of '=', if any
    pub type_: Expr<'a>, // *Ident, *ParenExpr, *SelectorExpr, *StarExpr, or any of the *XxxTypes
    pub comment: Option<CommentGroup>, // line comments; or nil
}

// https://pkg.go.dev/go/ast#StructType
#[derive(Debug)]
pub struct StructType<'a> {
    pub struct_: Position<'a>,         // position of "struct" keyword
    pub fields: Option<FieldList<'a>>, // list of field declarations
    pub incomplete: bool,              // true if (source) fields are missing in the Fields list
}

// https://pkg.go.dev/go/ast#StarExpr
#[derive(Debug)]
pub struct StarExpr<'a> {
    pub star: Position<'a>, // position of "*"
    pub x: Box<Expr<'a>>,   // operand
}

// https://pkg.go.dev/go/ast#InterfaceType
#[derive(Debug)]
pub struct InterfaceType<'a> {
    pub interface: Position<'a>,        // position of "interface" keyword
    pub methods: Option<FieldList<'a>>, // list of embedded interfaces, methods, or types
    pub incomplete: bool, // true if (source) methods or types are missing in the Methods list
}

// https://pkg.go.dev/go/ast#UnaryExpr
#[derive(Debug)]
pub struct UnaryExpr<'a> {
    pub op_pos: Position<'a>, // position of Op
    pub op: Token,            // operator
    pub x: Box<Expr<'a>>,     // operand
}

// https://pkg.go.dev/go/ast#CallExpr
#[derive(Debug)]
pub struct CallExpr<'a> {
    pub fun: Box<Expr<'a>>,             // function expression
    pub lparen: Position<'a>,           // position of "("
    pub args: Option<Vec<Expr<'a>>>,    // function arguments; or nil
    pub ellipsis: Option<Position<'a>>, // position of "..." (token.NoPos if there is no "...")
    pub rparen: Position<'a>,           // position of ")"
}

// https://pkg.go.dev/go/ast#SelectorExpr
#[derive(Debug)]
pub struct SelectorExpr<'a> {
    pub x: Box<Expr<'a>>, // expression
    pub sel: Ident<'a>,   // field selector
}

// https://pkg.go.dev/go/ast#ParenExpr
#[derive(Debug)]
pub struct ParenExpr<'a> {
    pub lparen: Position<'a>, // position of "("
    pub x: Box<Expr<'a>>,     // parenthesized expression
    pub rparen: Position<'a>, // position of ")"
}

// https://pkg.go.dev/go/ast#FuncLit
#[derive(Debug)]
pub struct FuncLit<'a> {
    pub type_: FuncType<'a>, // function type
    pub body: BlockStmt<'a>, // function body
}

// https://pkg.go.dev/go/ast#ChanType
#[derive(Debug)]
pub struct ChanType<'a> {
    pub begin: Position<'a>, // position of "chan" keyword or "<-" (whichever comes first)
    pub arrow: Option<Position<'a>>, // position of "<-" (token.NoPos if there is no "<-")
    pub dir: u8,             // channel direction
    pub value: Box<Expr<'a>>, // value type
}

// htt/opt/visual-studio-code/resources/app/out/vs/code/electron-sandbox/workbench/workbench.htmlps://pkg.go.dev/go/ast#IndexExpr
#[derive(Debug)]
pub struct IndexExpr<'a> {
    pub x: Box<Expr<'a>>,     // expression
    pub lbrack: Position<'a>, // position of "["
    pub index: Box<Expr<'a>>, // index expression
    pub rbrack: Position<'a>, // position of "]"
}

// https://pkg.go.dev/go/ast#MapType
#[derive(Debug)]
pub struct MapType<'a> {
    pub map: Position<'a>,
    pub key: Box<Expr<'a>>,
    pub value: Box<Expr<'a>>,
}

// https://pkg.go.dev/go/ast#CompositeLit
#[derive(Debug)]
pub struct CompositeLit<'a> {
    pub type_: Box<Expr<'a>>,        // literal type; or nil
    pub lbrace: Position<'a>,        // position of "{"
    pub elts: Option<Vec<Expr<'a>>>, // list of composite elements; or nil
    pub rbrace: Position<'a>,        // position of "}"
    pub incomplete: bool,            // true if (source) expressions are missing in the Elts list
}

// https://pkg.go.dev/go/ast#KeyValueExpr
#[derive(Debug)]
pub struct KeyValueExpr<'a> {
    pub key: Box<Expr<'a>>,
    pub colon: Position<'a>, // position of ":"
    pub value: Box<Expr<'a>>,
}

// https://pkg.go.dev/go/ast#ArrayType
#[derive(Debug)]
pub struct ArrayType<'a> {
    pub lbrack: Position<'a>,       // position of "["
    pub len: Option<Box<Expr<'a>>>, // Ellipsis node for [...]T array types, nil for slice types
    pub elt: Box<Expr<'a>>,         // element type
}

// https://pkg.go.dev/go/ast#ChanDir
#[derive(Debug)]
pub enum ChanDir {
    SEND = 1 << 0,
    RECV = 1 << 1,
}

// https://pkg.go.dev/go/ast#Spec
#[derive(Debug)]
pub enum Spec {}

// https://pkg.go.dev/go/ast#Expr
#[derive(Debug)]
pub enum Expr<'a> {
    ArrayType(ArrayType<'a>),
    BasicLit(BasicLit<'a>),
    BinaryExpr(BinaryExpr<'a>),
    CallExpr(CallExpr<'a>),
    ChanType(ChanType<'a>),
    CompositeLit(CompositeLit<'a>),
    Ellipsis(Ellipsis<'a>),
    FuncLit(FuncLit<'a>),
    FuncType(FuncType<'a>),
    Ident(Ident<'a>),
    IndexExpr(IndexExpr<'a>),
    InterfaceType(InterfaceType<'a>),
    KeyValueExpr(KeyValueExpr<'a>),
    MapType(MapType<'a>),
    ParenExpr(ParenExpr<'a>),
    SelectorExpr(SelectorExpr<'a>),
    SliceExpr(SliceExpr<'a>),
    StarExpr(StarExpr<'a>),
    StructType(StructType<'a>),
    TypeAssertExpr(TypeAssertExpr<'a>),
    UnaryExpr(UnaryExpr<'a>),
}

// https://pkg.go.dev/go/ast#Stmt
#[derive(Debug)]
pub enum Stmt {}
