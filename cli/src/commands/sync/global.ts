import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";

let GLOBAL_VERSIONS: {
  remoteMajor: number | undefined;
  remoteMinor: number | undefined;
} = {
  remoteMajor: undefined,
  remoteMinor: undefined,
};

export function updateGlobalVersions(version: string) {
  try {
    const [prefix, remoteMinorStr] = version.split(".");

    GLOBAL_VERSIONS = {
      remoteMajor: parseInt(prefix.split("v")[1]),
      remoteMinor: parseInt(remoteMinorStr),
    };
  } catch (e) {
    log.info(colors.gray(`Error reading remote version: ${e}`));
  }
}

export function isVersionsGeq1585(): boolean {
  return (
    GLOBAL_VERSIONS.remoteMajor !== undefined &&
    GLOBAL_VERSIONS.remoteMajor >= 1 &&
    GLOBAL_VERSIONS.remoteMinor !== undefined &&
    GLOBAL_VERSIONS.remoteMinor >= 585
  );
}
