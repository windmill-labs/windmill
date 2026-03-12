use std::collections::HashMap;

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Spanned};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};

use crate::dag::{DagEdge, DagNode, DagNodeType, Param, WorkflowDag};
use crate::validation::{self, CompileError};

/// Maps task function name → optional external path (from `task("f/path", ...)`)
type TaskFunctions = HashMap<String, Option<String>>;

/// First pass: scan top-level `const foo = task(async (...) => {})` or
/// `const foo = task("f/path", async (...) => {})` declarations.
fn collect_task_functions(module: &Module) -> TaskFunctions {
    let mut tasks = HashMap::new();
    for item in &module.body {
        // const foo = task(async (...) => { ... })
        // const foo = task("f/path", async (...) => { ... })
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item {
            for decl in &var_decl.decls {
                if let (Some(name), Some(init)) = (extract_var_name(&decl.name), &decl.init) {
                    if let Some(path) = extract_task_call_info(init) {
                        tasks.insert(name, path);
                    }
                }
            }
        }
        // export const foo = task(...)
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) = item {
            if let Decl::Var(var_decl) = &export.decl {
                for decl in &var_decl.decls {
                    if let (Some(name), Some(init)) = (extract_var_name(&decl.name), &decl.init) {
                        if let Some(path) = extract_task_call_info(init) {
                            tasks.insert(name, path);
                        }
                    }
                }
            }
        }
    }
    tasks
}

/// Extract variable name from a pattern (simple ident case)
fn extract_var_name(pat: &Pat) -> Option<String> {
    if let Pat::Ident(BindingIdent { id, .. }) = pat {
        Some(id.sym.to_string())
    } else {
        None
    }
}

/// Check if expr is `task(async fn)` or `task("path", async fn)`.
/// Returns Some(optional_path) if it is a task() call.
fn extract_task_call_info(expr: &Expr) -> Option<Option<String>> {
    if let Expr::Call(call) = expr {
        if let Callee::Expr(callee) = &call.callee {
            if let Expr::Ident(ident) = callee.as_ref() {
                if ident.sym.as_ref() == "task" {
                    // task("f/path", async fn) or task(async fn)
                    if call.args.len() == 2 {
                        // task("f/path", async fn)
                        let path = extract_string_lit(&call.args[0].expr);
                        return Some(path);
                    } else if call.args.len() == 1 {
                        // task(async fn)
                        return Some(None);
                    }
                }
            }
        }
    }
    None
}

struct TsWacWalker {
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
    errors: Vec<CompileError>,
    node_counter: usize,
    cm: Lrc<SourceMap>,
    task_functions: TaskFunctions,
    in_try: bool,
    in_while: bool,
    in_nested_func: bool,
}

