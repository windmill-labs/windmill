/**
 * Get script arguments.
 *
 * @internal
 */
export function getArgs(): Array<string> {
  // dnt-shim-ignore deno-lint-ignore no-explicit-any
  const { Deno, process } = globalThis as any;

  return Deno?.args ?? process?.argv.slice(2) ?? [];
}
