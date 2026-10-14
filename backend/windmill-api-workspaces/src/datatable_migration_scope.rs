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

use std::collections::{HashMap, HashSet};

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
pub(crate) fn is_annotatable_schema_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-'))
}

/// `code` with its scope annotation, ahead of everything else.
pub(crate) fn with_schemas_annotation(code: &str, schemas: &[&str]) -> String {
    format!("-- {KEYWORD} {}\n{code}", schemas.join(", "))
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

/// Group `names` (schemas, or extensions) in an order that replays: `deps` holds
/// `(dependent, referenced)` pairs, and a name comes after every name it references. Names that
/// reference each other, directly or through others, cannot be split and share one group: for
/// schemas, one migration declaring all of them.
pub(crate) fn order_referenced_first(
    names: &[String],
    deps: &[(String, String)],
) -> Vec<Vec<String>> {
    let mut names: Vec<&String> = names.iter().collect();
    names.sort();
    names.dedup();
    let index: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut edges: Vec<Vec<usize>> = vec![vec![]; names.len()];
    for (dependent, referenced) in deps {
        if let (Some(&a), Some(&b)) = (
            index.get(dependent.as_str()),
            index.get(referenced.as_str()),
        ) {
            if a != b && !edges[a].contains(&b) {
                edges[a].push(b);
            }
        }
    }
    for e in &mut edges {
        e.sort();
    }

    // Tarjan: components come out referenced-first, since a component is emitted only once
    // everything reachable from it has been.
    struct State<'a> {
        edges: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        groups: Vec<Vec<usize>>,
    }
    fn visit(s: &mut State, v: usize) {
        s.index[v] = Some(s.next);
        s.low[v] = s.next;
        s.next += 1;
        s.stack.push(v);
        s.on_stack[v] = true;
        for k in 0..s.edges[v].len() {
            let w = s.edges[v][k];
            match s.index[w] {
                None => {
                    visit(s, w);
                    s.low[v] = s.low[v].min(s.low[w]);
                }
                Some(iw) if s.on_stack[w] => s.low[v] = s.low[v].min(iw),
                Some(_) => {}
            }
        }
        if Some(s.low[v]) == s.index[v] {
            let mut group = vec![];
            while let Some(w) = s.stack.pop() {
                s.on_stack[w] = false;
                group.push(w);
                if w == v {
                    break;
                }
            }
            group.sort();
            s.groups.push(group);
        }
    }
    let mut state = State {
        edges: &edges,
        index: vec![None; names.len()],
        low: vec![0; names.len()],
        on_stack: vec![false; names.len()],
        stack: vec![],
        next: 0,
        groups: vec![],
    };
    for v in 0..names.len() {
        if state.index[v].is_none() {
            visit(&mut state, v);
        }
    }
    state
        .groups
        .into_iter()
        .map(|g| g.into_iter().map(|i| names[i].clone()).collect())
        .collect()
}

/// The scope of the snapshot of `group`: its schemas and every schema they reference. Declaring
/// the referenced ones is what keeps a run for one schema from replaying it ahead of the snapshot
/// that creates what it references.
pub(crate) fn snapshot_scope(group: &[String], deps: &[(String, String)]) -> Vec<String> {
    let mut scope: Vec<String> = group.to_vec();
    for (dependent, referenced) in deps {
        if group.contains(dependent) && !scope.contains(referenced) {
            scope.push(referenced.clone());
        }
    }
    scope
}

