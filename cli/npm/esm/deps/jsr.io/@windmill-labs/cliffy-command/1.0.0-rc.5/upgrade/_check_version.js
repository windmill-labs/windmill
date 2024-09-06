import { bold, yellow } from "../../../../@std/fmt/0.225.6/colors.js";
import { Command } from "../command.js";
/** Check if new version is available and add hint to version. */
// deno-lint-ignore no-explicit-any
export async function checkVersion(cmd) {
    const mainCommand = cmd.getMainCommand();
    const upgradeCommand = mainCommand.getCommand("upgrade");
    if (!isUpgradeCommand(upgradeCommand)) {
        return;
    }
    const latestVersion = await upgradeCommand.getLatestVersion();
    const currentVersion = mainCommand.getVersion();
    if (!currentVersion || currentVersion === latestVersion) {
        return;
    }
    const versionHelpText = `(New version available: ${latestVersion}. Run '${mainCommand.getName()} upgrade' to upgrade to the latest version!)`;
    mainCommand.version(`${currentVersion}  ${bold(yellow(versionHelpText))}`);
}
function isUpgradeCommand(command) {
    return command instanceof Command && "getLatestVersion" in command;
}