impl TsWacWalker {
    fn new(cm: Lrc<SourceMap>, task_functions: TaskFunctions) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            errors: Vec::new(),
            node_counter: 0,
            cm,
            task_functions,
            in_try: false,
            in_while: false,
            in_nested_func: false,
        }
    }

    fn next_id(&mut self) -> String {
        let id = format!("step_{}", self.node_counter);
        self.node_counter += 1;
        id
    }

    fn add_node(&mut self, node: DagNode) -> String {
        let id = node.id.clone();
        self.nodes.push(node);
        id
    }

    fn add_edge(&mut self, from: &str, to: &str, label: Option<String>) {
        self.edges
            .push(DagEdge { from: from.to_string(), to: to.to_string(), label });
    }

    fn span_line(&self, span: swc_common::Span) -> usize {
        let loc = self.cm.lookup_char_pos(span.lo);
        loc.line
    }

    /// Check if expr is a call to a known task function
    fn is_task_call(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Ident(ident) = callee.as_ref() {
                    return self.task_functions.contains_key(ident.sym.as_ref());
                }
            }
        }
        false
    }

    /// Check if expr is `Promise.all([...])`
    fn is_promise_all(expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Callee::Expr(callee) = &call.callee {
                if let Expr::Member(MemberExpr { obj, prop: MemberProp::Ident(prop), .. }) =
                    callee.as_ref()
                {
                    if prop.sym.as_ref() == "all" {
                        if let Expr::Ident(ident) = obj.as_ref() {
                            return ident.sym.as_ref() == "Promise";
                        }
                    }
                }
            }
        }
        false
    }

    /// Extract step name and script from a task function call.
    /// Name = function name, script = task_path or function name.
    fn extract_step_info_from_task_call(&self, call: &CallExpr) -> Option<(String, String)> {
        if let Callee::Expr(callee) = &call.callee {
            if let Expr::Ident(ident) = callee.as_ref() {
                let name = ident.sym.to_string();
                let script = self
                    .task_functions
                    .get(ident.sym.as_ref())
                    .and_then(|p| p.clone())
                    .unwrap_or_else(|| name.clone());
                return Some((name, script));
            }
        }
        None
    }

    fn expr_to_source(&self, expr: &Expr) -> String {
        let span = expr.span();
        self.cm
            .span_to_snippet(span)
            .unwrap_or_else(|_| "...".to_string())
    }

    fn body_contains_step(&self, stmts: &[Stmt]) -> bool {
        stmts.iter().any(|s| self.stmt_contains_step(s))
    }

    fn stmt_contains_step(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => self.expr_contains_step(&expr_stmt.expr),
            Stmt::Decl(Decl::Var(var_decl)) => var_decl.decls.iter().any(|d| {
                d.init
                    .as_ref()
                    .map_or(false, |init| self.expr_contains_step(init))
            }),
            Stmt::If(if_stmt) => {
                self.stmt_contains_step(&if_stmt.cons)
                    || if_stmt
                        .alt
                        .as_ref()
                        .map_or(false, |alt| self.stmt_contains_step(alt))
            }
            Stmt::Block(block) => self.body_contains_step(&block.stmts),
            Stmt::For(for_stmt) => self.stmt_contains_step(&for_stmt.body),
            Stmt::ForIn(for_in) => self.stmt_contains_step(&for_in.body),
            Stmt::ForOf(for_of) => self.stmt_contains_step(&for_of.body),
            Stmt::While(while_stmt) => self.stmt_contains_step(&while_stmt.body),
            Stmt::Try(try_stmt) => {
                self.body_contains_step(&try_stmt.block.stmts)
                    || try_stmt
                        .handler
                        .as_ref()
                        .map_or(false, |h| self.body_contains_step(&h.body.stmts))
                    || try_stmt
                        .finalizer
                        .as_ref()
                        .map_or(false, |f| self.body_contains_step(&f.stmts))
            }
            Stmt::Return(ret) => ret
                .arg
                .as_ref()
                .map_or(false, |arg| self.expr_contains_step(arg)),
            _ => false,
        }
    }

    fn expr_contains_step(&self, expr: &Expr) -> bool {
        if self.is_task_call(expr) {
            return true;
        }
        match expr {
            Expr::Await(await_expr) => self.expr_contains_step(&await_expr.arg),
            Expr::Call(call) => {
                if Self::is_promise_all(&Expr::Call(call.clone())) {
                    return call.args.iter().any(|a| self.expr_contains_step(&a.expr));
                }
                false
            }
            Expr::Paren(p) => self.expr_contains_step(&p.expr),
            _ => false,
        }
    }

    fn walk_body(&mut self, stmts: &[Stmt]) -> Option<(String, String)> {
        let mut first_id: Option<String> = None;
        let mut prev_id: Option<String> = None;

        for stmt in stmts {
            if let Some((stmt_first, stmt_last)) = self.walk_stmt(stmt) {
                if let Some(ref prev) = prev_id {
                    self.add_edge(prev, &stmt_first, None);
                }
                if first_id.is_none() {
                    first_id = Some(stmt_first);
                }
                prev_id = Some(stmt_last);
            }
        }

        match (first_id, prev_id) {
            (Some(f), Some(l)) => Some((f, l)),
            _ => None,
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) -> Option<(String, String)> {
        match stmt {
            Stmt::Expr(expr_stmt) => self.walk_expr_stmt(&expr_stmt.expr),
            Stmt::Decl(Decl::Var(var_decl)) => {
                // const result = await task_fn(...)
                for decl in &var_decl.decls {
                    if let Some(init) = &decl.init {
                        if let Some(result) = self.walk_expr_stmt(init) {
                            return Some(result);
                        }
                    }
                }
                None
            }
            Stmt::If(if_stmt) => self.walk_if(if_stmt),
            Stmt::For(for_stmt) => self.walk_for_stmt(for_stmt),
            Stmt::ForIn(for_in) => self.walk_for_in(for_in),
            Stmt::ForOf(for_of) => self.walk_for_of(for_of),
            Stmt::While(while_stmt) => self.walk_while(while_stmt),
            Stmt::Try(try_stmt) => self.walk_try(try_stmt),
            Stmt::Block(block) => self.walk_body(&block.stmts),
            Stmt::Return(ret) => self.walk_return(ret),
            Stmt::Decl(Decl::Fn(_)) => {
                if self.stmt_contains_step(stmt) {
                    self.errors.push(validation::error_step_in_nested_function(
                        self.span_line(stmt.span()),
                    ));
                }
                None
            }
            _ => None,
        }
    }

    fn walk_expr_stmt(&mut self, expr: &Expr) -> Option<(String, String)> {
        // await task_fn(...)
        if let Expr::Await(await_expr) = expr {
            if let Expr::Call(call) = await_expr.arg.as_ref() {
                if self.is_task_call(&Expr::Call(call.clone())) {
                    return self.emit_step(call, expr);
                }
            }
            // await Promise.all([task_fn(...), ...])
            if Self::is_promise_all(&await_expr.arg) {
                if let Expr::Call(promise_call) = await_expr.arg.as_ref() {
                    return self.emit_parallel(promise_call, expr);
                }
            }
        }

        // Bare task_fn() without await
        if self.is_task_call(expr) {
            self.errors
                .push(validation::error_missing_await(self.span_line(expr.span())));
        }

        None
    }

    fn emit_step(&mut self, call: &CallExpr, expr: &Expr) -> Option<(String, String)> {
        if self.in_try {
            self.errors
                .push(validation::error_step_in_catch(self.span_line(expr.span())));
            return None;
        }
        if self.in_while {
            self.errors
                .push(validation::error_step_in_while(self.span_line(expr.span())));
            return None;
        }
        if self.in_nested_func {
            self.errors.push(validation::error_step_in_nested_function(
                self.span_line(expr.span()),
            ));
            return None;
        }

        let (name, script) = self
            .extract_step_info_from_task_call(call)
            .unwrap_or(("unknown".into(), "unknown".into()));
        let id = self.next_id();
        let node_id = self.add_node(DagNode {
            id: id.clone(),
            node_type: DagNodeType::Step { name: name.clone(), script },
            label: name,
            line: self.span_line(expr.span()),
        });
        Some((node_id.clone(), node_id))
    }

    fn emit_parallel(&mut self, promise_call: &CallExpr, expr: &Expr) -> Option<(String, String)> {
        if self.in_try {
            self.errors
                .push(validation::error_step_in_catch(self.span_line(expr.span())));
            return None;
        }
        if self.in_while {
            self.errors
                .push(validation::error_step_in_while(self.span_line(expr.span())));
            return None;
        }

        let line = self.span_line(expr.span());
        let start_id = self.next_id();
        let start_node_id = self.add_node(DagNode {
            id: start_id.clone(),
            node_type: DagNodeType::ParallelStart,
            label: "parallel".to_string(),
            line,
        });

        let mut step_ids = Vec::new();

        // Promise.all takes an array as first argument
        if let Some(first_arg) = promise_call.args.first() {
            if let Expr::Array(ArrayLit { elems, .. }) = first_arg.expr.as_ref() {
                for elem in elems.iter().flatten() {
                    if let Expr::Call(call) = elem.expr.as_ref() {
                        if self.is_task_call(&Expr::Call(call.clone())) {
                            let (name, script) = self
                                .extract_step_info_from_task_call(call)
                                .unwrap_or(("unknown".into(), "unknown".into()));
                            let step_id = self.next_id();
                            let node_id = self.add_node(DagNode {
                                id: step_id.clone(),
                                node_type: DagNodeType::Step { name: name.clone(), script },
                                label: name,
                                line: self.span_line(elem.expr.span()),
                            });
                            self.add_edge(&start_node_id, &node_id, None);
                            step_ids.push(node_id);
                        }
                    }
                }
            }
        }

        let end_id = self.next_id();
        let end_node_id = self.add_node(DagNode {
            id: end_id.clone(),
            node_type: DagNodeType::ParallelEnd,
            label: "join".to_string(),
            line,
        });

        for step_id in &step_ids {
            self.add_edge(step_id, &end_node_id, None);
        }

        Some((start_node_id, end_node_id))
    }

    fn walk_if(&mut self, if_stmt: &IfStmt) -> Option<(String, String)> {
        let has_steps_cons = self.stmt_contains_step(&if_stmt.cons);
        let has_steps_alt = if_stmt
            .alt
            .as_ref()
            .map_or(false, |a| self.stmt_contains_step(a));

        if !has_steps_cons && !has_steps_alt {
            return None;
        }

        let line = self.span_line(if_stmt.span);
        let condition_source = self.expr_to_source(&if_stmt.test);

        let branch_id = self.next_id();
        let branch_node_id = self.add_node(DagNode {
            id: branch_id.clone(),
            node_type: DagNodeType::Branch { condition_source },
            label: "if".to_string(),
            line,
        });

        let mut last_ids = Vec::new();

        // True branch
        if let Some((true_first, true_last)) = self.walk_stmt(&if_stmt.cons) {
            self.add_edge(&branch_node_id, &true_first, Some("true".to_string()));
            last_ids.push(true_last);
        } else {
            last_ids.push(branch_node_id.clone());
        }

        // False branch
        if let Some(alt) = &if_stmt.alt {
            if let Some((else_first, else_last)) = self.walk_stmt(alt) {
                self.add_edge(&branch_node_id, &else_first, Some("false".to_string()));
                last_ids.push(else_last);
            } else {
                last_ids.push(branch_node_id.clone());
            }
        }

        if last_ids.len() == 1 {
            Some((branch_node_id, last_ids.into_iter().next().unwrap()))
        } else {
            let merge_id = format!("{branch_id}_merge");
            Some((branch_node_id, merge_id))
        }
    }

    fn walk_for_stmt(&mut self, for_stmt: &ForStmt) -> Option<(String, String)> {
        if !self.stmt_contains_step(&for_stmt.body) {
            return None;
        }
        self.walk_loop_body(&for_stmt.body, for_stmt.span, "for")
    }

    fn walk_for_in(&mut self, for_in: &ForInStmt) -> Option<(String, String)> {
        if !self.stmt_contains_step(&for_in.body) {
            return None;
        }
        let iter_source = self.expr_to_source(&for_in.right);
        self.walk_loop_body_with_iter(&for_in.body, for_in.span, &iter_source)
    }

    fn walk_for_of(&mut self, for_of: &ForOfStmt) -> Option<(String, String)> {
        if !self.stmt_contains_step(&for_of.body) {
            return None;
        }
        let iter_source = self.expr_to_source(&for_of.right);
        self.walk_loop_body_with_iter(&for_of.body, for_of.span, &iter_source)
    }

    fn walk_loop_body(
        &mut self,
        body: &Stmt,
        span: swc_common::Span,
        _label: &str,
    ) -> Option<(String, String)> {
        self.walk_loop_body_with_iter(body, span, "...")
    }

    fn walk_loop_body_with_iter(
        &mut self,
        body: &Stmt,
        span: swc_common::Span,
        iter_source: &str,
    ) -> Option<(String, String)> {
        let line = self.span_line(span);
        let start_id = self.next_id();
        let start_node_id = self.add_node(DagNode {
            id: start_id.clone(),
            node_type: DagNodeType::LoopStart { iter_source: iter_source.to_string() },
            label: "for".to_string(),
            line,
        });

        if let Some((body_first, body_last)) = self.walk_stmt(body) {
            self.add_edge(&start_node_id, &body_first, None);
            self.add_edge(&body_last, &start_node_id, Some("next".to_string()));
        }

        let end_id = self.next_id();
        let end_node_id = self.add_node(DagNode {
            id: end_id.clone(),
            node_type: DagNodeType::LoopEnd,
            label: "end for".to_string(),
            line,
        });
        self.add_edge(&start_node_id, &end_node_id, Some("done".to_string()));

        Some((start_node_id, end_node_id))
    }

    fn walk_while(&mut self, while_stmt: &WhileStmt) -> Option<(String, String)> {
        if self.stmt_contains_step(&while_stmt.body) {
            self.errors.push(validation::error_step_in_while(
                self.span_line(while_stmt.span),
            ));
        }
        None
    }

    fn walk_try(&mut self, try_stmt: &TryStmt) -> Option<(String, String)> {
        let has_steps = self.body_contains_step(&try_stmt.block.stmts)
            || try_stmt
                .handler
                .as_ref()
                .map_or(false, |h| self.body_contains_step(&h.body.stmts))
            || try_stmt
                .finalizer
                .as_ref()
                .map_or(false, |f| self.body_contains_step(&f.stmts));

        if has_steps {
            self.errors.push(validation::error_step_in_catch(
                self.span_line(try_stmt.span),
            ));
        }
        None
    }

    fn walk_return(&mut self, ret: &ReturnStmt) -> Option<(String, String)> {
        let line = self.span_line(ret.span);
        let id = self.next_id();
        let node_id = self.add_node(DagNode {
            id: id.clone(),
            node_type: DagNodeType::Return,
            label: "return".to_string(),
            line,
        });
        Some((node_id.clone(), node_id))
    }
}

