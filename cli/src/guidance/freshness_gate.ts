/**
 * Argv-only gate for the prompts freshness check. Kept in its own module so
 * `main.ts` can import it without pulling in the heavy `skills.gen.ts`
 * bundle (~360 KB) on every `wmill` invocation. The full check (which does
 * touch the bundle) lives in `./freshness.ts` and is loaded lazily after
 * this gate returns `true`.
 */

/**
 * Subcommands where a freshness warning is noise (the user is either fixing
 * it, asking for help, or doing something orthogonal).
 */
const SKIP_FRESHNESS_FOR_SUBCOMMANDS = new Set([
  "init",
  "refresh",
  "completions",
  "upgrade",
]);

/**
 * Cliffy global options that consume the *next* argv element as their value.
 * Must be kept in sync with the option declarations on the top-level
 * `command` in `cli/src/main.ts`.
 */
const VALUE_GLOBAL_OPTS = new Set([
  "--workspace",
  "--token",
  "--base-url",
  "--config-dir",
]);

/**
 * Returns `true` if the freshness check should run for this invocation.
 *
 * Bypasses:
 * - bare `wmill` (no subcommand → shows help)
 * - `--help`, `-h`, `--version`, `-V` anywhere in the args
 * - subcommands in {init, refresh, completions, upgrade}
 *
 * Handles cliffy global options that take a value (`--workspace foo`,
 * `--token tok`, `--base-url https://…`, `--config-dir /etc/wmill`) by
 * skipping their value when scanning for the first positional argument.
 * Without that, `wmill --workspace prod refresh prompts` would misread
 * `"prod"` as the subcommand and fire the warning during the very command
 * meant to fix it.
 */
export function shouldRunFreshnessCheck(argv: readonly string[]): boolean {
  const args = argv.slice(2); // strip node + script
  if (args.length === 0) return false;
  if (args.includes("--help") || args.includes("-h")) return false;
  if (args.includes("--version") || args.includes("-V")) return false;

  let i = 0;
  while (i < args.length) {
    const arg = args[i];
    if (VALUE_GLOBAL_OPTS.has(arg)) {
      i += 2; // skip flag + its value
      continue;
    }
    if (arg.startsWith("-")) {
      i += 1; // flag with no value
      continue;
    }
    return !SKIP_FRESHNESS_FOR_SUBCOMMANDS.has(arg);
  }
  return false;
}
