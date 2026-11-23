//! `request_secret`: the user types secrets into a Windmill card, which saves them; the model
//! only ever gets the path. The chat draws the card inline. An MCP client cannot draw it, so it
//! gets a link to the same card on a Windmill page instead.

use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use windmill_common::error::{Error, JsonResult};
use windmill_common::jwt::{decode_with_internal_secret, encode_with_internal_secret};

use super::chat::Effect;
use super::workspace::path_enc;
use super::{bool_arg, enc, opt_str_arg, str_arg, Ctx, Surface, ToolResult};
use crate::db::ApiAuthed;

pub(super) const TOOLS: &[&str] = &["request_secret"];

/// The resource types whose card can take a connection string instead of the fields, and the
/// secret field the string's password is saved as.
const CONNECTION_STRING_TYPES: &[(&str, &str, &str)] = &[("postgresql", "password", "Password")];

/// How long a link to the card can be opened.
const LINK_TTL_SECS: u64 = 60 * 60;

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "request_secret" => request_secret(ctx, args).await,
        _ => unreachable!("{name} is not a secrets tool"),
    }
}

async fn request_secret(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let request = secret_request(ctx.w_id, args)?;
    let path = request["path"].as_str().unwrap_or_default().to_string();
    if !bool_arg(args, "replace") {
        if let Some(taken) = taken_path(ctx, &request).await? {
            return Err(format!(
                "{taken}. Ask again with a free path, or with replace: true only if the user asked to replace it."
            ));
        }
    }
    if ctx.surface == Surface::Chat {
        ctx.effect(Effect::SecretRequest { request });
        return Ok(String::new());
    }
    let claims = LinkClaims {
        sub: ctx.auth.email.clone(),
        w: ctx.w_id.to_string(),
        req: request.clone(),
        exp: now_secs() + LINK_TTL_SECS,
    };
    let token = encode_with_internal_secret(claims)
        .await
        .map_err(|e| format!("Could not sign the link: {e}"))?;
    let link = ctx.page_url(&format!("/secure_input?token={}", enc(&token)));
    let (kind, reference) = if request.get("resource").is_some() {
        ("resource", format!("$res:{path}"))
    } else {
        ("secret variable", format!("$var:{path}"))
    };
    Ok(format!(
        "Give the user this link, as a link, and tell them to open it, enter the secret there and come back: {link}\n\
         It opens a Windmill page, signed in as them, where the secret is saved as the {kind} {path} without reaching you. \
         It works for one hour and only for this user. Never ask for the secret in this conversation. \
         Once they say it is saved, check that the {kind} exists (they can change the path on the page, so ask which one if it is not at {path}), \
         then reference it as \"{reference}\"."
    ))
}

