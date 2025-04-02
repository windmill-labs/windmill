use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

// --- JSON-RPC Structures ---

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

// Define the part that is *either* a result or an error
#[derive(Serialize, Debug)]
#[serde(untagged)] // Important: Don't add a type field for the enum itself
enum ResponseBody {
    Success { result: Value },
    Error { error: JsonRpcError },
}

// Modify the main response struct to use the enum
#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(flatten)] // Important: Embed the Success or Error fields directly
    body: ResponseBody,
}

#[derive(Serialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

// --- MCP Specific Structures ---

#[derive(Serialize, Debug)]
struct ServerInfo {
    name: String,
    version: String,
}

#[derive(Serialize, Debug)]
struct ServerCapabilities {
    // Simplified: only enabling tools for this example
    tools: HashMap<String, Value>, // Use Value for flexibility, could be boolean or object
}

#[derive(Serialize, Debug)]
struct InitializeResult {
    #[serde(rename = "serverInfo")]
    server_info: ServerInfo,
    capabilities: ServerCapabilities,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value, // JSON schema as a Value
}

#[derive(Serialize, Debug)]
struct ListToolsResult {
    tools: Vec<Tool>,
}

#[derive(Serialize, Debug)]
struct TextContent {
    #[serde(rename = "type")]
    content_type: String, // "text"
    text: String,
}

#[derive(Serialize, Debug)]
struct CallToolResult {
    content: Vec<TextContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    is_error: Option<bool>, // Optional, defaults to false if None
}

// --- Error Codes ---
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;
const PARSE_ERROR: i32 = -32700;

// --- Main Server Logic ---

// Helper function to flush stderr, crucial for seeing logs immediately
fn flush_stderr() {
    let _ = io::stderr().flush(); // Ignore error on stderr flush
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    // Use BufReader for potentially better performance with read_line
    let mut reader = io::BufReader::new(stdin.lock());

    eprintln!("Rust MCP Hello Server started. Listening on stdio...");
    flush_stderr();

    // Replace the for loop:
    // for line_result in reader.lines() { ... }

    // With a while let loop:
    loop {
        let mut line_buffer = String::new(); // Create buffer inside the loop

        // Explicitly read the next line
        eprintln!("DEBUG: Top of loop, waiting for read_line...");
        flush_stderr();
        match reader.read_line(&mut line_buffer) {
            Ok(0) => {
                eprintln!("DEBUG: read_line returned Ok(0) - EOF detected.");
                flush_stderr();
                eprintln!("Input stream closed by client (EOF). Shutting down.");
                flush_stderr();
                break; // Exit the loop cleanly
            }
            Ok(bytes_read) => {
                eprintln!("DEBUG: read_line returned Ok({})", bytes_read);
                flush_stderr();
                // Successfully read a line, trim whitespace (like the newline)
                let line = line_buffer.trim();
                eprintln!("DEBUG: Read line content (trimmed): '{}'", line);
                flush_stderr();
                if line.is_empty() {
                    // Skip empty lines if they occur
                    eprintln!("DEBUG: Line was empty, continuing loop.");
                    flush_stderr();
                    continue;
                }

                // --- Existing processing logic ---
                eprintln!("DEBUG: Attempting to parse JSON...");
                flush_stderr();
                let request: JsonRpcRequest = match serde_json::from_str(line) {
                    Ok(req) => {
                        eprintln!("DEBUG: JSON parsed successfully.");
                        flush_stderr();
                        req
                    }
                    Err(e) => {
                        eprintln!("DEBUG: JSON parsing failed: {}", e);
                        flush_stderr();
                        eprintln!("Error parsing JSON: {}", e);
                        flush_stderr();
                        let _ = send_error_response(
                            None,
                            PARSE_ERROR,
                            "Failed to parse JSON request",
                            &mut stdout,
                        )?;
                        continue; // Try next line
                    }
                };

                eprintln!("Received request: {:?}", request.method);

                eprintln!("DEBUG: Handling method '{}'...", request.method);
                flush_stderr();
                let response = match request.method.as_str() {
                    "initialize" => handle_initialize(&request),
                    "initialized" => {
                        eprintln!("DEBUG: Matched 'initialized' notification.");
                        flush_stderr();
                        eprintln!("Connection initialized by client.");
                        flush_stderr();
                        None
                    }
                    "tools/list" => handle_tools_list(&request),
                    "tools/call" => handle_tools_call(&request),
                    _ => {
                        eprintln!("DEBUG: Matched unknown method.");
                        flush_stderr();
                        eprintln!("Unknown method: {}", request.method);
                        flush_stderr();
                        Some(create_error_response(
                            request.id.clone(),
                            METHOD_NOT_FOUND,
                            "Method not found",
                        ))
                    }
                };
                eprintln!("DEBUG: Method handling complete.");
                flush_stderr();

                if let Some(resp) = response {
                    eprintln!("DEBUG: Response generated, attempting serialization...");
                    flush_stderr();
                    match serde_json::to_string(&resp) {
                        Ok(json_resp) => {
                            eprintln!("DEBUG: Serialization successful. Attempting to write...");
                            flush_stderr();
                            if let Err(e) = writeln!(stdout, "{}", json_resp) {
                                eprintln!("DEBUG: Error writing response: {}", e);
                                flush_stderr();
                                eprintln!("Error writing response: {}", e);
                                flush_stderr();
                                return Err(e); // Exit if we can't write
                            }
                            eprintln!("DEBUG: Write successful. Attempting to flush stdout...");
                            flush_stderr();
                            if let Err(e) = stdout.flush() {
                                eprintln!("DEBUG: Error flushing stdout: {}", e);
                                flush_stderr();
                                eprintln!("Error flushing stdout: {}", e);
                                flush_stderr();
                                return Err(e); // Exit if we can't flush
                            }
                            eprintln!("Sent response for ID: {:?}", resp.id);
                            flush_stderr();
                            eprintln!("DEBUG: Stdout flush successful.");
                            flush_stderr(); // Check if this prints
                        }
                        Err(e) => {
                            eprintln!("DEBUG: Error serializing response: {}", e);
                            flush_stderr();
                            eprintln!("Error serializing response: {}", e);
                            flush_stderr();
                            let _ = send_error_response(
                                request.id,
                                INTERNAL_ERROR,
                                "Failed to serialize response",
                                &mut stdout,
                            )?;
                        }
                    }
                } else {
                    eprintln!("DEBUG: No response generated (likely a notification).");
                    flush_stderr();
                }
                // CRITICAL: See if this line is reached after initialize response
                eprintln!(
                    "DEBUG: Reached end of loop iteration for method '{}'.",
                    request.method
                );
                flush_stderr();
            }
            Err(e) => {
                eprintln!("DEBUG: read_line returned Err: {}", e);
                flush_stderr();
                eprintln!("Error reading line from stdin: {}", e);
                flush_stderr();
                // Attempt to send an error if possible, then break as the stream might be unusable
                let _ = send_error_response(
                    None,
                    PARSE_ERROR,
                    "Failed to read input line",
                    &mut stdout,
                )?;
                break; // Exit the loop on read error
            }
        }
        // Clear the buffer for the next read iteration
        // line_buffer.clear(); // Already done by creating buffer inside loop
    } // End of loop

    eprintln!("Main loop finished. Server shutting down.");
    flush_stderr();
    Ok(())
}

