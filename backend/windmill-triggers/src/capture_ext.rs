/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::{
    error::Result,
    triggers::TriggerKind,
    worker::CLOUD_HOSTED,
    DB,
};
use windmill_queue::{PushArgs, PushArgsOwned};

const KEEP_LAST: i64 = 20;

async fn clear_captures_history(db: &DB, w_id: &str) -> Result<()> {
    if *CLOUD_HOSTED {
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
        sqlx::types::Json(PushArgs { args: &main_args.args, extra: main_args.extra })
            as sqlx::types::Json<PushArgs>,
        sqlx::types::Json(PushArgs {
            args: &preprocessor_args.args,
            extra: preprocessor_args.extra
        }) as sqlx::types::Json<PushArgs>,
        owner,
    )
    .execute(db)
    .await?;

    clear_captures_history(db, &w_id).await?;

    Ok(())
}
