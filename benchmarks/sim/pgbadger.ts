// Reusable pgBadger integration: turn a PostgreSQL stderr log into a rich HTML
// report (top queries by time + call counts, lock waits, temp files,
// checkpoints, connection/session timeline, query-type distribution).
//
// Used two ways:
//   1. The sim (`sim.ts --pgbadger`) sets these PG log settings on the
//      provisioned Postgres, captures its log after the run, and renders a
//      per-run `pgbadger.html` into the results folder.
//   2. Standalone, against ANY PostgreSQL log (e.g. from a manual benchmark
//      run against an external Windmill): enable PGBADGER_PG_SETTINGS on that
//      PG, capture its log, then:
//          deno run -A pgbadger.ts <pg.log> <out.html>
//
// pgBadger must be on PATH (it's in the flake devshell / `nix run nixpkgs#pgbadger`).

// Must stay in sync with PGBADGER_PG_SETTINGS' log_line_prefix below.
export const PGBADGER_LOG_PREFIX = "%t [%p]: user=%u,db=%d,app=%a,client=%h ";

// PostgreSQL settings that make the log pgBadger-parseable and rich. Pass each
// as a `-c name=value` to postgres. log_min_duration_statement=0 logs EVERY
// query with its duration (verbose — only enable when you want the report).
export const PGBADGER_PG_SETTINGS: string[] = [
  "log_min_duration_statement=0",
  "log_checkpoints=on",
  "log_connections=on",
  "log_disconnections=on",
  "log_lock_waits=on",
  "log_temp_files=0",
  "log_autovacuum_min_duration=0",
  "lc_messages=C", // pgBadger needs English log messages to parse them
  `log_line_prefix=${PGBADGER_LOG_PREFIX}`,
];

// Render a pgBadger HTML report from a PostgreSQL log file. Best-effort: logs a
// warning and returns false on failure rather than throwing.
export async function runPgbadger(logPath: string, outHtml: string): Promise<boolean> {
  try {
    const p = new Deno.Command("pgbadger", {
      args: ["--prefix", PGBADGER_LOG_PREFIX, "-f", "stderr", "-o", outHtml, logPath],
      stdout: "piped",
      stderr: "piped",
    });
    const { code, stderr } = await p.output();
    if (code !== 0) {
      console.warn(`[pgbadger] failed (${code}): ${new TextDecoder().decode(stderr).slice(0, 500)}`);
      return false;
    }
    return true;
  } catch (e) {
    console.warn(`[pgbadger] could not run (is it on PATH?): ${(e as Error).message}`);
    return false;
  }
}

if (import.meta.main) {
  const [log, out] = Deno.args;
  if (!log || !out) {
    console.error("usage: deno run -A pgbadger.ts <pg.log> <out.html>");
    Deno.exit(2);
  }
  const ok = await runPgbadger(log, out);
  if (ok) console.log(`pgBadger report written to ${out}`);
  Deno.exit(ok ? 0 : 1);
}
