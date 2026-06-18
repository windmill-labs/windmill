pub mod dag;
pub mod python;
pub mod typescript;
pub mod validation;

use dag::WorkflowDag;
use validation::CompileError;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
pub enum ParseResult {
    #[serde(rename = "success")]
    Success(WorkflowDag),
    #[serde(rename = "error")]
    Error { errors: Vec<CompileError> },
}

pub fn parse_workflow(code: &str, language: &str) -> ParseResult {
    let result = match language {
        "python" | "python3" | "py" => python::parse_python_workflow(code),
        "typescript" | "ts" | "deno" | "bun" => typescript::parse_ts_workflow(code),
        _ => Err(vec![CompileError {
            message: format!("Unsupported language: {language}"),
            line: 0,
        }]),
    };

    match result {
        Ok(dag) => ParseResult::Success(dag),
        Err(errors) => ParseResult::Error { errors },
    }
}
