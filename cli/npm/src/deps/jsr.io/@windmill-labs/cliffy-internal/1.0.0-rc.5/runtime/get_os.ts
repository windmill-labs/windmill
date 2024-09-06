/**
 * Get operating system name.
 *
 * @internal
 */
export function getOs():
  | "darwin"
  | "linux"
  | "android"
  | "windows"
  | "freebsd"
  | "netbsd"
  | "aix"
  | "solaris"
  | "illumos"
  | "openbsd"
  | "sunos"
  | "win32" {
  // dnt-shim-ignore deno-lint-ignore no-explicit-any
  const { Deno, process } = globalThis as any;

  if (Deno) {
    return Deno.build.os;
  } else if (process) {
    return process.platform;
  } else {
    throw new Error("unsupported runtime");
  }
}
