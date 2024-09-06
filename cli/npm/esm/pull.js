import { colors, Command, JSZip, log } from "./deps.js";
import { getHeaders } from "./utils.js";
export async function downloadZip(workspace, plainSecrets, skipVariables, skipResources, skipSecrets, includeSchedules, includeUsers, includeGroups, includeSettings, includeKey, defaultTs) {
    const requestHeaders = new Headers();
    requestHeaders.set("Authorization", "Bearer " + workspace.token);
    requestHeaders.set("Content-Type", "application/octet-stream");
    const extraHeaders = getHeaders();
    if (extraHeaders) {
        for (const [key, value] of Object.entries(extraHeaders)) {
            requestHeaders.set(key, value);
        }
    }
    const zipResponse = await fetch(workspace.remote +
        "api/w/" +
        workspace.workspaceId +
        `/workspaces/tarball?archive_type=zip&plain_secret=${plainSecrets ?? false}&skip_variables=${skipVariables ?? false}&skip_resources=${skipResources ?? false}&skip_secrets=${skipSecrets ?? false}&include_schedules=${includeSchedules ?? false}&include_users=${includeUsers ?? false}&include_groups=${includeGroups ?? false}&include_settings=${includeSettings ?? false}&include_key=${includeKey ?? false}&default_ts=${defaultTs ?? "bun"}`, {
        headers: requestHeaders,
        method: "GET",
    });
    if (!zipResponse.ok) {
        console.log(colors.red("Failed to request tarball from API " + zipResponse.statusText));
        throw new Error(await zipResponse.text());
    }
    else {
        log.debug(`Downloaded zip/tarball successfully`);
    }
    const blob = await zipResponse.blob();
    return await JSZip.loadAsync((await blob.arrayBuffer()));
}
function stub(_opts, _dir) {
    console.log(colors.red.underline('Pull is deprecated. Use "sync pull --raw" instead. See <TODO_LINK_HERE> for more information.'));
}
const command = new Command()
    .description("Pull all definitions in the current workspace from the API and write them to disk.")
    .arguments("<dir:string>")
    .action(stub);
export default command;
