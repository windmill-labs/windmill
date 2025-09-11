use async_recursion::async_recursion;
use base64::Engine;
use regex::Regex;
use serde_json::value::RawValue;
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::{
    ai_providers::AIProvider,
    cache,
    client::AuthedClient,
    db::DB,
    error::{self, Error},
    flow_status::AgentAction,
    flows::{FlowModuleValue, Step},
    get_latest_hash_for_path,
    jobs::JobKind,
    s3_helpers::S3Object,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, Connection},
};
use windmill_queue::{flow_status::get_step_of_flow_status, CanceledBy, MiniPulledJob};

use crate::{
    ai::types::*,
    common::{build_args_map, OccupancyMetrics},
    handle_child::run_future_with_polling_update_job_poller,
    parse_sig_of_lang, JobCompletedSender,
};

lazy_static::lazy_static! {
    static ref TOOL_NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
}

/// Generate image from provider and extract base64 data
async fn generate_image_from_provider(
    provider: &ProviderWithResource,
    user_message: &str,
    system_prompt: Option<&str>,
    base_url: &str,
    api_key: &str,
    image: Option<&S3Object>,
    client: &AuthedClient,
    workspace_id: &str,
) -> error::Result<String> {
    match provider.kind {
        AIProvider::OpenAI => {
            // Build content array with text and optional image
            let mut content =
                vec![ImageGenerationContent::InputText { text: user_message.to_string() }];

            // Add image if provided
            if let Some(image) = image {
                if !image.s3.is_empty() {
                    // Download and encode S3 image to base64
                    let image_bytes = client
                        .download_s3_file(workspace_id, &image.s3, image.storage.clone())
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!("Failed to download S3 image: {}", e))
                        })?;

                    let base64_image =
                        base64::engine::general_purpose::STANDARD.encode(&image_bytes);
                    let image_data_url = format!("data:image/jpeg;base64,{}", base64_image);

                    content.push(ImageGenerationContent::InputImage { image_url: image_data_url });
                }
            }

            let image_request = ImageGenerationRequest {
                model: provider.get_model(),
                input: vec![ImageGenerationMessage { role: "user".to_string(), content }],
                instructions: system_prompt,
                tools: vec![ImageGenerationTool {
                    r#type: "image_generation".to_string(),
                    quality: Some("low".to_string()),
                    background: None,
                }],
            };

            let resp = HTTP_CLIENT
                .post(format!("{}/responses", base_url))
                .timeout(std::time::Duration::from_secs(120))
                .bearer_auth(api_key)
                .json(&image_request)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call OpenAI API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let image_response = resp.json::<OpenAIImageResponse>().await.map_err(|e| {
                        Error::internal_err(format!("Failed to parse OpenAI response: {}", e))
                    })?;

                    // Find the first image generation output
                    let image_generation_call = image_response
                        .output
                        .iter()
                        .find(|output| output.r#type == "image_generation_call" && output.status == "completed")
                        .and_then(|output| output.result.as_ref());

                    if let Some(base64_image) = image_generation_call {
                        Ok(base64_image.clone())
                    } else {
                        Err(Error::internal_err(
                            "No completed image output received from OpenAI".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "OpenAI API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        AIProvider::GoogleAI => {
            let is_imagen = provider.get_model().contains("imagen");

            let gemini_request = if is_imagen {
                // For Imagen models, we keep the simple prompt format (no image support)
                GeminiImageRequest {
                    instances: Some(vec![GeminiPredictContent {
                        prompt: user_message.trim().to_string(),
                    }]),
                    contents: None,
                }
            } else {
                // For Gemini models, build parts array with text and optional image
                let mut parts = vec![GeminiPart::Text { text: user_message.trim().to_string() }];

                if let Some(system_prompt) = system_prompt {
                    parts.insert(
                        0,
                        GeminiPart::Text {
                            text: format!("SYSTEM PROMPT: {}", system_prompt.trim().to_string()),
                        },
                    );
                }

                // Add image if provided
                if let Some(image) = image {
                    if !image.s3.is_empty() {
                        // Download and encode S3 image to base64
                        let image_bytes = client
                            .download_s3_file(workspace_id, &image.s3, image.storage.clone())
                            .await
                            .map_err(|e| {
                                Error::internal_err(format!("Failed to download S3 image: {}", e))
                            })?;

                        let base64_image =
                            base64::engine::general_purpose::STANDARD.encode(&image_bytes);

                        parts.push(GeminiPart::InlineData {
                            inline_data: GeminiInlineData {
                                mime_type: "image/jpeg".to_string(),
                                data: base64_image,
                            },
                        });
                    }
                }

                GeminiImageRequest {
                    instances: None,
                    contents: Some(vec![GeminiContent { parts }]),
                }
            };

            let url_suffix = if is_imagen {
                "predict"
            } else {
                "generateContent"
            };
            let gemini_url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:{}",
                provider.get_model(),
                url_suffix
            );

            let resp = HTTP_CLIENT
                .post(&gemini_url)
                .timeout(std::time::Duration::from_secs(120))
                .header("x-goog-api-key", api_key)
                .header("Content-Type", "application/json")
                .json(&gemini_request)
                .send()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to call Gemini API: {}", e)))?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let response_text = resp.text().await.map_err(|e| {
                        Error::internal_err(format!("Failed to read response text: {}", e))
                    })?;

                    let gemini_response: GeminiImageResponse = serde_json::from_str(&response_text)
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to parse Gemini response: {}. Raw response: {}",
                                e, response_text
                            ))
                        })?;

                    // Find the first candidate with inline image data
                    let mut image_data =
                        gemini_response.candidates.as_ref().and_then(|candidates| {
                            candidates.iter().find_map(|candidate| {
                                candidate.content.parts.iter().find_map(|part| {
                                    part.inline_data.as_ref().map(|data| &data.data)
                                })
                            })
                        });

                    if image_data.is_none() {
                        image_data = gemini_response
                            .predictions
                            .as_ref()
                            .and_then(|predictions| {
                                predictions
                                    .iter()
                                    .find_map(|prediction| Some(&prediction.bytes_base64_encoded))
                            });
                    }

                    if let Some(base64_image) = image_data {
                        Ok(base64_image.clone())
                    } else {
                        Err(Error::internal_err(
                            "No image data received from Gemini".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "Gemini API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        AIProvider::OpenRouter => {
            let mut messages = Vec::new();

            // Add system message if provided
            if let Some(system_prompt) = system_prompt {
                messages.push(OpenRouterImageMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                });
            }

            // Add user message
            messages.push(OpenRouterImageMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            });

            let openrouter_request = OpenRouterImageRequest {
                model: provider.get_model(),
                messages,
                modalities: vec!["image", "text"],
            };

            let resp = HTTP_CLIENT
                .post(format!("{}/chat/completions", base_url))
                .timeout(std::time::Duration::from_secs(120))
                .bearer_auth(api_key)
                .json(&openrouter_request)
                .send()
                .await
                .map_err(|e| {
                    Error::internal_err(format!("Failed to call OpenRouter API: {}", e))
                })?;

            match resp.error_for_status_ref() {
                Ok(_) => {
                    let openrouter_response =
                        resp.json::<OpenRouterImageResponse>().await.map_err(|e| {
                            Error::internal_err(format!(
                                "Failed to parse OpenRouter response: {}",
                                e
                            ))
                        })?;

                    // Extract base64 image from the first choice
                    let image_url = openrouter_response
                        .choices
                        .get(0)
                        .and_then(|choice| choice.message.images.as_ref())
                        .and_then(|images| images.get(0))
                        .map(|image| &image.image_url.url);

                    if let Some(data_url) = image_url {
                        // Extract base64 data from data URL format: data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...
                        if let Some(base64_start) = data_url.find("base64,") {
                            let base64_data = &data_url[base64_start + 7..]; // Skip "base64," prefix
                            Ok(base64_data.to_string())
                        } else {
                            Err(Error::internal_err(
                                "Invalid data URL format received from OpenRouter".to_string(),
                            ))
                        }
                    } else {
                        Err(Error::internal_err(
                            "No image data received from OpenRouter".to_string(),
                        ))
                    }
                }
                Err(e) => {
                    let _status = resp.status();
                    let text = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "<failed to read body>".to_string());
                    Err(Error::internal_err(format!(
                        "OpenRouter API error: {} - {}",
                        e, text
                    )))
                }
            }
        }
        _ => Err(Error::BadRequest(format!(
            "Image generation is not supported for provider: {:?}",
            provider.kind
        ))),
    }
}

