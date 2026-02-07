/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::types::Json as SqlxJson;
use windmill_common::{
    db::DB, error::Result, triggers::TriggerKind, utils::not_found_if_none, worker::CLOUD_HOSTED,
};
use windmill_queue::{PushArgs, PushArgsOwned};

const KEEP_LAST: i64 = 20;

struct ActiveCaptureOwner {
    owner: String,
    email: String,
}

pub async fn get_active_capture_owner_and_email(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    kind: &TriggerKind,
) -> Result<(String, String)> {
    let capture_config = sqlx::query_as!(
        ActiveCaptureOwner,
        r#"
        SELECT
            owner,
            email
        FROM
            capture_config
        WHERE
            workspace_id = $1
            AND path = $2
            AND is_flow = $3
            AND trigger_kind = $4
            AND last_client_ping > NOW() - INTERVAL '10 seconds'
        "#,
        &w_id,
        &path,
        is_flow,
        kind as &TriggerKind,
    )
    .fetch_optional(db)
    .await?;

    let capture_config = not_found_if_none(
        capture_config,
        &format!("capture config for {} trigger", kind),
        path,
    )?;

    Ok((capture_config.owner, capture_config.email))
}

async fn clear_captures_history(db: &DB, w_id: &str) -> Result<()> {
    if *CLOUD_HOSTED {
        /* Retain only KEEP_LAST most recent captures in this workspace. */
        sqlx::query!(
            r#"
        DELETE FROM
            capture
        WHERE
            workspace_id = $1
            AND created_at <= (
                SELECT
                    created_at
                FROM
                    capture
                WHERE
                    workspace_id = $1
                ORDER BY
                    created_at DESC
                OFFSET $2
                LIMIT 1
            )
        "#,
            &w_id,
            KEEP_LAST,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

pub async fn insert_capture_payload(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    trigger_kind: &TriggerKind,
    main_args: PushArgsOwned,
    preprocessor_args: PushArgsOwned,
    owner: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
    INSERT INTO
        capture (
            workspace_id, path, is_flow, trigger_kind, main_args, preprocessor_args, created_by
        )
    VALUES (
        $1, $2, $3, $4, $5, $6, $7
    )
    "#,
        &w_id,
        path,
        is_flow,
        trigger_kind as &TriggerKind,
        SqlxJson(PushArgs { args: &main_args.args, extra: main_args.extra }) as SqlxJson<PushArgs>,
        SqlxJson(PushArgs { args: &preprocessor_args.args, extra: preprocessor_args.extra })
            as SqlxJson<PushArgs>,
        owner,
    )
    .execute(db)
    .await?;

    clear_captures_history(db, &w_id).await?;

    Ok(())
}