/// Extract workflow function params (no longer skips ctx)
fn extract_ts_params(params: &[swc_ecma_ast::Param], cm: &Lrc<SourceMap>) -> Vec<Param> {
    let mut result = Vec::new();
    for param in params {
        let (name, typ) = match &param.pat {
            Pat::Ident(BindingIdent { id, type_ann, .. }) => {
                let name = id.sym.to_string();
                let typ = type_ann.as_ref().map(|ann| {
                    cm.span_to_snippet(ann.type_ann.span())
                        .unwrap_or_else(|_| "unknown".to_string())
                });
                (name, typ)
            }
            _ => continue,
        };
        result.push(Param { name, typ });
    }
    result
}

pub fn parse_ts_workflow(code: &str) -> Result<WorkflowDag, Vec<CompileError>> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("workflow.ts".into()).into(), code.into());
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| vec![CompileError { message: format!("Parse error: {e:?}"), line: 0 }])?;

    // First pass: collect task functions
    let task_functions = collect_task_functions(&module);

    // Find: export default workflow(async (...) => { ... })
    // or: export default workflow(async function(...) { ... })
    let mut workflow_body: Option<(&[Stmt], Vec<Param>)> = None;

    for item in &module.body {
        // export default workflow(async (...) => { ... })
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export)) = item {
            if let Some(result) = find_workflow_call(&export.expr, &cm) {
                workflow_body = Some(result);
                break;
            }
        }
        // const wf = workflow(async (...) => { ... }); export default wf;
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) = item {
            if let DefaultDecl::Fn(_) = &export.decl {
                // `export default async function(...) { ... }` — not wrapped in workflow(), skip
            }
        }
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = item {
            for decl in &var_decl.decls {
                if let Some(init) = &decl.init {
                    if let Some(result) = find_workflow_call(init, &cm) {
                        workflow_body = Some(result);
                        break;
                    }
                }
            }
        }
    }

    let (stmts, params) = workflow_body.ok_or_else(|| {
        vec![CompileError {
            message: "No workflow() wrapped async function found.".to_string(),
            line: 0,
        }]
    })?;

    let source_hash = compute_source_hash(code);

    let mut walker = TsWacWalker::new(cm, task_functions);
    walker.walk_body(stmts);

    if !walker.errors.is_empty() {
        return Err(walker.errors);
    }

    Ok(WorkflowDag { nodes: walker.nodes, edges: walker.edges, params, source_hash })
}