/// Upload image to S3 and return S3Object
async fn upload_image_to_s3(
    base64_image: &str,
    job: &MiniPulledJob,
    client: &AuthedClient,
) -> error::Result<S3Object> {
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_image)
        .map_err(|e| Error::internal_err(format!("Failed to decode base64 image: {}", e)))?;

    // Generate unique S3 key
    let unique_id = ulid::Ulid::new().to_string();
    let s3_key = format!("ai_images/{}/{}.png", job.id, unique_id);

    // Create byte stream
    let byte_stream = futures::stream::once(async move {
        Ok::<_, std::convert::Infallible>(bytes::Bytes::from(image_bytes))
    });

    // Upload to S3
    client
        .upload_s3_file(
            &job.workspace_id,
            s3_key.clone(),
            None, // storage - use default
            byte_stream,
        )
        .await
        .map_err(|e| Error::internal_err(format!("Failed to upload image to S3: {}", e)))?;

    Ok(S3Object {
        s3: s3_key,
        storage: None,
        filename: Some("generated_image.png".to_string()),
        presigned: None,
    })
}

/// Handle image output generation and return S3 object and messages
async fn handle_image_output(
    args: &AIAgentArgs,
    job: &MiniPulledJob,
    client: &AuthedClient,
    db: &DB,
) -> error::Result<(Option<S3Object>, Vec<OpenAIMessage>)> {
    let base_url = args.provider.get_base_url(db).await?;
    let api_key = args.provider.get_api_key();

    let mut messages =
        if let Some(system_prompt) = args.system_prompt.clone().filter(|s| !s.is_empty()) {
            vec![OpenAIMessage {
                role: "system".to_string(),
                content: Some(OpenAIContent::Text(system_prompt)),
                ..Default::default()
            }]
        } else {
            vec![]
        };

    // Generate image from provider
    let base64_image = generate_image_from_provider(
        &args.provider,
        &args.user_message,
        args.system_prompt.as_deref(),
        &base_url,
        api_key,
        args.image.as_ref(),
        client,
        &job.workspace_id,
    )
    .await?;

    // Add assistant success message
    messages.push(OpenAIMessage {
        role: "assistant".to_string(),
        content: Some(OpenAIContent::Text(
            "Image created successfully".to_string(),
        )),
        ..Default::default()
    });

    // Upload to S3
    let s3_object = upload_image_to_s3(&base64_image, job, client).await?;

    Ok((Some(s3_object), messages))
}

