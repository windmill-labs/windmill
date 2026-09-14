//! `PROPER_EMAIL` mirrors the `proper_email` CHECK constraint so callers can tell, before
//! writing, whether `usr` will hold an address. The two only stay equivalent while nobody
//! edits one without the other: each sample below must be stored by `usr` exactly when the
//! mirror accepts it, and everything `VALID_EMAIL` accepts must be stored too.

use sqlx::{Pool, Postgres};
use windmill_common::users::{PROPER_EMAIL, VALID_EMAIL};

#[sqlx::test(migrations = "../migrations")]
async fn proper_email_mirror_agrees_with_usr_constraint(db: Pool<Postgres>) -> anyhow::Result<()> {
    for email in [
        "alice@example.com",
        "Alice@Example.COM",
        "alice.bob+tag@sub.example.co.uk",
        "\"quoted\"@example.com",
        "\"quoted local\"@example.com",
        "alice@[192.168.0.1]",
        "ef40ea04-1a9e-4a84-9e65-cb1baa81dfed",
        // Unicode case folding would map the long s and the Kelvin sign into `[a-z]`.
        "u\u{17f}er@example.com",
        "alice@example\u{212a}.com",
        "alice",
        "alice@example",
        "alice@@example.com",
        "alice @example.com",
        "alice@example.com\nbob@example.com",
        "",
    ] {
        let mut tx = db.begin().await?;
        let stored = sqlx::query(
            "INSERT INTO usr (workspace_id, username, email, is_admin, operator)
             VALUES ('admins', 'probe', $1, false, false)",
        )
        .bind(email)
        .execute(&mut *tx)
        .await
        .is_ok();
        tx.rollback().await?;

        assert_eq!(
            stored,
            PROPER_EMAIL.is_match(email),
            "{email:?}: `usr` and PROPER_EMAIL disagree"
        );
        if VALID_EMAIL.is_match(email) {
            assert!(stored, "{email:?}: VALID_EMAIL accepts what `usr` rejects");
        }
    }
    Ok(())
}
