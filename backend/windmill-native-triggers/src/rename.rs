//! Keeping native triggers usable when their runnable is renamed.

use std::collections::HashMap;

use windmill_api_auth::ApiAuthed;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    error::{Error, Result},
    triggers::MovedNativeTrigger,
    DB,
};

use crate::{
    decrypt_oauth_data, delete_token_by_hash, get_native_trigger, github::GitHub, google::Google,
    handler::new_webhook_token, lock::TriggerLock, nextcloud::NextCloud, record_reregistration,
    update_native_trigger_error, External, NativeTriggerData, ServiceName,
};

/// Re-register the webhooks of the native triggers a rename moved onto a new runnable path.
///
/// The URL held by the external service embeds the runnable path and a token scoped to it, so
/// after a rename the registration points at a path that no longer resolves.
///
/// `moved` comes from `windmill_common::triggers::update_triggers_script_path`. Callers MUST have
/// already committed that rename and verified the caller's write access to the path the rows now
/// carry — this mints fresh `jobs:run:*` tokens for it. Run it *after* the commit: repointing a
/// webhook is not undoable, so doing it while the deploy transaction can still roll back would
/// strand the trigger on a path and token that never existed.
///
/// The replacement token belongs to `authed`, so the runnable now executes as whoever deployed the
/// rename rather than as the trigger's previous owner — the same identity swap a manual trigger
/// edit performs, since a path-scoped token cannot outlive the path it names.
///
/// A service that rejects the update gets the failure recorded on its trigger row rather than
/// propagated: the runnable is renamed either way, and the user can retry by re-saving the trigger.
pub async fn reregister_triggers_after_rename(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    moved: &[MovedNativeTrigger],
) {
    let mut by_service: HashMap<ServiceName, Vec<&MovedNativeTrigger>> = HashMap::new();
    for trigger in moved {
        match ServiceName::try_from(trigger.service_name.clone()) {
            Ok(service) => by_service.entry(service).or_default().push(trigger),
            Err(e) => tracing::error!(
                "Unknown native trigger service '{}' on trigger '{}' in workspace '{w_id}': {e:#}",
                trigger.service_name,
                trigger.external_id
            ),
        }
    }

    for (service, of_service) in by_service {
        match service {
            ServiceName::Nextcloud => {
                reregister_service(db, authed, w_id, &of_service, NextCloud).await
            }
            ServiceName::Google => reregister_service(db, authed, w_id, &of_service, Google).await,
            ServiceName::Github => reregister_service(db, authed, w_id, &of_service, GitHub).await,
        }
    }
}

async fn reregister_service<T: External>(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    moved: &[&MovedNativeTrigger],
    handler: T,
) {
    let oauth_data: T::OAuthData =
        match decrypt_oauth_data(db, w_id, T::SERVICE_NAME.integration_service()).await {
            Ok(oauth_data) => oauth_data,
            Err(e) => {
                for trigger in moved {
                    record_failure::<T>(db, w_id, &trigger.external_id, &e).await;
                }
                return;
            }
        };

    for trigger in moved {
        if let Err(e) = reregister_one(db, authed, w_id, &handler, &oauth_data, trigger).await {
            record_failure::<T>(db, w_id, &trigger.external_id, &e).await;
        }
    }
}