async fn update_flow_status_module_with_actions(
    db: &DB,
    parent_job: &uuid::Uuid,
    actions: &[AgentAction],
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $3::TEXT, 'agent_actions'],
                        $2
                    )
                WHERE id = $1
                "#,
                parent_job,
                sqlx::types::Json(actions) as _,
                step as i32
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn update_flow_status_module_with_actions_success(
    db: &DB,
    parent_job: &uuid::Uuid,
    action_success: bool,
) -> Result<(), Error> {
    let step = get_step_of_flow_status(db, parent_job.to_owned()).await?;
    match step {
        Step::Step(step) => {
            // Append the new bool to the existing array, or create a new array if it doesn't exist
            sqlx::query!(
                r#"
                UPDATE v2_job_status SET
                    flow_status = jsonb_set(
                        flow_status,
                        array['modules', $2::TEXT, 'agent_actions_success'],
                        COALESCE(
                            flow_status->'modules'->$2->'agent_actions_success',
                            to_jsonb(ARRAY[]::bool[])
                        ) || to_jsonb(ARRAY[$3::bool])
                    )
                WHERE id = $1
                "#,
                parent_job,
                step as i32,
                action_success
            )
            .execute(db)
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn parse_raw_script_schema(content: &str, language: &ScriptLang) -> Result<Box<RawValue>, Error> {
    let main_arg_signature = parse_sig_of_lang(content, Some(&language), None)?.unwrap(); // safe to unwrap as langauge is some

    let schema = OpenAPISchema {
        r#type: Some(SchemaType::default()),
        properties: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| {
                    let name = arg.name.clone();
                    let typ = OpenAPISchema::from_typ(&arg.typ);
                    (name, Box::new(typ))
                })
                .collect(),
        ),
        required: Some(
            main_arg_signature
                .args
                .iter()
                .map(|arg| arg.name.clone())
                .collect(),
        ),
        ..Default::default()
    };

    Ok(to_raw_value(&schema))
}

// Use the new unified agent runner from the ai module
use crate::ai::agent_runner::run_agent_unified;

#[async_recursion]
async fn run_agent(
    // connection
    db: &DB,
    conn: &Connection,

    // agent job and flow data
    job: &MiniPulledJob,
    parent_job: &uuid::Uuid,
    args: &AIAgentArgs,
    tools: &[Tool],

    // job execution context
    client: &AuthedClient,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> error::Result<Box<RawValue>> {
    // Delegate to the new unified agent runner
    run_agent_unified(
        db,
        conn,
        job,
        parent_job,
        args,
        tools,
        client,
        occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
    )
    .await
}

pub struct FlowJobRunnableIdAndRawFlow {
    pub runnable_id: Option<ScriptHash>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub kind: JobKind,
}

pub async fn get_flow_job_runnable_and_raw_flow(
    db: &DB,
    job_id: &uuid::Uuid,
) -> windmill_common::error::Result<FlowJobRunnableIdAndRawFlow> {
    let job = sqlx::query_as!(
        FlowJobRunnableIdAndRawFlow,
        "SELECT runnable_id as \"runnable_id: ScriptHash\", raw_flow as \"raw_flow: _\", kind as \"kind: _\" FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_one(db)
    .await?;
    Ok(job)
}

pub async fn handle_ai_agent_job(
    // connection
    conn: &Connection,
    db: &DB,

    // agent job
    job: &MiniPulledJob,

    // job execution context
    client: &AuthedClient,
    canceled_by: &mut Option<CanceledBy>,
    mem_peak: &mut i32,
    occupancy_metrics: &mut OccupancyMetrics,
    job_completed_tx: &JobCompletedSender,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    hostname: &str,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> Result<Box<RawValue>, Error> {
    let args = build_args_map(job, client, conn).await?;

    let args = serde_json::from_str::<AIAgentArgs>(&serde_json::to_string(&args)?)?;

    let Some(flow_step_id) = &job.flow_step_id else {
        return Err(Error::internal_err(
            "AI agent job has no flow step id".to_string(),
        ));
    };

    let Some(parent_job) = &job.parent_job else {
        return Err(Error::internal_err(
            "AI agent job has no parent job".to_string(),
        ));
    };

    let flow_job = get_flow_job_runnable_and_raw_flow(db, &parent_job).await?;

    let flow_data = match flow_job.kind {
        JobKind::Flow | JobKind::FlowNode => {
            cache::job::fetch_flow(db, &flow_job.kind, flow_job.runnable_id).await?
        }
        JobKind::FlowPreview => {
            cache::job::fetch_preview_flow(db, &parent_job, flow_job.raw_flow).await?
        }
        _ => {
            return Err(Error::internal_err(
                "expected parent flow, flow preview or flow node for ai agent job".to_string(),
            ));
        }
    };

    let value = flow_data.value();

    let module = value.modules.iter().find(|m| m.id == *flow_step_id);

    let Some(module) = module else {
        return Err(Error::internal_err(
            "AI agent module not found in flow".to_string(),
        ));
    };

    let FlowModuleValue::AIAgent { tools, .. } = module.get_value()? else {
        return Err(Error::internal_err(
            "AI agent module is not an AI agent".to_string(),
        ));
    };

    let tools = futures::future::try_join_all(tools.into_iter().map(|mut t| {
        let conn = conn;
        let db = db;
        let job = job;
        async move {
            let Some(summary) = t.summary.as_ref().filter(|s| TOOL_NAME_REGEX.is_match(s)) else {
                return Err(Error::internal_err(format!(
                    "Invalid tool name: {:?}",
                    t.summary
                )));
            };

            let schema = match &t.get_value() {
                Ok(FlowModuleValue::Script {
                    hash,
                    path,
                    tag_override,
                    input_transforms,
                    is_trigger,
                }) => match hash {
                    Some(hash) => {
                        let (_, metadata) = cache::script::fetch(conn, hash.clone()).await?;
                        Ok::<_, Error>(
                            metadata
                                .schema
                                .clone()
                                .map(|s| RawValue::from_string(s).ok())
                                .flatten(),
                        )
                    }
                    None => {
                        if path.starts_with("hub/") {
                            let hub_script = get_full_hub_script_by_path(
                                StripPath(path.to_string()),
                                &HTTP_CLIENT,
                                None,
                            )
                            .await?;
                            Ok(Some(hub_script.schema))
                        } else {
                            let hash = get_latest_hash_for_path(db, &job.workspace_id, path, true)
                                .await?
                                .0;
                            // update module definition to use a fixed hash so all tool calls match the same schema
                            t.value = to_raw_value(&FlowModuleValue::Script {
                                hash: Some(hash),
                                path: path.clone(),
                                tag_override: tag_override.clone(),
                                input_transforms: input_transforms.clone(),
                                is_trigger: *is_trigger,
                            });
                            let (_, metadata) = cache::script::fetch(conn, hash).await?;
                            Ok(metadata
                                .schema
                                .clone()
                                .map(|s| RawValue::from_string(s).ok())
                                .flatten())
                        }
                    }
                },
                Ok(FlowModuleValue::RawScript { content, language, .. }) => {
                    Ok(Some(parse_raw_script_schema(&content, &language)?))
                }
                Err(e) => {
                    return Err(Error::internal_err(format!(
                        "Invalid tool {}: {}",
                        summary,
                        e.to_string()
                    )));
                }
                _ => {
                    return Err(Error::internal_err(format!(
                        "Unsupported tool: {}",
                        summary
                    )));
                }
            }?;

            Ok(Tool {
                def: ToolDef {
                    r#type: "function".to_string(),
                    function: ToolDefFunction {
                        name: summary.clone(),
                        description: None,
                        parameters: schema.unwrap_or_else(|| {
                            to_raw_value(&serde_json::json!({
                                "type": "object",
                                "properties": {},
                                "required": [],
                            }))
                        }),
                    },
                },
                module: t,
            })
        }
    }))
    .await?;

    let mut inner_occupancy_metrics = occupancy_metrics.clone();

    let agent_fut = run_agent(
        db,
        conn,
        job,
        parent_job,
        &args,
        &tools,
        client,
        &mut inner_occupancy_metrics,
        job_completed_tx,
        worker_dir,
        base_internal_url,
        worker_name,
        hostname,
        killpill_rx,
    );

    let result = run_future_with_polling_update_job_poller(
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        agent_fut,
        worker_name,
        &job.workspace_id,
        &mut Some(occupancy_metrics),
        Box::pin(futures::stream::once(async { 0 })),
    )
    .await?;

    Ok(result)
}