/// Cross-schema dependencies between the objects of a database, as `(dependent, referenced)`
/// schema names. Sub-objects (column defaults, view rules, triggers, policies) are attributed to
/// their table's schema. Members of an extension are left out: the extensions snapshot creates
/// them ahead of every schema. Every catalog of schema-qualified objects is listed: one left out
/// drops its dependencies from the join, and its snapshot then replays ahead of what it needs.
pub(crate) const CROSS_SCHEMA_DEPENDENCIES_SQL: &str = "\
WITH objs(classid, objid, nsp) AS (
    SELECT 'pg_class'::regclass::oid, oid, relnamespace FROM pg_class
    UNION ALL SELECT 'pg_type'::regclass::oid, oid, typnamespace FROM pg_type
    UNION ALL SELECT 'pg_proc'::regclass::oid, oid, pronamespace FROM pg_proc
    UNION ALL SELECT 'pg_constraint'::regclass::oid, oid, connamespace FROM pg_constraint
    UNION ALL SELECT 'pg_collation'::regclass::oid, oid, collnamespace FROM pg_collation
    UNION ALL SELECT 'pg_operator'::regclass::oid, oid, oprnamespace FROM pg_operator
    UNION ALL SELECT 'pg_opclass'::regclass::oid, oid, opcnamespace FROM pg_opclass
    UNION ALL SELECT 'pg_opfamily'::regclass::oid, oid, opfnamespace FROM pg_opfamily
    UNION ALL SELECT 'pg_conversion'::regclass::oid, oid, connamespace FROM pg_conversion
    UNION ALL SELECT 'pg_statistic_ext'::regclass::oid, oid, stxnamespace FROM pg_statistic_ext
    UNION ALL SELECT 'pg_ts_config'::regclass::oid, oid, cfgnamespace FROM pg_ts_config
    UNION ALL SELECT 'pg_ts_dict'::regclass::oid, oid, dictnamespace FROM pg_ts_dict
    UNION ALL SELECT 'pg_ts_parser'::regclass::oid, oid, prsnamespace FROM pg_ts_parser
    UNION ALL SELECT 'pg_ts_template'::regclass::oid, oid, tmplnamespace FROM pg_ts_template
    UNION ALL SELECT 'pg_rewrite'::regclass::oid, r.oid, c.relnamespace
        FROM pg_rewrite r JOIN pg_class c ON c.oid = r.ev_class
    UNION ALL SELECT 'pg_attrdef'::regclass::oid, a.oid, c.relnamespace
        FROM pg_attrdef a JOIN pg_class c ON c.oid = a.adrelid
    UNION ALL SELECT 'pg_trigger'::regclass::oid, t.oid, c.relnamespace
        FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
    UNION ALL SELECT 'pg_policy'::regclass::oid, p.oid, c.relnamespace
        FROM pg_policy p JOIN pg_class c ON c.oid = p.polrelid
)
SELECT DISTINCT dn.nspname AS dependent, rn.nspname AS referenced
FROM pg_depend d
JOIN objs a ON a.classid = d.classid AND a.objid = d.objid
JOIN objs b ON b.classid = d.refclassid AND b.objid = d.refobjid
JOIN pg_namespace dn ON dn.oid = a.nsp
JOIN pg_namespace rn ON rn.oid = b.nsp
WHERE a.nsp <> b.nsp
  AND d.deptype IN ('n', 'a')
  AND NOT EXISTS (SELECT 1 FROM pg_depend e
                  WHERE e.classid = d.refclassid AND e.objid = d.refobjid AND e.deptype = 'e')
  AND NOT EXISTS (SELECT 1 FROM pg_depend e
                  WHERE e.classid = d.classid AND e.objid = d.objid AND e.deptype = 'e')";

/// The schemas a snapshot covers: everything but the system schemas and those an extension owns.
pub(crate) const SNAPSHOT_SCHEMAS_SQL: &str = "\
SELECT n.nspname,
       EXISTS (SELECT 1 FROM pg_depend d
               WHERE d.classid = 'pg_namespace'::regclass AND d.objid = n.oid
                 AND d.deptype = 'e') AS extension_owned
FROM pg_namespace n
WHERE n.nspname NOT LIKE 'pg\\_%' AND n.nspname <> 'information_schema'
ORDER BY n.nspname";

/// Whether the database holds objects that belong to no schema besides extensions: `pg_dump
/// --schema` leaves those out, so a per-schema snapshot would drop them (and a foreign table could
/// not replay without its server). Extension members are skipped, `CREATE EXTENSION` recreates them.
pub(crate) const UNSCHEMED_OBJECTS_SQL: &str = "\
SELECT EXISTS (SELECT 1 FROM pg_publication)
    OR EXISTS (SELECT 1 FROM pg_event_trigger)
    OR EXISTS (SELECT 1 FROM pg_foreign_server)
    OR EXISTS (SELECT 1 FROM pg_foreign_data_wrapper w WHERE NOT EXISTS (SELECT 1 FROM pg_depend d
               WHERE d.classid = 'pg_foreign_data_wrapper'::regclass AND d.objid = w.oid
                 AND d.deptype = 'e'))
    OR EXISTS (SELECT 1 FROM pg_cast c WHERE c.oid >= 16384 AND NOT EXISTS (SELECT 1 FROM pg_depend d
               WHERE d.classid = 'pg_cast'::regclass AND d.objid = c.oid AND d.deptype = 'e'))
    OR EXISTS (SELECT 1 FROM pg_language l WHERE l.oid >= 16384 AND NOT EXISTS (SELECT 1
               FROM pg_depend d WHERE d.classid = 'pg_language'::regclass AND d.objid = l.oid
                 AND d.deptype = 'e'))
    OR EXISTS (SELECT 1 FROM pg_am a WHERE a.oid >= 16384 AND NOT EXISTS (SELECT 1
               FROM pg_depend d WHERE d.classid = 'pg_am'::regclass AND d.objid = a.oid
                 AND d.deptype = 'e'))
    OR EXISTS (SELECT 1 FROM pg_transform t WHERE NOT EXISTS (SELECT 1
               FROM pg_depend d WHERE d.classid = 'pg_transform'::regclass AND d.objid = t.oid
                 AND d.deptype = 'e'))
    OR EXISTS (SELECT 1 FROM pg_subscription s
               WHERE s.subdbid = (SELECT oid FROM pg_database WHERE datname = current_database()))
    AS present";

