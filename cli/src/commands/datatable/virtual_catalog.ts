const CURRENT_DATABASE_QUERY_RE = /\b(?:pg_catalog\.)?current_database\s*\(\s*\)/i;
const CURRENT_DATABASE_RE = /\b(?:pg_catalog\.)?current_database\s*\(\s*\)/gi;
const PG_DATABASE_RE = /\b(?:pg_catalog\.)?pg_database\b/i;
const PG_DATABASE_FROM_OR_JOIN_RE =
  /(\b(?:from|join)\s+(?:only\s+)?)(?:pg_catalog\.)?pg_database\b/gi;
const PG_DATABASE_COMMA_JOIN_RE = /(,\s*)(?:pg_catalog\.)?pg_database\b/gi;
const PG_SIZE_PRETTY_DATABASE_SIZE_RE =
  /\b(?:pg_catalog\.)?pg_size_pretty\s*\(\s*(?:pg_catalog\.)?pg_database_size\s*\((?:[^()]|\([^()]*\))*\)\s*\)/gi;
const PG_DATABASE_SIZE_RE =
  /\b(?:pg_catalog\.)?pg_database_size\s*\((?:[^()]|\([^()]*\))*\)/gi;
const HAS_DATABASE_PRIVILEGE_RE =
  /\b(?:pg_catalog\.)?has_database_privilege\s*\((?:[^()]|\([^()]*\))*\)/gi;
const PG_DATABASE_COLLATION_VERSION_RE =
  /\b(?:pg_catalog\.)?pg_database_collation_actual_version\s*\((?:[^()]|\([^()]*\))*\)/gi;

export function queryTouchesVirtualDatabaseCatalog(query: string): boolean {
  return CURRENT_DATABASE_QUERY_RE.test(query) || PG_DATABASE_RE.test(query);
}

export function virtualizeDatabaseCatalogQuery(
  query: string,
  currentDatabaseName: string,
  databaseNames: string[],
): string {
  let rewritten = query.replace(
    CURRENT_DATABASE_RE,
    sqlStringLiteral(currentDatabaseName),
  );

  if (!PG_DATABASE_RE.test(query)) {
    return rewritten;
  }

  const relation = buildVirtualPgDatabaseRelation(databaseNames);
  rewritten = rewritten
    .replace(PG_SIZE_PRETTY_DATABASE_SIZE_RE, "NULL::text")
    .replace(PG_DATABASE_SIZE_RE, "NULL::bigint")
    .replace(HAS_DATABASE_PRIVILEGE_RE, "true")
    .replace(PG_DATABASE_COLLATION_VERSION_RE, "NULL::text")
    .replace(PG_DATABASE_FROM_OR_JOIN_RE, `$1${relation}`)
    .replace(PG_DATABASE_COMMA_JOIN_RE, `$1${relation}`);

  return rewritten;
}

function buildVirtualPgDatabaseRelation(databaseNames: string[]): string {
  const names = Array.from(new Set(databaseNames));
  const arrayLiteral = names.map(sqlStringLiteral).join(", ");

  return `(
  SELECT
    (20000 + src.ordinality)::oid AS oid,
    src.datname,
    COALESCE((SELECT oid FROM pg_catalog.pg_roles WHERE rolname = CURRENT_USER LIMIT 1), 10::oid) AS datdba,
    pg_char_to_encoding(current_setting('server_encoding')) AS encoding,
    'c'::"char" AS datlocprovider,
    false AS datistemplate,
    true AS datallowconn,
    -1 AS datconnlimit,
    '0'::xid AS datfrozenxid,
    '1'::xid AS datminmxid,
    COALESCE((SELECT oid FROM pg_catalog.pg_tablespace WHERE spcname = 'pg_default' LIMIT 1), 1663::oid) AS dattablespace,
    current_setting('lc_collate') AS datcollate,
    current_setting('lc_ctype') AS datctype,
    NULL::text AS daticulocale,
    NULL::text AS daticurules,
    NULL::text AS datcollversion,
    NULL::aclitem[] AS datacl
  FROM unnest(ARRAY[${arrayLiteral}]::text[]) WITH ORDINALITY AS src(datname, ordinality)
)`;
}

function sqlStringLiteral(value: string): string {
  return `'${value.replaceAll("'", "''")}'`;
}