/// The card's request from the tool's arguments: the combinations it refuses, and a secret the
/// model also put in the prepared value dropped, since the card would show it in clear.
fn secret_request(w_id: &str, args: &Value) -> Result<Value, String> {
    let path = str_arg(args, "path")?;
    let label = str_arg(args, "label")?;
    let resource_type = opt_str_arg(args, "resource_type");
    let connection_string = bool_arg(args, "connection_string");
    let string_field = connection_string
        .then(|| {
            CONNECTION_STRING_TYPES
                .iter()
                .find(|(t, _, _)| Some(*t) == resource_type)
        })
        .flatten();
    if connection_string && string_field.is_none() {
        let types: Vec<_> = CONNECTION_STRING_TYPES.iter().map(|(t, _, _)| *t).collect();
        return Err(format!(
            "connection_string works with resource_type {} only.",
            types.join(", ")
        ));
    }
    let secret_fields: Vec<Value> = match string_field {
        Some((_, field, label)) => vec![json!({ "field": field, "label": label })],
        None => args
            .get("secret_fields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    };
    let resource_value = args.get("resource_value").filter(|v| !v.is_null());
    let mut request = Map::new();
    request.insert("workspace".into(), json!(w_id));
    request.insert("path".into(), json!(path));
    request.insert("label".into(), json!(label));
    for (arg, key) in [("description", "description"), ("help_url", "helpUrl")] {
        if let Some(value) = opt_str_arg(args, arg) {
            request.insert(key.into(), json!(value));
        }
    }
    // Models sometimes escape the newlines of a JSON string twice.
    if let Some(text) = opt_str_arg(args, "help_text") {
        request.insert("helpText".into(), json!(text.replace("\\n", "\n")));
    }
    if bool_arg(args, "replace") {
        request.insert("replace".into(), json!(true));
    }
    match resource_type {
        Some(resource_type) => {
            if secret_fields.is_empty() {
                return Err("secret_fields is required with resource_type.".to_string());
            }
            let secret_keys: Vec<&str> = secret_fields
                .iter()
                .filter_map(|f| f.get("field").and_then(Value::as_str))
                .collect();
            let value: Map<String, Value> = resource_value
                .and_then(Value::as_object)
                .map(|value| {
                    value
                        .iter()
                        .filter(|(k, _)| !secret_keys.contains(&k.as_str()))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let mut resource = json!({
                "type": resource_type,
                "value": value,
                "secretFields": secret_fields,
            });
            if string_field.is_some() {
                resource["connectionString"] = json!(true);
            }
            request.insert("resource".into(), resource);
        }
        None if !secret_fields.is_empty() || resource_value.is_some() => {
            return Err("secret_fields and resource_value need resource_type.".to_string());
        }
        None => {}
    }
    Ok(Value::Object(request))
}

/// Where each secret is saved: the app-connect form's convention, a lone secret at the
/// resource's path and several at `<path>_<field>`. Mirrors `secretVariablePaths`.
fn variable_paths(request: &Value) -> Vec<String> {
    let path = request["path"].as_str().unwrap_or_default();
    let fields = request
        .pointer("/resource/secretFields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if fields.len() <= 1 {
        return vec![path.to_string()];
    }
    fields
        .iter()
        .filter_map(|f| f.get("field").and_then(Value::as_str))
        .map(|field| format!("{path}_{field}"))
        .collect()
}

/// Refused before the card shows, so a path the AI did not check costs it a retry rather than
/// costing the user a choice it should have made.
async fn taken_path(ctx: &Ctx<'_>, request: &Value) -> Result<Option<String>, String> {
    let path = request["path"].as_str().unwrap_or_default();
    if request.get("resource").is_some()
        && exists(ctx, &format!("resources/exists/{}", path_enc(path))).await?
    {
        return Ok(Some(format!("A resource already exists at {path}")));
    }
    for p in variable_paths(request) {
        if exists(ctx, &format!("variables/exists/{}", path_enc(&p))).await? {
            return Ok(Some(format!("A variable already exists at {p}")));
        }
    }
    Ok(None)
}

async fn exists(ctx: &Ctx<'_>, route: &str) -> Result<bool, String> {
    Ok(ctx.get(&ctx.ws(route)).await?.as_bool().unwrap_or(false))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

/// What a link to the card carries. Signed, so a link nobody's tool call made, which could
/// pre-fill a host its author controls, does not open; bound to the user who asked.
#[derive(Serialize, Deserialize)]
struct LinkClaims {
    sub: String,
    w: String,
    req: Value,
    exp: u64,
}

#[derive(Deserialize)]
pub(super) struct OpenLink {
    token: String,
}

/// The card's request a link carries, for the page that draws it.
pub(super) async fn open_secret_request(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Json(OpenLink { token }): Json<OpenLink>,
) -> JsonResult<Value> {
    let claims: LinkClaims = decode_with_internal_secret(&token)
        .await
        .map_err(|_| Error::BadRequest("This link is invalid or has expired.".to_string()))?;
    if claims.w != w_id || claims.sub != authed.email {
        return Err(Error::PermissionDenied(
            "This link was made for another user or workspace.".to_string(),
        ));
    }
    Ok(Json(claims.req))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_request_keeps_secrets_off_the_card() {
        let request = secret_request(
            "ws",
            &json!({
                "path": "u/a/db", "label": "DB", "resource_type": "postgresql",
                "connection_string": true,
                "resource_value": { "host": "h", "password": "typed-by-the-model" },
            }),
        )
        .unwrap();
        assert_eq!(request["resource"]["value"], json!({ "host": "h" }));
        assert_eq!(request["resource"]["secretFields"][0]["field"], "password");

        let refused = |args: Value| secret_request("ws", &args).unwrap_err();
        assert!(
            refused(json!({ "path": "p", "label": "l", "resource_type": "s3" }))
                .contains("secret_fields is required")
        );
        assert!(refused(json!({ "path": "p", "label": "l", "resource_type": "mysql", "connection_string": true }))
            .contains("connection_string works with"));
        assert!(
            refused(json!({ "path": "p", "label": "l", "secret_fields": [{ "field": "k" }] }))
                .contains("need resource_type")
        );
    }
}
