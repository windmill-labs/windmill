/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! A data table migration's scope: the schemas it declares it touches, written as a
//! `-- schemas sales, billing` annotation in its up SQL. A migration that declares none applies to
//! the whole database.
//!
//! Scope decides ordering, not access: a migration waits on every earlier migration whose scope
//! overlaps its own, and its rollback on every later overlapping one. A whole-database migration
//! overlaps everything, so it is a barrier; migrations on disjoint schemas do not wait on each
//! other. Nothing checks that the SQL stays inside its declared scope.

use std::collections::HashSet;

use windmill_common::error::{Error, Result};

/// The schemas a migration declares, lowercased, or `None` for the whole database.
pub(crate) type Scope = Option<Vec<String>>;

const KEYWORD: &str = "schemas";

/// Read the `-- schemas a, b` annotation from the leading comment block of `code`, the way
/// `SqlAnnotations::datatable_role` reads `-- role`. A leading comment whose first word is
/// `schemas` is an annotation attempt, and a malformed one is an error rather than a silent
/// whole-database scope.
///
/// Names are lowercased: SQL folds unquoted identifiers, and two migrations spelling one schema
/// `Sales` and `sales` must still be ordered against each other. A false overlap only costs
/// waiting; a false disjointness lets a migration run before one it depends on.
pub(crate) fn parse_migration_schemas(code: &str) -> Result<Scope> {
    for line in code.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("--") {
            break;
        }
        let body = line[2..].trim_start();
        let Some(after) = body
            .get(..KEYWORD.len())
            .filter(|kw| kw.eq_ignore_ascii_case(KEYWORD))
            .map(|_| &body[KEYWORD.len()..])
        else {
            continue;
        };
        if !after.is_empty()
            && !after.starts_with(char::is_whitespace)
            && !after.starts_with([':', '='])
        {
            continue;
        }
        let after = after.trim_start();
        let after = after.strip_prefix([':', '=']).unwrap_or(after).trim();
        let after = after.strip_suffix(';').unwrap_or(after);
        let mut schemas: Vec<String> = vec![];
        for piece in after.split(',') {
            let name = piece.trim();
            if !is_annotatable_schema_name(name) {
                return Err(Error::BadRequest(format!(
                    "Malformed data table migration scope annotation: `{line}`. Write it as \
                     `-- schemas <name>, <name>` on a line of its own, where each name is \
                     letters, digits, '_', '$' or '-', or leave it out for a migration that \
                     applies to the whole database. A comment in the leading block that starts \
                     with the word 'schemas' is read as this annotation; move it below the first \
                     statement if it is prose."
                )));
            }
            let name = name.to_ascii_lowercase();
            if !schemas.contains(&name) {
                schemas.push(name);
            }
        }
        return Ok(Some(schemas));
    }
    Ok(None)
}

/// Whether `name` can be written in a `-- schemas` annotation.
fn is_annotatable_schema_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-'))
}

pub(crate) fn scopes_overlap(a: &Scope, b: &Scope) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.iter().any(|s| b.contains(s)),
        _ => true,
    }
}

/// One migration of a data table's chain, as ordering sees it.
pub(crate) struct ChainEntry {
    pub(crate) timestamp: i64,
    pub(crate) name: String,
    /// A malformed annotation counts as the whole database, so it orders conservatively. It is
    /// refused only when a migration is written: one stored before scopes existed may open with
    /// prose that starts with the word, and must keep running as it always did.
    pub(crate) scope: Scope,
}

impl ChainEntry {
    pub(crate) fn new(timestamp: i64, name: String, code_up: &str) -> Self {
        let scope = parse_migration_schemas(code_up).unwrap_or(None);
        Self { timestamp, name, scope }
    }

    fn label(&self) -> String {
        format!("{} ({})", self.timestamp, self.name)
    }
}

/// Why `later` has to wait for `earlier`, for an error message.
fn overlap_reason(later: &ChainEntry, earlier: &ChainEntry) -> String {
    match (&later.scope, &earlier.scope) {
        (_, None) => format!("{} applies to the whole database", earlier.label()),
        (None, _) => format!("{} applies to the whole database", later.label()),
        (Some(a), Some(b)) => {
            let shared: Vec<&str> = a
                .iter()
                .filter(|s| b.contains(s))
                .map(String::as_str)
                .collect();
            format!("both touch schema {}", shared.join(", "))
        }
    }
}

/// Which pending migrations one run applies.
pub(crate) enum RunSelection {
    /// Every pending migration, or those up to and including a version.
    UpTo(Option<i64>),
    /// One version.
    Only(i64),
    /// The pending migrations that declare this schema. A whole-database one is never part of
    /// it: running for one schema must not change the others.
    Schema(String),
}

/// The chain indices one run applies, in order, or why it cannot start. Every selected migration
/// needs each earlier overlapping migration applied or applied by this same run, so the whole run
/// is refused before anything is applied rather than stopping half-way.
///
/// `Only` is exempt: it is the explicit out-of-order run, which the editors confirm with the user
/// (through `blocked_by`) before asking for it.
pub(crate) fn plan_run(
    chain: &[ChainEntry],
    applied: &HashSet<i64>,
    selection: &RunSelection,
) -> Result<Vec<usize>> {
    let pending = |i: usize| !applied.contains(&chain[i].timestamp);
    let selected: Vec<usize> = (0..chain.len())
        .filter(|&i| {
            pending(i)
                && match selection {
                    RunSelection::UpTo(None) => true,
                    RunSelection::UpTo(Some(up_to)) => chain[i].timestamp <= *up_to,
                    RunSelection::Only(version) => chain[i].timestamp == *version,
                    RunSelection::Schema(schema) => {
                        chain[i].scope.as_ref().is_some_and(|s| s.contains(schema))
                    }
                }
        })
        .collect();
    if matches!(selection, RunSelection::Only(_)) {
        return Ok(selected);
    }
    let in_run: HashSet<usize> = selected.iter().copied().collect();
    for &i in &selected {
        let blocker = (0..i).find(|&j| {
            pending(j) && !in_run.contains(&j) && scopes_overlap(&chain[i].scope, &chain[j].scope)
        });
        if let Some(j) = blocker {
            return Err(Error::BadRequest(format!(
                "Migration {} cannot run before {}, which is not applied yet: {}. Apply it first, \
                 or run all pending migrations.",
                chain[i].label(),
                chain[j].label(),
                overlap_reason(&chain[i], &chain[j]),
            )));
        }
    }
    Ok(selected)
}

