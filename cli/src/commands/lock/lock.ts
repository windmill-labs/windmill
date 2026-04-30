import { Command } from "@cliffy/command";
import { colors } from "@cliffy/ansi/colors";
import { writeFile } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";
import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { readLockfile, LockVersion, suppressV2Nudge } from "../../utils/metadata.ts";
import { yamlOptions } from "../sync/sync.ts";
import { rehashOnly } from "../generate-metadata/generate-metadata.ts";

const WMILL_LOCKFILE = "wmill-lock.yaml";
const TARGET_VERSION: LockVersion = "v3";

async function upgrade(opts: GlobalOptions & SyncOptions) {
  opts = await mergeConfigWithConfigFile(opts);
  // Suppress the "Tip: run wmill lock upgrade" nudge — that's literally the
  // command being run; printing it during the upgrade is confusing.
  suppressV2Nudge();
  const conf = await readLockfile();
  if (conf.version === TARGET_VERSION) {
    log.info(colors.green(`wmill-lock.yaml already at ${TARGET_VERSION}, nothing to do.`));
    return;
  }

  const fromVersion = conf.version ?? "v1";
  log.info(`Upgrading wmill-lock.yaml from ${fromVersion} to ${TARGET_VERSION}...`);

  // Rewrite every entry with the canonical hash formula.
  await rehashOnly(opts);

  // Bump the version field. rehash-only deliberately doesn't do this — bumping
  // to v3 is the explicit migration step that disables the legacy fallback.
  const refreshed = await readLockfile();
  refreshed.version = TARGET_VERSION;
  await writeFile(
    WMILL_LOCKFILE,
    yamlStringify(refreshed as Record<string, any>, yamlOptions),
    "utf-8",
  );
  log.info(colors.green(`wmill-lock.yaml upgraded to ${TARGET_VERSION}.`));
}

const command = new Command()
  .description("Manage wmill-lock.yaml")
  .command(
    "upgrade",
    new Command()
      .description(
        "Migrate wmill-lock.yaml to the latest format (currently v3). Walks all " +
        "local scripts/flows/apps, rewrites entries with canonical hashes from " +
        "disk, and bumps the version field. No backend trip required."
      )
      .action(upgrade as any),
  );

export default command;
