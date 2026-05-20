import { colors } from "@cliffy/ansi/colors";
import { spawn } from "node:child_process";

import * as log from "../../core/log.ts";
import { startServe, type ServeOpts } from "./serve.ts";

export interface PsqlOpts extends ServeOpts {
  name?: string;
}

const DEFAULT_DATATABLE_NAME = "main";

export async function psql(opts: PsqlOpts): Promise<void> {
  const datatableName = opts.name ?? DEFAULT_DATATABLE_NAME;
  const handle = await startServe(opts);
  const connectionString = handle.connectionString(datatableName);

  log.info(
    colors.gray(
      `Launching psql against datatable '${datatableName}' (proxy on ${handle.host}:${handle.port})`,
    ),
  );

  const child = spawn("psql", [connectionString], {
    stdio: "inherit",
  });

  // If psql isn't installed, surface that cleanly instead of a stack trace.
  child.on("error", (err) => {
    const code = (err as NodeJS.ErrnoException).code;
    if (code === "ENOENT") {
      log.error(
        "psql not found in PATH. Install the postgresql client (e.g. `apt install postgresql-client`, `brew install libpq`) and try again.",
      );
    } else {
      log.error(`failed to launch psql: ${err.message}`);
    }
    handle.close().finally(() => process.exit(1));
  });

  const exitCode = await new Promise<number>((resolve) => {
    child.on("exit", (code, signal) => {
      // Mirror shell convention: signal N → 128 + N.
      if (signal && typeof signal === "string") {
        const num = signalToNumber(signal);
        resolve(num !== undefined ? 128 + num : 1);
      } else {
        resolve(code ?? 0);
      }
    });
  });

  await handle.close();
  process.exit(exitCode);
}

function signalToNumber(name: string): number | undefined {
  const table: Record<string, number> = {
    SIGINT: 2,
    SIGQUIT: 3,
    SIGTERM: 15,
    SIGHUP: 1,
    SIGKILL: 9,
  };
  return table[name];
}
