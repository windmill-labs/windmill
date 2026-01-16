/**
 * Barrel file for configuration exports used by tests.
 * Re-exports key functions from the core conf module.
 */

export {
  readConfigFile,
  getEffectiveSettings,
  DEFAULT_SYNC_OPTIONS,
  type SyncOptions,
} from "./src/core/conf.ts";
