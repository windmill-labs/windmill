/**
 * Read from stdin.
 *
 * @internal
 * @param data Uint8Array to store the data.
 */
export function read(data: Uint8Array): Promise<number | null> {
  // dnt-shim-ignore deno-lint-ignore no-explicit-any
  const { Deno, process } = globalThis as any;

  if (Deno) {
    return Deno.stdin.read(data);
  } else if (process) {
    return new Promise((resolve, reject) => {
      process.stdin.once("readable", () => {
        try {
          const buffer = process.stdin.read();

          if (buffer === null) {
            return null;
          }

          for (let i = 0; i < buffer.length; i++) {
            data[i] = buffer[i];
          }

          resolve(buffer.length);
        } catch (error) {
          reject(error);
        }
      });
    });
  } else {
    throw new Error("unsupported runtime");
  }
}