/// Point one trigger's webhook at the runnable the rename moved it to.
///
/// `TriggerLock` excludes the other operations that touch a registration — edits, deletes, channel
/// renewal, another re-registration — for the whole span including the network call. Renames are
/// not among them: they move the row from inside the runnable's own transaction without taking
/// this lock, so a further rename can still land mid-call. That is why the write-back is
/// additionally conditional on `updated_at` rather than trusting the row read here.
async fn reregister_one<T: External>(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    handler: &T,
    oauth_data: &T::OAuthData,
    moved: &MovedNativeTrigger,
) -> Result<()> {
    let external_id = moved.external_id.as_str();
    let mut lock = TriggerLock::acquire(db, w_id, T::SERVICE_NAME, external_id).await?;

    let trigger = get_native_trigger(db, w_id, T::SERVICE_NAME, external_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Native trigger not found: {external_id}")))?;

    // Something took the lock first and pointed the trigger elsewhere. That operation authorized
    // its own destination and installed its own registration; re-registering here would overwrite
    // both with a token minted for a runnable this rename never authorized.
    if trigger.script_path != moved.script_path || trigger.is_flow != moved.is_flow {
        tracing::info!(
            "Skipping re-registration of the {} trigger '{external_id}': it was moved to '{}' \
             after the rename that queued this",
            T::SERVICE_NAME,
            trigger.script_path
        );
        return lock.release().await;
    }

    let service_config: T::ServiceConfig = serde_json::from_value(
        trigger
            .service_config
            .clone()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| Error::internal_err(format!("stored trigger config cannot be read back: {e}")))?;

    let data = NativeTriggerData {
        script_path: trigger.script_path.clone(),
        is_flow: trigger.is_flow,
        service_config,
        summary: trigger.summary.clone(),
    };

    // The token is scoped to the runnable path and only its hash is kept, so pointing the webhook
    // at the new path means minting a replacement rather than reusing the old one. Commit it
    // before handing it out: a service may call back the moment it accepts the new URL.
    let mut tx = db.begin().await?;
    let webhook_token = new_webhook_token(
        &mut tx,
        db,
        authed,
        &trigger.script_path,
        trigger.is_flow,
        w_id,
        T::SERVICE_NAME,
    )
    .await?;
    tx.commit().await?;

    let updated = handler
        .update(
            w_id,
            oauth_data,
            external_id,
            &webhook_token,
            &data,
            db,
            lock.conn(),
        )
        .await;

    // A failed `update` does not mean the service never installed the token — Nextcloud mutates and
    // then reads back, and any service can fail on the response after committing the mutation. So
    // the token stays valid; deleting one the service did install would turn a webhook that still
    // works into one that 401s.
    let service_config = updated.inspect_err(|_| {
        tracing::warn!(
            "The webhook token minted for the {} trigger '{external_id}' was kept even though no \
             row references it, since the service may have installed it",
            T::SERVICE_NAME
        )
    })?;

    let mut tx = db.begin().await?;

    let applied = record_reregistration(
        &mut *tx,
        w_id,
        T::SERVICE_NAME,
        external_id,
        &webhook_token,
        service_config,
        trigger.updated_at,
    )
    .await?;

    if !applied {
        // Written again while this was on the network — another rename, a save, or a disconnect
        // that removed the row. Leave whatever is there now, including a newer rename's pending
        // marker, for its own writer to finish, and keep this token: the service may be holding
        // it. If the row is gone outright, the disconnect path revokes every token it deletes.
        tx.rollback().await?;
        tracing::info!(
            "Discarding the re-registration of the {} trigger '{external_id}': the row changed \
             while the service was being updated",
            T::SERVICE_NAME
        );
        return lock.release().await;
    }

    delete_token_by_hash(&mut *tx, &trigger.webhook_token_hash).await?;

    audit_log(
        &mut *tx,
        authed,
        &format!("native_triggers.{}.update", T::SERVICE_NAME),
        ActionKind::Update,
        w_id,
        Some(external_id),
        Some([("reason", "runnable renamed")].into()),
    )
    .await?;

    tx.commit().await?;
    lock.release().await?;

    Ok(())
}

async fn record_failure<T: External>(db: &DB, w_id: &str, external_id: &str, err: &Error) {
    tracing::error!(
        "Failed to re-register the {} trigger '{external_id}' in workspace '{w_id}' after rename: {err:#}",
        T::SERVICE_NAME,
    );
    let message = format!(
        "Could not re-point the webhook registered on {} at the renamed runnable, so it is no \
         longer known to be delivering: {err}. Save the trigger again to retry.",
        T::DISPLAY_NAME
    );
    if let Err(e) =
        update_native_trigger_error(db, w_id, T::SERVICE_NAME, external_id, Some(&message)).await
    {
        tracing::error!(
            "Failed to record the re-registration failure of the {} trigger '{external_id}': {e:#}",
            T::SERVICE_NAME,
        );
    }
}
