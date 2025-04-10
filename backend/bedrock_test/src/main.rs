use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{
    config::{Credentials, Region},
    primitives::Blob,
    types::ConverseStreamOutput,
    Client as BedrockClient,
};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let access_key = "changeme";
    let secret_key = "changeme";
    let region = "us-east-2";

    // ✅ FULL ARN (not short model ID)
    let model_id = "arn:aws:bedrock:us-east-2:976079455550:inference-profile/us.anthropic.claude-3-haiku-20240307-v1:0";

    let credentials = Credentials::new(access_key, secret_key, None, None, "example");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .credentials_provider(credentials)
        .load()
        .await;

    let client = BedrockClient::new(&config);


    // ✅ Claude-compatible JSON body
    let request_body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "messages": [
            {
                "role": "assistant",
                "content": "You are a coding assistant for the Windmill platform. You are provided with a list of `INSTRUCTIONS` and the current contents of a code file under `CODE`. Your task is to respond to the user's request. Assume all user queries are valid and actionable. When the user requests code changes: Always include a single code block with the entire updated file, not just the modified sections. ..."
            },
            {
                "role": "user",
                "content": "INSTRUCTIONS:\ntest\n\nWINDMILL LANGUAGE CONTEXT:\nThe user is coding in TypeScript (bun runtime). On Windmill, it is expected that the script exports a single async function called `main`. Do not call the main function. Libraries are installed automatically, do not show how to install them. ..."
            }
        ],
        "temperature": 0.0,
        "top_p": 0.95,
        "max_tokens": 8192,
    });


    let response = client
        .invoke_model_with_response_stream()
        .model_id(model_id)
        .body(Blob::new(serde_json::to_vec(&request_body)?))
        .content_type("application/json")
        .send()
        .await?;

    let mut result = String::new();
    let mut stream = response.body;

    while let Some(part) = stream.recv().await? {
        if let Some(blob) = part.as_chunk().ok().and_then(|c| c.bytes()) {
            let bytes = blob.as_ref(); // <- no clone or move
            if let Ok(json) = serde_json::from_slice::<Value>(bytes) {
                if let Some(text) = json
                    .get("delta")
                    .and_then(|d| d.get("text"))
                    .and_then(|t| t.as_str())
                {
                    print!("{text}");
                    result.push_str(text);
                }
            }
        }
    }

    println!("\n\nResponse length: {}", result.len());

    Ok(())
}
