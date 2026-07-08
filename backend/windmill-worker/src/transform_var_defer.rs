//! Syntactic classifier for flow input transform expressions whose `variable()`
//! calls can be deferred to the child job.
//!
//! When a transform is exactly `variable(<path>)` or a template literal whose
//! dynamic slots are direct `variable(<path>)` calls, the secret value does not
//! need to be fetched at push time: the transform compiles to a `$var:<path>`
//! arg or a `{"$interpolate": [...]}` arg that the child job's worker resolves
//! (and masks) when it starts, so the raw value never lands in the child's
//! stored args.
//!
//! Classification is purely syntactic on purpose: evaluating with placeholder
//! values instead would silently change the semantics of value-dependent
//! expressions like `variable("f/x") || "default"` (a placeholder is always
//! truthy). Anything that does not match the two shapes above keeps eager
//! evaluation.

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Spanned};
use swc_ecma_ast::{Callee, Expr, Lit};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};

/// The path argument of a `variable(...)` call.
#[derive(Debug, PartialEq)]
pub enum PathSpec {
    /// String literal path, known without evaluation.
    Literal(String),
    /// Source of a dynamic path expression, to be evaluated eagerly at push time.
    Expr(String),
}

#[derive(Debug, PartialEq)]
pub enum InterpolatePart {
    /// Static text from the template literal.
    Literal(String),
    /// A `variable(<path>)` slot, resolved by the child job.
    Var(PathSpec),
    /// Source of a non-variable slot, evaluated eagerly at push time.
    Expr(String),
}

#[derive(Debug, PartialEq)]
pub enum DeferPlan {
    /// The whole expression is `variable(<path>)` → `$var:<path>` arg.
    WholeVar(PathSpec),
    /// A template literal with `variable(<path>)` slots → `$interpolate` arg.
    Interpolate(Vec<InterpolatePart>),
}

/// Classify `expr` if its `variable()` calls can be deferred to the child job.
/// Returns `None` (keep eager evaluation) for anything but the two supported
/// shapes, including any expression that *computes* on a variable value.
pub fn classify_deferable_variables(expr: &str) -> Option<DeferPlan> {
    // Cheap pre-filter: no variable() call means nothing to defer.
    if !expr.contains("variable") {
        return None;
    }

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), expr.to_string());
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax { jsx: false, ..Default::default() }),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let ast = parser.parse_expr().ok()?;
    if !parser.take_errors().is_empty() {
        return None;
    }

    let snippet = |e: &dyn Spanned| cm.span_to_snippet(e.span()).ok();

    let ast = unwrap_expr(&ast);
    if let Some(path) = as_variable_call(ast, &|e| snippet(e)) {
        return Some(DeferPlan::WholeVar(path));
    }

    let Expr::Tpl(tpl) = ast else {
        return None;
    };
    let mut parts: Vec<InterpolatePart> = Vec::new();
    // A template literal with N slots has N+1 quasis; interleave them.
    for (i, quasi) in tpl.quasis.iter().enumerate() {
        // `cooked` is None when the literal contains invalid escapes.
        let text = quasi.cooked.as_ref()?.to_string();
        if !text.is_empty() {
            parts.push(InterpolatePart::Literal(text));
        }
        if let Some(slot) = tpl.exprs.get(i) {
            let slot = unwrap_expr(slot);
            if let Some(path) = as_variable_call(slot, &|e| snippet(e)) {
                parts.push(InterpolatePart::Var(path));
            } else {
                let src = snippet(&slot.span())?;
                // A slot that references `variable` but is not a direct call
                // computes on the value; the whole expression stays eager.
                if src.contains("variable") {
                    return None;
                }
                parts.push(InterpolatePart::Expr(src));
            }
        }
    }
    if !parts.iter().any(|p| matches!(p, InterpolatePart::Var(_))) {
        return None;
    }
    Some(DeferPlan::Interpolate(parts))
}

/// Strip parens and awaits: `(await variable(x))` classifies like `variable(x)`.
fn unwrap_expr(mut e: &Expr) -> &Expr {
    loop {
        match e {
            Expr::Paren(p) => e = &p.expr,
            Expr::Await(a) => e = &a.arg,
            _ => return e,
        }
    }
}