/// The earliest pending migration `chain[i]` has to wait for, when it is pending; the latest
/// applied migration that has to be rolled back before it, when it is applied.
pub(crate) fn blocked_by(chain: &[ChainEntry], applied: &HashSet<i64>, i: usize) -> Option<usize> {
    let is_applied = |j: usize| applied.contains(&chain[j].timestamp);
    let overlaps = |j: usize| scopes_overlap(&chain[i].scope, &chain[j].scope);
    if is_applied(i) {
        (i + 1..chain.len())
            .rev()
            .find(|&j| is_applied(j) && overlaps(j))
    } else {
        (0..i).find(|&j| !is_applied(j) && overlaps(j))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(names: &[&str]) -> Scope {
        Some(names.iter().map(|n| n.to_string()).collect())
    }

    #[test]
    fn schemas_annotation_is_read_from_the_leading_comment_block() {
        let p = |code| parse_migration_schemas(code).unwrap();
        assert_eq!(p("-- schemas sales\nBEGIN;"), s(&["sales"]));
        assert_eq!(
            p("-- role analyst\n-- Schemas: Sales, billing,sales;\nBEGIN;"),
            s(&["sales", "billing"])
        );
        assert_eq!(p("-- schemas=a\nBEGIN;"), s(&["a"]));
        assert_eq!(p("BEGIN;\n-- schemas sales"), None);
        assert_eq!(p("-- schemasx sales\nBEGIN;"), None);
        assert_eq!(p("-- up migration: x\nBEGIN;"), None);
        for malformed in [
            "-- schemas\nBEGIN;",
            "-- schemas:\nBEGIN;",
            "-- schemas sales billing\nBEGIN;",
            "-- schemas: changes for the orders table\nBEGIN;",
            "-- schemas a,,b\nBEGIN;",
            "-- schemas a.b\nBEGIN;",
        ] {
            assert!(parse_migration_schemas(malformed).is_err(), "{malformed}");
        }
    }

    fn chain(entries: &[(i64, Scope)]) -> Vec<ChainEntry> {
        entries
            .iter()
            .map(|(t, scope)| ChainEntry {
                timestamp: *t,
                name: format!("m{t}"),
                scope: scope.clone(),
            })
            .collect()
    }

    fn versions(c: &[ChainEntry], idx: Vec<usize>) -> Vec<i64> {
        idx.into_iter().map(|i| c[i].timestamp).collect()
    }

    #[test]
    fn a_run_waits_only_on_overlapping_earlier_migrations() {
        let c = chain(&[
            (1, None),
            (2, s(&["sales"])),
            (3, s(&["billing"])),
            (4, s(&["sales", "billing"])),
            (5, None),
            (6, s(&["sales"])),
        ]);
        let applied: HashSet<i64> = [1].into();

        // Disjoint schemas don't wait on each other; a cross-schema migration waits on both, a
        // whole-database one on everything.
        assert_eq!(blocked_by(&c, &applied, 2).map(|j| c[j].timestamp), None);
        assert_eq!(blocked_by(&c, &applied, 3).map(|j| c[j].timestamp), Some(2));
        assert_eq!(blocked_by(&c, &applied, 4).map(|j| c[j].timestamp), Some(2));
        // An explicit single run is the confirmed override.
        assert_eq!(
            versions(&c, plan_run(&c, &applied, &RunSelection::Only(4)).unwrap()),
            vec![4]
        );

        // Running for a schema takes what declares it, refuses past a pending whole-database
        // migration, and refuses when a selected migration also needs another schema's.
        let for_schema =
            |a: &HashSet<i64>, x: &str| plan_run(&c, a, &RunSelection::Schema(x.into()));
        let err = for_schema(&applied, "sales").unwrap_err().to_string();
        assert!(err.contains("4 (m4)") && err.contains("3 (m3)"), "{err}");
        let applied: HashSet<i64> = [1, 3].into();
        let err = for_schema(&applied, "sales").unwrap_err().to_string();
        assert!(
            err.contains("5 (m5)") && err.contains("whole database"),
            "{err}"
        );
        let applied: HashSet<i64> = [1, 3, 5].into();
        assert_eq!(
            versions(&c, for_schema(&applied, "sales").unwrap()),
            vec![2, 4, 6]
        );
        // Everything pending in order needs no check: each earlier pending one is in the run.
        assert_eq!(
            versions(
                &c,
                plan_run(&c, &[1].into(), &RunSelection::UpTo(None)).unwrap()
            ),
            vec![2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn a_rollback_waits_on_later_overlapping_migrations() {
        let c = chain(&[
            (1, None),
            (2, s(&["sales"])),
            (3, s(&["billing"])),
            (4, s(&["sales"])),
        ]);
        let applied: HashSet<i64> = [1, 2, 3, 4].into();
        let blocker = |i| blocked_by(&c, &applied, i).map(|j| c[j].timestamp);
        assert_eq!(blocker(2), None);
        assert_eq!(blocker(1), Some(4));
        assert_eq!(blocker(0), Some(4));
    }
}
