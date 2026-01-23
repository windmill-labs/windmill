//! Simple script executor for local mode
//!
//! This is a minimal executor that supports a few languages for demonstration.
//! A full implementation would integrate with windmill-worker's execution logic.

use std::process::Stdio;
use tokio::process::Command;
use tokio::io::AsyncReadExt;

use crate::error::{LocalError, Result};
use crate::jobs::ScriptLang;

/// Result of script execution
#[derive(Debug)]
pub struct ExecutionResult {
    pub success: bool,
    pub result: serde_json::Value,
    pub logs: String,
}

/// Execute a script with the given language and arguments
pub async fn execute_script(
    language: ScriptLang,
    code: &str,
    args: &serde_json::Value,
) -> Result<ExecutionResult> {
    match language {
        ScriptLang::Bash => execute_bash(code, args).await,
        ScriptLang::Python3 => execute_python(code, args).await,
        ScriptLang::Deno => execute_deno(code, args).await,
        ScriptLang::Bun => execute_bun(code, args).await,
        _ => Ok(ExecutionResult {
            success: false,
            result: serde_json::json!({
                "error": format!("Language {:?} not supported in local mode yet", language)
            }),
            logs: format!("Language {:?} not supported in local mode", language),
        }),
    }
}

/// Execute a bash script
async fn execute_bash(code: &str, args: &serde_json::Value) -> Result<ExecutionResult> {
    // Create environment variables from args
    let mut env_vars = Vec::new();
    if let serde_json::Value::Object(map) = args {
        for (key, value) in map {
            let val_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            env_vars.push((key.to_uppercase(), val_str));
        }
    }

    let mut child = Command::new("bash")
        .arg("-c")
        .arg(code)
        .envs(env_vars)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LocalError::Execution(format!("Failed to spawn bash: {}", e)))?;

    let status = child
        .wait()
        .await
        .map_err(|e| LocalError::Execution(format!("Failed to wait for bash: {}", e)))?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout).await.ok();
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr).await.ok();
    }

    let logs = format!("{}{}", stdout, stderr);
    let success = status.success();

    // Try to parse stdout as JSON, otherwise use as string
    let result = if success {
        let trimmed = stdout.trim();
        serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!(trimmed))
    } else {
        serde_json::json!({ "error": stderr.trim() })
    };

    Ok(ExecutionResult {
        success,
        result,
        logs,
    })
}

/// Execute a Python script
async fn execute_python(code: &str, args: &serde_json::Value) -> Result<ExecutionResult> {
    // Wrap the code to handle args and return JSON result
    let wrapped_code = format!(
        r#"
import json
import sys

# Args passed as JSON
args = json.loads('''{}''')

# User code
{}

# Call main if it exists
if 'main' in dir():
    result = main(**args)
    print(json.dumps(result))
"#,
        serde_json::to_string(args)?,
        code
    );

    let mut child = Command::new("python3")
        .arg("-c")
        .arg(&wrapped_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LocalError::Execution(format!("Failed to spawn python3: {}", e)))?;

    let status = child
        .wait()
        .await
        .map_err(|e| LocalError::Execution(format!("Failed to wait for python3: {}", e)))?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout).await.ok();
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr).await.ok();
    }

    let logs = format!("{}{}", stdout, stderr);
    let success = status.success();

    let result = if success {
        let trimmed = stdout.trim();
        // Get the last line as result (in case there's debug output)
        let last_line = trimmed.lines().last().unwrap_or("");
        serde_json::from_str(last_line).unwrap_or_else(|_| serde_json::json!(trimmed))
    } else {
        serde_json::json!({ "error": stderr.trim() })
    };

    Ok(ExecutionResult {
        success,
        result,
        logs,
    })
}

/// Execute a Deno/TypeScript script
async fn execute_deno(code: &str, args: &serde_json::Value) -> Result<ExecutionResult> {
    // Wrap the code to handle args and return JSON result
    let wrapped_code = format!(
        r#"
const args = {};

{}

// Call main if it exists
if (typeof main === 'function') {{
    const result = await main(args);
    console.log(JSON.stringify(result));
}}
"#,
        serde_json::to_string(args)?,
        code
    );

    let mut child = Command::new("deno")
        .arg("eval")
        .arg("--unstable")
        .arg(&wrapped_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LocalError::Execution(format!("Failed to spawn deno: {}", e)))?;

    let status = child
        .wait()
        .await
        .map_err(|e| LocalError::Execution(format!("Failed to wait for deno: {}", e)))?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout).await.ok();
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr).await.ok();
    }

    let logs = format!("{}{}", stdout, stderr);
    let success = status.success();

    let result = if success {
        let trimmed = stdout.trim();
        let last_line = trimmed.lines().last().unwrap_or("");
        serde_json::from_str(last_line).unwrap_or_else(|_| serde_json::json!(trimmed))
    } else {
        serde_json::json!({ "error": stderr.trim() })
    };

    Ok(ExecutionResult {
        success,
        result,
        logs,
    })
}

/// Execute a Bun/TypeScript script
async fn execute_bun(code: &str, args: &serde_json::Value) -> Result<ExecutionResult> {
    // Similar to Deno but using Bun
    let wrapped_code = format!(
        r#"
const args = {};

{}

// Call main if it exists
if (typeof main === 'function') {{
    const result = await main(args);
    console.log(JSON.stringify(result));
}}
"#,
        serde_json::to_string(args)?,
        code
    );

    let mut child = Command::new("bun")
        .arg("eval")
        .arg(&wrapped_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| LocalError::Execution(format!("Failed to spawn bun: {}", e)))?;

    let status = child
        .wait()
        .await
        .map_err(|e| LocalError::Execution(format!("Failed to wait for bun: {}", e)))?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_string(&mut stdout).await.ok();
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_string(&mut stderr).await.ok();
    }

    let logs = format!("{}{}", stdout, stderr);
    let success = status.success();

    let result = if success {
        let trimmed = stdout.trim();
        let last_line = trimmed.lines().last().unwrap_or("");
        serde_json::from_str(last_line).unwrap_or_else(|_| serde_json::json!(trimmed))
    } else {
        serde_json::json!({ "error": stderr.trim() })
    };

    Ok(ExecutionResult {
        success,
        result,
        logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_execution() {
        let result = execute_script(
            ScriptLang::Bash,
            "echo 42",
            &serde_json::json!({}),
        )
        .await
        .unwrap();

        assert!(result.success);
        // Output "42" is parsed as JSON number
        assert_eq!(result.result, serde_json::json!(42));
    }

    #[tokio::test]
    async fn test_bash_with_args() {
        let result = execute_script(
            ScriptLang::Bash,
            "echo $NAME",
            &serde_json::json!({"name": "world"}),
        )
        .await
        .unwrap();

        assert!(result.success);
        assert_eq!(result.result, serde_json::json!("world"));
    }

    #[tokio::test]
    async fn test_bash_json_output() {
        let result = execute_script(
            ScriptLang::Bash,
            r#"echo '{"key": "value"}'"#,
            &serde_json::json!({}),
        )
        .await
        .unwrap();

        assert!(result.success);
        assert_eq!(result.result, serde_json::json!({"key": "value"}));
    }
}
