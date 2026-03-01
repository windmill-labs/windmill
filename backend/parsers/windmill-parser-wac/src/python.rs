use std::collections::HashMap;

use rustpython_parser::{
    ast::{
        Expr, ExprAwait, ExprCall, ExprName, Stmt, StmtExpr, StmtFor, StmtIf, StmtReturn, StmtTry,
        StmtTryStar, StmtWhile,
    },
    Parse,
};

use crate::dag::{DagEdge, DagNode, DagNodeType, Param, WorkflowDag};
use crate::validation::{self, CompileError};

struct LineIndex {
    newline_offsets: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut offsets = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                offsets.push(i + 1);
            }
        }
        Self { newline_offsets: offsets }
    }

    fn line_of(&self, byte_offset: usize) -> usize {
        match self.newline_offsets.binary_search(&byte_offset) {
            Ok(line) => line + 1,
            Err(line) => line,
        }
    }
}

/// Maps task function name → optional external path (from `@task(path="...")`)
type TaskFunctions = HashMap<String, Option<String>>;

/// First pass: scan top-level `@task async def foo(...)` declarations.
fn collect_task_functions(stmts: &[Stmt]) -> TaskFunctions {
    let mut tasks = HashMap::new();
    for stmt in stmts {
        if let Stmt::AsyncFunctionDef(func) = stmt {
            for dec in &func.decorator_list {
                match dec {
                    // @task  (bare decorator)
                    Expr::Name(ExprName { id, .. }) if id.as_str() == "task" => {
                        tasks.insert(func.name.to_string(), None);
                    }
                    // @task(path="...")
                    Expr::Call(call) => {
                        if let Expr::Name(ExprName { id, .. }) = call.func.as_ref() {
                            if id.as_str() == "task" {
                                let path = extract_task_path_kwarg(call);
                                tasks.insert(func.name.to_string(), path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    tasks
}

/// Extract the `path=` keyword argument from a `@task(path="...")` call.
fn extract_task_path_kwarg(call: &ExprCall) -> Option<String> {
    for kw in &call.keywords {
        if let Some(ref arg) = kw.arg {
            if arg.as_str() == "path" {
                if let Expr::Constant(c) = &kw.value {
                    if let rustpython_parser::ast::Constant::Str(s) = &c.value {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

struct WacWalker {
    nodes: Vec<DagNode>,
    edges: Vec<DagEdge>,
    errors: Vec<CompileError>,
    node_counter: usize,
    line_index: LineIndex,
    task_functions: TaskFunctions,
    in_try: bool,
    in_while: bool,
    in_nested_func: bool,
    in_comprehension: bool,
}

impl WacWalker {
    fn new(source: &str, task_functions: TaskFunctions) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            errors: Vec::new(),
            node_counter: 0,
            line_index: LineIndex::new(source),
            task_functions,
            in_try: false,
            in_while: false,
            in_nested_func: false,
            in_comprehension: false,
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

    fn line_of_expr(&self, expr: &Expr) -> usize {
        let offset = match expr {
            Expr::Call(c) => c.range.start().to_usize(),
            Expr::Await(a) => a.range.start().to_usize(),
            Expr::Attribute(a) => a.range.start().to_usize(),
            Expr::Name(n) => n.range.start().to_usize(),
            _ => 0,
        };
        self.line_index.line_of(offset)
    }

    fn line_of_stmt(&self, stmt: &Stmt) -> usize {
        let offset = match stmt {
            Stmt::If(s) => s.range.start().to_usize(),
            Stmt::For(s) => s.range.start().to_usize(),
            Stmt::While(s) => s.range.start().to_usize(),
            Stmt::Return(s) => s.range.start().to_usize(),
            Stmt::Expr(s) => s.range.start().to_usize(),
            Stmt::Try(s) => s.range.start().to_usize(),
            Stmt::TryStar(s) => s.range.start().to_usize(),
            Stmt::Assign(s) => s.range.start().to_usize(),
            Stmt::AnnAssign(s) => s.range.start().to_usize(),
            Stmt::FunctionDef(s) => s.range.start().to_usize(),
            Stmt::AsyncFunctionDef(s) => s.range.start().to_usize(),
            _ => 0,
        };
        self.line_index.line_of(offset)
    }

    /// Check if an expression is a call to a known @task function
    fn is_task_fn_call(&self, expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Expr::Name(ExprName { id, .. }) = call.func.as_ref() {
                return self.task_functions.contains_key(id.as_str());
            }
        }
        false
    }

    /// Check if an expression is `asyncio.gather(...)` call
    fn is_asyncio_gather_call(expr: &Expr) -> bool {
        if let Expr::Call(call) = expr {
            if let Expr::Attribute(rustpython_parser::ast::ExprAttribute { value, attr, .. }) =
                call.func.as_ref()
            {
                if attr.as_str() == "gather" {
                    if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                        return id.as_str() == "asyncio";
                    }
                }
            }
        }
        false
    }

    /// Extract step name and script from a task function call.
    /// Name = function name, script = task_path or function name.
    fn extract_step_info_from_task_call(&self, call: &ExprCall) -> Option<(String, String)> {
        if let Expr::Name(ExprName { id, .. }) = call.func.as_ref() {
            let name = id.to_string();
            let script = self
                .task_functions
                .get(id.as_str())
                .and_then(|p| p.clone())
                .unwrap_or_else(|| name.clone());
            Some((name, script))
        } else {
            None
        }
    }

    fn expr_to_source(expr: &Expr) -> String {
        match expr {
            Expr::Compare(c) => {
                let left = Self::expr_to_source(&c.left);
                if let Some(comparator) = c.comparators.first() {
                    let right = Self::expr_to_source(comparator);
                    let op = match c.ops.first() {
                        Some(rustpython_parser::ast::CmpOp::Gt) => ">",
                        Some(rustpython_parser::ast::CmpOp::Lt) => "<",
                        Some(rustpython_parser::ast::CmpOp::GtE) => ">=",
                        Some(rustpython_parser::ast::CmpOp::LtE) => "<=",
                        Some(rustpython_parser::ast::CmpOp::Eq) => "==",
                        Some(rustpython_parser::ast::CmpOp::NotEq) => "!=",
                        Some(rustpython_parser::ast::CmpOp::In) => "in",
                        Some(rustpython_parser::ast::CmpOp::NotIn) => "not in",
                        Some(rustpython_parser::ast::CmpOp::Is) => "is",
                        Some(rustpython_parser::ast::CmpOp::IsNot) => "is not",
                        None => "?",
                    };
                    format!("{left} {op} {right}")
                } else {
                    left
                }
            }
            Expr::Subscript(s) => {
                let value = Self::expr_to_source(&s.value);
                let slice = Self::expr_to_source(&s.slice);
                format!("{value}[{slice}]")
            }
            Expr::Attribute(a) => {
                let value = Self::expr_to_source(&a.value);
                format!("{value}.{}", a.attr)
            }
            Expr::Name(n) => n.id.to_string(),
            Expr::Constant(c) => match &c.value {
                rustpython_parser::ast::Constant::Str(s) => format!("\"{s}\""),
                rustpython_parser::ast::Constant::Int(i) => i.to_string(),
                rustpython_parser::ast::Constant::Float(f) => f.to_string(),
                rustpython_parser::ast::Constant::Bool(b) => b.to_string(),
                rustpython_parser::ast::Constant::None => "None".to_string(),
                _ => "...".to_string(),
            },
            _ => "...".to_string(),
        }
    }

    /// Check if a statement body contains any task function calls (recursively)
    fn body_contains_step(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.stmt_contains_step(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_contains_step(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Expr(StmtExpr { value, .. }) => self.expr_contains_step(value),
            Stmt::Assign(a) => self.expr_contains_step(&a.value),
            Stmt::If(s) => self.body_contains_step(&s.body) || self.body_contains_step(&s.orelse),
            Stmt::For(s) => self.body_contains_step(&s.body) || self.body_contains_step(&s.orelse),
            Stmt::While(s) => {
                self.body_contains_step(&s.body) || self.body_contains_step(&s.orelse)
            }
            Stmt::Try(s) => {
                self.body_contains_step(&s.body)
                    || self.body_contains_step(&s.orelse)
                    || self.body_contains_step(&s.finalbody)
                    || s.handlers.iter().any(|h| match h {
                        rustpython_parser::ast::ExceptHandler::ExceptHandler(eh) => {
                            self.body_contains_step(&eh.body)
                        }
                    })
            }
            Stmt::TryStar(s) => {
                self.body_contains_step(&s.body)
                    || self.body_contains_step(&s.orelse)
                    || self.body_contains_step(&s.finalbody)
                    || s.handlers.iter().any(|h| match h {
                        rustpython_parser::ast::ExceptHandler::ExceptHandler(eh) => {
                            self.body_contains_step(&eh.body)
                        }
                    })
            }
            Stmt::Return(_) => false,
            _ => false,
        }
    }

    fn expr_contains_step(&self, expr: &Expr) -> bool {
        if self.is_task_fn_call(expr) {
            return true;
        }
        match expr {
            Expr::Await(ExprAwait { value, .. }) => self.expr_contains_step(value),
            Expr::Call(call) => {
                if self.is_task_fn_call(&Expr::Call(call.clone())) {
                    return true;
                }
                if Self::is_asyncio_gather_call(&Expr::Call(call.clone())) {
                    return call.args.iter().any(|a| self.expr_contains_step(a));
                }
                false
            }
            _ => false,
        }
    }

    /// Walk a list of statements, returning (first_node_id, last_node_id)
    fn walk_body(&mut self, body: &[Stmt]) -> Option<(String, String)> {
        let mut first_id: Option<String> = None;
        let mut prev_id: Option<String> = None;

        for stmt in body {
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
            Stmt::Expr(StmtExpr { value, .. }) => self.walk_expr_stmt(value),
            Stmt::Assign(a) => self.walk_expr_stmt(&a.value),
            Stmt::If(if_stmt) => self.walk_if(if_stmt),
            Stmt::For(for_stmt) => self.walk_for(for_stmt),
            Stmt::While(while_stmt) => self.walk_while(while_stmt),
            Stmt::Try(try_stmt) => self.walk_try(try_stmt),
            Stmt::TryStar(try_stmt) => self.walk_try_star(try_stmt),
            Stmt::Return(ret) => self.walk_return(ret),
            Stmt::FunctionDef(_) | Stmt::AsyncFunctionDef(_) => {
                if self.stmt_contains_step(stmt) {
                    self.errors.push(validation::error_step_in_nested_function(
                        self.line_of_stmt(stmt),
                    ));
                }
                None
            }
            _ => None,
        }
    }

    fn walk_expr_stmt(&mut self, expr: &Expr) -> Option<(String, String)> {
        // await task_fn(...)
        if let Expr::Await(ExprAwait { value, .. }) = expr {
            // await task_fn(...)
            if let Expr::Call(call) = value.as_ref() {
                if self.is_task_fn_call(&Expr::Call(call.clone())) {
                    return self.emit_step(call, expr);
                }
            }
            // await asyncio.gather(task_fn(...), task_fn(...), ...)
            if Self::is_asyncio_gather_call(value) {
                if let Expr::Call(gather_call) = value.as_ref() {
                    return self.emit_parallel(gather_call, expr);
                }
            }
        }

        // Bare task_fn() without await — validation error
        if self.is_task_fn_call(expr) {
            self.errors
                .push(validation::error_missing_await(self.line_of_expr(expr)));
        }

        None
    }

    fn emit_step(&mut self, call: &ExprCall, expr: &Expr) -> Option<(String, String)> {
        if self.in_try {
            self.errors
                .push(validation::error_step_in_try(self.line_of_expr(expr)));
            return None;
        }
        if self.in_while {
            self.errors
                .push(validation::error_step_in_while(self.line_of_expr(expr)));
            return None;
        }
        if self.in_nested_func {
            self.errors.push(validation::error_step_in_nested_function(
                self.line_of_expr(expr),
            ));
            return None;
        }
        if self.in_comprehension {
            self.errors.push(validation::error_step_in_comprehension(
                self.line_of_expr(expr),
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
            line: self.line_of_expr(expr),
        });
        Some((node_id.clone(), node_id))
    }

    fn emit_parallel(&mut self, gather_call: &ExprCall, expr: &Expr) -> Option<(String, String)> {
        if self.in_try {
            self.errors
                .push(validation::error_step_in_try(self.line_of_expr(expr)));
            return None;
        }
        if self.in_while {
            self.errors
                .push(validation::error_step_in_while(self.line_of_expr(expr)));
            return None;
        }

        let line = self.line_of_expr(expr);
        let start_id = self.next_id();
        let start_node_id = self.add_node(DagNode {
            id: start_id.clone(),
            node_type: DagNodeType::ParallelStart,
            label: "parallel".to_string(),
            line,
        });

        let mut step_ids = Vec::new();
        for arg in &gather_call.args {
            // Each arg should be task_fn(...)
            if let Expr::Call(call) = arg {
                if self.is_task_fn_call(&Expr::Call(call.clone())) {
                    let (name, script) = self
                        .extract_step_info_from_task_call(call)
                        .unwrap_or(("unknown".into(), "unknown".into()));
                    let step_id = self.next_id();
                    let node_id = self.add_node(DagNode {
                        id: step_id.clone(),
                        node_type: DagNodeType::Step { name: name.clone(), script },
                        label: name,
                        line: self.line_of_expr(arg),
                    });
                    self.add_edge(&start_node_id, &node_id, None);
                    step_ids.push(node_id);
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

    fn walk_if(&mut self, if_stmt: &StmtIf) -> Option<(String, String)> {
        let has_steps_in_body = self.body_contains_step(&if_stmt.body);
        let has_steps_in_else = self.body_contains_step(&if_stmt.orelse);

        if !has_steps_in_body && !has_steps_in_else {
            return None;
        }

        let line = self.line_index.line_of(if_stmt.range.start().to_usize());
        let condition_source = Self::expr_to_source(&if_stmt.test);

        let branch_id = self.next_id();
        let branch_node_id = self.add_node(DagNode {
            id: branch_id.clone(),
            node_type: DagNodeType::Branch { condition_source },
            label: "if".to_string(),
            line,
        });

        let merge_id = format!("{branch_id}_merge");

        let mut last_ids = Vec::new();

        if let Some((true_first, true_last)) = self.walk_body(&if_stmt.body) {
            self.add_edge(&branch_node_id, &true_first, Some("true".to_string()));
            last_ids.push(true_last);
        } else {
            last_ids.push(branch_node_id.clone());
        }

        if !if_stmt.orelse.is_empty() {
            if let Some((else_first, else_last)) = self.walk_body(&if_stmt.orelse) {
                self.add_edge(&branch_node_id, &else_first, Some("false".to_string()));
                last_ids.push(else_last);
            } else {
                last_ids.push(branch_node_id.clone());
            }
        }

        if last_ids.len() == 1 {
            Some((branch_node_id, last_ids.into_iter().next().unwrap()))
        } else {
            Some((branch_node_id, merge_id))
        }
    }

    fn walk_for(&mut self, for_stmt: &StmtFor) -> Option<(String, String)> {
        if !self.body_contains_step(&for_stmt.body) {
            return None;
        }

        let line = self.line_index.line_of(for_stmt.range.start().to_usize());
        let iter_source = Self::expr_to_source(&for_stmt.iter);

        let start_id = self.next_id();
        let start_node_id = self.add_node(DagNode {
            id: start_id.clone(),
            node_type: DagNodeType::LoopStart { iter_source },
            label: "for".to_string(),
            line,
        });

        if let Some((body_first, body_last)) = self.walk_body(&for_stmt.body) {
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

    fn walk_while(&mut self, while_stmt: &StmtWhile) -> Option<(String, String)> {
        if self.body_contains_step(&while_stmt.body) {
            let line = self.line_index.line_of(while_stmt.range.start().to_usize());
            self.errors.push(validation::error_step_in_while(line));
        }
        None
    }

    fn walk_try(&mut self, try_stmt: &StmtTry) -> Option<(String, String)> {
        let has_steps = self.body_contains_step(&try_stmt.body)
            || self.body_contains_step(&try_stmt.orelse)
            || self.body_contains_step(&try_stmt.finalbody)
            || try_stmt.handlers.iter().any(|h| match h {
                rustpython_parser::ast::ExceptHandler::ExceptHandler(eh) => {
                    self.body_contains_step(&eh.body)
                }
            });

        if has_steps {
            let line = self.line_index.line_of(try_stmt.range.start().to_usize());
            self.errors.push(validation::error_step_in_try(line));
        }
        None
    }

    fn walk_try_star(&mut self, try_stmt: &StmtTryStar) -> Option<(String, String)> {
        let has_steps = self.body_contains_step(&try_stmt.body)
            || self.body_contains_step(&try_stmt.orelse)
            || self.body_contains_step(&try_stmt.finalbody)
            || try_stmt.handlers.iter().any(|h| match h {
                rustpython_parser::ast::ExceptHandler::ExceptHandler(eh) => {
                    self.body_contains_step(&eh.body)
                }
            });

        if has_steps {
            let line = self.line_index.line_of(try_stmt.range.start().to_usize());
            self.errors.push(validation::error_step_in_try(line));
        }
        None
    }

    fn walk_return(&mut self, ret: &StmtReturn) -> Option<(String, String)> {
        let line = self.line_index.line_of(ret.range.start().to_usize());
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

/// Extract workflow function parameters (no longer skips ctx)
fn extract_params(args: &rustpython_parser::ast::Arguments) -> Vec<Param> {
    let mut params = Vec::new();
    for arg_with_default in args.args.iter().chain(args.posonlyargs.iter()) {
        let name = arg_with_default.def.arg.to_string();
        let typ = arg_with_default
            .def
            .annotation
            .as_ref()
            .map(|ann| WacWalker::expr_to_source(ann));
        params.push(Param { name, typ });
    }
    params
}

pub fn parse_python_workflow(code: &str) -> Result<WorkflowDag, Vec<CompileError>> {
    let ast = rustpython_parser::ast::Suite::parse(code, "<workflow>")
        .map_err(|e| vec![CompileError { message: format!("Parse error: {e}"), line: 0 }])?;

    // First pass: collect @task functions
    let task_functions = collect_task_functions(&ast);

    // Find the @workflow async def
    let workflow_fn = ast.iter().find_map(|stmt| {
        if let Stmt::AsyncFunctionDef(func) = stmt {
            let has_workflow_decorator = func.decorator_list.iter().any(|dec| {
                if let Expr::Name(ExprName { id, .. }) = dec {
                    id.as_str() == "workflow"
                } else {
                    false
                }
            });
            if has_workflow_decorator {
                return Some(func);
            }
        }
        // Also check non-async for error reporting
        if let Stmt::FunctionDef(func) = stmt {
            let has_workflow_decorator = func.decorator_list.iter().any(|dec| {
                if let Expr::Name(ExprName { id, .. }) = dec {
                    id.as_str() == "workflow"
                } else {
                    false
                }
            });
            if has_workflow_decorator {
                return None; // Will be reported as not-async below
            }
        }
        None
    });

    // Check for non-async workflow function
    let non_async_workflow = ast.iter().find_map(|stmt| {
        if let Stmt::FunctionDef(func) = stmt {
            let has_workflow_decorator = func.decorator_list.iter().any(|dec| {
                if let Expr::Name(ExprName { id, .. }) = dec {
                    id.as_str() == "workflow"
                } else {
                    false
                }
            });
            if has_workflow_decorator {
                let line_index = LineIndex::new(code);
                return Some(line_index.line_of(func.range.start().to_usize()));
            }
        }
        None
    });

    if let Some(line) = non_async_workflow {
        if workflow_fn.is_none() {
            return Err(vec![validation::error_not_async(line)]);
        }
    }

    let workflow_fn = workflow_fn.ok_or_else(|| {
        vec![CompileError { message: "No @workflow async function found.".to_string(), line: 0 }]
    })?;

    let params = extract_params(&workflow_fn.args);
    let source_hash = compute_source_hash(code);

    let mut walker = WacWalker::new(code, task_functions);
    walker.walk_body(&workflow_fn.body);

    if !walker.errors.is_empty() {
        return Err(walker.errors);
    }

    Ok(WorkflowDag { nodes: walker.nodes, edges: walker.edges, params, source_hash })
}

fn compute_source_hash(code: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
}

trait ToUsize {
    fn to_usize(self) -> usize;
}

impl ToUsize for rustpython_parser::text_size::TextSize {
    fn to_usize(self) -> usize {
        u32::from(self) as usize
    }
}