// --- Request Handlers ---

fn handle_initialize(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    eprintln!("Handling initialize request...");
    flush_stderr();
    let result = InitializeResult {
        server_info: ServerInfo {
            name: "rust-hello-server".to_string(),
            version: "0.1.0".to_string(),
        },
        capabilities: ServerCapabilities {
            tools: HashMap::from([("listChanged".to_string(), Value::Bool(false))]),
        },
    };
    Some(create_success_response(
        request.id.clone(),
        serde_json::to_value(result).unwrap(), // Use unwrap for simplicity in example
    ))
}

fn handle_tools_list(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    eprintln!("Handling tools/list request...");
    flush_stderr();
    let hello_tool = Tool {
        name: "hello_world".to_string(),
        description: "Returns a simple 'hello world' message.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {} // No input parameters
        }),
    };
    let result = ListToolsResult { tools: vec![hello_tool] };
    Some(create_success_response(
        request.id.clone(),
        serde_json::to_value(result).unwrap(),
    ))
}

fn handle_tools_call(request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    eprintln!("Handling tools/call request...");
    flush_stderr();
    let params = match request.params.as_ref() {
        Some(p) => p,
        None => {
            return Some(create_error_response(
                request.id.clone(),
                INVALID_PARAMS,
                "Missing params for tools/call",
            ))
        }
    };

    let tool_name = params.get("name").and_then(|v| v.as_str());
    // let _tool_args = params.get("arguments"); // We don't need args for hello_world

    match tool_name {
        Some("hello_world") => {
            let result = CallToolResult {
                content: vec![TextContent {
                    content_type: "text".to_string(),
                    text: "hello world".to_string(),
                }],
                is_error: None, // Indicate success
            };
            Some(create_success_response(
                request.id.clone(),
                serde_json::to_value(result).unwrap(),
            ))
        }
        Some(other) => {
            eprintln!("Tool not found: {}", other);
            flush_stderr();
            Some(create_error_response(
                request.id.clone(),
                METHOD_NOT_FOUND, // Or a custom tool error code
                format!("Tool '{}' not found", other),
            ))
        }
        None => Some(create_error_response(
            request.id.clone(),
            INVALID_PARAMS,
            "Missing 'name' field in tools/call params",
        )),
    }
}

// --- Response Helpers ---

fn create_success_response(id: Option<Value>, result_value: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        body: ResponseBody::Success { result: result_value }, // Use the Success variant
    }
}

fn create_error_response(
    id: Option<Value>,
    code: i32,
    message: impl Into<String>,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        body: ResponseBody::Error {
            // Use the Error variant
            error: JsonRpcError { code, message: message.into(), data: None },
        },
    }
}

// Update the standalone error sender too
fn send_error_response(
    id: Option<Value>,
    code: i32,
    message: &str,
    mut stdout: &io::Stdout,
) -> io::Result<()> {
    let error_resp = create_error_response(id, code, message); // This now returns the new structure
    match serde_json::to_string(&error_resp) {
        Ok(json_resp) => {
            writeln!(stdout, "{}", json_resp)?;
            stdout.flush()
        }
        Err(e) => {
            eprintln!(
                "Critical error: Failed to serialize even the error response: {}",
                e
            );
            // Cannot send JSON error, maybe write plain text error?
            writeln!(stdout, "{{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{{\"code\":-32000,\"message\":\"Internal Server Error: Failed to serialize error response\"}}}}")?; // Basic JSON error
            stdout.flush()
        }
    }
}