/// Find workflow(async (...) => { ... }) or workflow(async function(...) { ... })
fn find_workflow_call<'a>(expr: &'a Expr, cm: &Lrc<SourceMap>) -> Option<(&'a [Stmt], Vec<Param>)> {
    if let Expr::Call(call) = expr {
        // Check if callee is `workflow`
        let is_workflow = match &call.callee {
            Callee::Expr(callee_expr) => {
                if let Expr::Ident(ident) = callee_expr.as_ref() {
                    ident.sym.as_ref() == "workflow"
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_workflow {
            if let Some(first_arg) = call.args.first() {
                return extract_async_fn_body(&first_arg.expr, cm);
            }
        }
    }
    None
}

fn extract_async_fn_body<'a>(
    expr: &'a Expr,
    cm: &Lrc<SourceMap>,
) -> Option<(&'a [Stmt], Vec<Param>)> {
    match expr {
        Expr::Arrow(arrow) if arrow.is_async => {
            let params = extract_arrow_params(&arrow.params, cm);
            match &*arrow.body {
                BlockStmtOrExpr::BlockStmt(block) => Some((&block.stmts, params)),
                _ => None,
            }
        }
        Expr::Fn(fn_expr) if fn_expr.function.is_async => {
            let params = extract_ts_params(&fn_expr.function.params, cm);
            fn_expr
                .function
                .body
                .as_ref()
                .map(|body| (body.stmts.as_slice(), params))
        }
        Expr::Paren(p) => extract_async_fn_body(&p.expr, cm),
        _ => None,
    }
}

/// Extract arrow function params (no longer skips ctx)
fn extract_arrow_params(pats: &[Pat], cm: &Lrc<SourceMap>) -> Vec<Param> {
    let mut result = Vec::new();
    for pat in pats {
        match pat {
            Pat::Ident(BindingIdent { id, type_ann, .. }) => {
                let name = id.sym.to_string();
                let typ = type_ann.as_ref().map(|ann| {
                    cm.span_to_snippet(ann.type_ann.span())
                        .unwrap_or_else(|_| "unknown".to_string())
                });
                result.push(Param { name, typ });
            }
            _ => {}
        }
    }
    result
}

fn extract_string_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
        Expr::Tpl(tpl) if tpl.exprs.is_empty() && tpl.quasis.len() == 1 => {
            tpl.quasis.first().map(|q| q.raw.to_string())
        }
        _ => None,
    }
}

fn compute_source_hash(code: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
}
