use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

pub fn error_step_in_try(line: usize) -> CompileError {
    CompileError {
        message:
            "Task calls inside try/except are not allowed. Steps have built-in error handling."
                .to_string(),
        line,
    }
}

pub fn error_step_in_while(line: usize) -> CompileError {
    CompileError {
        message: "Task calls inside while loops are not allowed. Use for loops instead."
            .to_string(),
        line,
    }
}

pub fn error_step_in_nested_function(line: usize) -> CompileError {
    CompileError {
        message: "Task calls inside nested functions, closures, or lambdas are not allowed."
            .to_string(),
        line,
    }
}

pub fn error_step_in_comprehension(line: usize) -> CompileError {
    CompileError { message: "Task calls inside comprehensions are not allowed.".to_string(), line }
}

pub fn error_not_async(line: usize) -> CompileError {
    CompileError { message: "Workflow function must be async.".to_string(), line }
}

pub fn error_missing_await(line: usize) -> CompileError {
    CompileError {
        message:
            "Task calls must be awaited directly or used inside asyncio.gather()/Promise.all()."
                .to_string(),
        line,
    }
}

pub fn error_step_in_catch(line: usize) -> CompileError {
    CompileError {
        message:
            "Task calls inside catch blocks are not allowed. Steps have built-in error handling."
                .to_string(),
        line,
    }
}