/// Default privileges set for the whole database rather than one schema, which `pg_dump --schema`
/// leaves out too. Only checked when the snapshot keeps privileges at all.
pub(crate) const DATABASE_DEFAULT_ACL_SQL: &str = "\
SELECT EXISTS (SELECT 1 FROM pg_default_acl WHERE defaclnamespace = 0) AS present";

/// `(dependent, required)` extension pairs, so an extension is created after the ones its control
/// file requires.
pub(crate) const EXTENSION_DEPENDENCIES_SQL: &str = "\
SELECT e.extname, r.extname FROM pg_depend d
JOIN pg_extension e ON e.oid = d.objid
JOIN pg_extension r ON r.oid = d.refobjid
WHERE d.classid = 'pg_extension'::regclass AND d.refclassid = 'pg_extension'::regclass";

pub(crate) const SNAPSHOT_EXTENSIONS_SQL: &str = "\
SELECT e.extname, n.nspname FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace
WHERE e.extname <> 'plpgsql'
ORDER BY e.extname";

/// A `pg_dump --schema` snapshot made replayable on a database that already has the schema: a
/// per-schema dump creates `public` too, which every database has, where a whole-database dump
/// leaves it out.
pub(crate) fn make_schema_snapshot_replayable(dump: &str) -> String {
    dump.lines()
        .filter(|line| line.trim() != "COMMENT ON SCHEMA public IS 'standard public schema';")
        .map(|line| match line.strip_prefix("CREATE SCHEMA ") {
            Some(rest) if !rest.starts_with("IF NOT EXISTS ") => {
                format!("CREATE SCHEMA IF NOT EXISTS {rest}")
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a schema snapshot creates anything besides the schema itself.
pub(crate) fn snapshot_has_objects(dump: &str) -> bool {
    dump.lines().map(str::trim).any(|line| {
        !(line.is_empty()
            || line.starts_with("--")
            || line.starts_with("SET ")
            || line.starts_with("SELECT pg_catalog.set_config(")
            || line.starts_with("CREATE SCHEMA ")
            || line.starts_with("COMMENT ON SCHEMA "))
    })
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

    #[test]
    fn schema_groups_come_referenced_first_and_merge_cycles() {
        let names = |v: &[&str]| v.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let dep = |a: &str, b: &str| (a.to_string(), b.to_string());
        let deps = [
            dep("public", "sales"),
            dep("sales", "billing"),
            dep("billing", "sales"),
            dep("crm", "public"),
        ];
        assert_eq!(
            order_referenced_first(
                &names(&["public", "sales", "billing", "audit", "crm"]),
                &deps
            ),
            vec![
                names(&["audit"]),
                names(&["billing", "sales"]),
                names(&["public"]),
                names(&["crm"])
            ]
        );
        // A snapshot declares what it references, so a run for `crm` waits on `public`'s.
        assert_eq!(
            snapshot_scope(&names(&["crm"]), &deps),
            names(&["crm", "public"])
        );
    }

    #[test]
    fn a_schema_snapshot_replays_onto_an_existing_schema() {
        let dump = "CREATE SCHEMA public;\nCOMMENT ON SCHEMA public IS 'standard public schema';\nCREATE SCHEMA \"Sales\";\nCREATE TABLE public.t (id int);";
        let fixed = make_schema_snapshot_replayable(dump);
        assert_eq!(
            fixed,
            "CREATE SCHEMA IF NOT EXISTS public;\nCREATE SCHEMA IF NOT EXISTS \"Sales\";\nCREATE TABLE public.t (id int);"
        );
        assert!(snapshot_has_objects(&fixed));
        assert!(!snapshot_has_objects(
            "--\nSET x = 1;\nCREATE SCHEMA IF NOT EXISTS public;"
        ));
    }
}