/// Match a direct `variable(<single arg>)` call on the bare `variable` identifier.
fn as_variable_call(
    e: &Expr,
    snippet: &dyn Fn(&dyn Spanned) -> Option<String>,
) -> Option<PathSpec> {
    let Expr::Call(call) = e else {
        return None;
    };
    let Callee::Expr(callee) = &call.callee else {
        return None;
    };
    let Expr::Ident(ident) = unwrap_expr(callee) else {
        return None;
    };
    if ident.sym.as_str() != "variable" || call.args.len() != 1 {
        return None;
    }
    let arg = &call.args[0];
    if arg.spread.is_some() {
        return None;
    }
    if let Expr::Lit(Lit::Str(s)) = unwrap_expr(&arg.expr) {
        return Some(PathSpec::Literal(s.value.to_string()));
    }
    let src = snippet(&arg.expr.span())?;
    // A nested variable() inside the path expression would compute on a
    // variable value; keep the whole expression eager.
    if src.contains("variable") {
        return None;
    }
    Some(PathSpec::Expr(src))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_var_literal() {
        assert_eq!(
            classify_deferable_variables(r#"variable("f/pw")"#),
            Some(DeferPlan::WholeVar(PathSpec::Literal("f/pw".to_string())))
        );
        assert_eq!(
            classify_deferable_variables(r#"(variable('f/pw'))"#),
            Some(DeferPlan::WholeVar(PathSpec::Literal("f/pw".to_string())))
        );
    }

    #[test]
    fn whole_var_dynamic_path() {
        assert_eq!(
            classify_deferable_variables(r#"variable("f/" + flow_input.env + "/pw")"#),
            Some(DeferPlan::WholeVar(PathSpec::Expr(
                r#""f/" + flow_input.env + "/pw""#.to_string()
            )))
        );
        assert_eq!(
            classify_deferable_variables(r#"variable(`f/${flow_input.env}/pw`)"#),
            Some(DeferPlan::WholeVar(PathSpec::Expr(
                r#"`f/${flow_input.env}/pw`"#.to_string()
            )))
        );
    }

    #[test]
    fn template_interpolation() {
        assert_eq!(
            classify_deferable_variables(r#"`mycli --password=${variable("f/pw")} -v`"#),
            Some(DeferPlan::Interpolate(vec![
                InterpolatePart::Literal("mycli --password=".to_string()),
                InterpolatePart::Var(PathSpec::Literal("f/pw".to_string())),
                InterpolatePart::Literal(" -v".to_string()),
            ]))
        );
    }

    #[test]
    fn template_with_eager_slots() {
        assert_eq!(
            classify_deferable_variables(
                r#"`--pw=${variable("f/pw")} --retries=${flow_input.n + 1}`"#
            ),
            Some(DeferPlan::Interpolate(vec![
                InterpolatePart::Literal("--pw=".to_string()),
                InterpolatePart::Var(PathSpec::Literal("f/pw".to_string())),
                InterpolatePart::Literal(" --retries=".to_string()),
                InterpolatePart::Expr("flow_input.n + 1".to_string()),
            ]))
        );
    }

    #[test]
    fn computing_on_value_stays_eager() {
        for expr in [
            r#"variable("f/pw") || "default""#,
            r#"variable("f/pw").slice(0, 4)"#,
            r#"variable("f/pw").length > 3 ? variable("f/pw") : "short""#,
            r#"`x=${variable("f/pw").trim()}`"#,
            r#"`x=${variable("f/pw") || "default"}`"#,
            r#"JSON.parse(variable("f/creds")).user"#,
            r#"variable(variable("f/indirect"))"#,
            r#"`x=${cond ? variable("f/a") : variable("f/b")}`"#,
        ] {
            assert_eq!(classify_deferable_variables(expr), None, "expr: {expr}");
        }
    }

    #[test]
    fn unrelated_expressions_stay_eager() {
        for expr in [
            r#"flow_input.x"#,
            r#"`hello ${flow_input.x}`"#,
            r#"results.a.variable"#,
            r#"my_variable + 1"#,
            r#"not valid js ("#,
        ] {
            assert_eq!(classify_deferable_variables(expr), None, "expr: {expr}");
        }
    }

    #[test]
    fn template_without_var_slots_stays_eager() {
        // contains the word variable only in a literal part
        assert_eq!(classify_deferable_variables(r#"`variable ${x}`"#), None);
    }
}
