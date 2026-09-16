//! `usr_accepts_email` predicts whether `usr` will store an address by evaluating
//! `PROPER_EMAIL_PATTERN` in the database. It only stays right while that text matches the
//! `proper_email` constraint and the width matches the column: each sample below must be
//! stored by `usr` exactly when the check accepts it, and everything `VALID_EMAIL` accepts
//! within the width must be stored too.

use sqlx::{Pool, Postgres};
use windmill_common::users::{usr_accepts_email, EMAIL_COLUMN_MAX_LEN, VALID_EMAIL};

#[sqlx::test(migrations = "../migrations")]
async fn usr_accepts_email_agrees_with_the_constraint(db: Pool<Postgres>) -> anyhow::Result<()> {
    let domain = "@example.com";
    let widest = format!(
        "{}{domain}",
        "a".repeat(EMAIL_COLUMN_MAX_LEN - domain.len())
    );
    let too_wide = format!("a{widest}");
    for email in [
        "alice@example.com",
        "Alice@Example.COM",
        "alice.bob+tag@sub.example.co.uk",
        "\"quoted\"@example.com",
        "\"quoted local\"@example.com",
        "alice@[192.168.0.1]",
        widest.as_str(),
        too_wide.as_str(),
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
            usr_accepts_email(&db, email).await?,
            "{email:?}: `usr` and usr_accepts_email disagree"
        );
        if VALID_EMAIL.is_match(email) && email.len() <= EMAIL_COLUMN_MAX_LEN {
            assert!(stored, "{email:?}: VALID_EMAIL accepts what `usr` rejects");
        }
    }
    Ok(())
}
