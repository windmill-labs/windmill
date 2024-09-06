import * as dntShim from "./_dnt.shims.js";
import { log, yamlParse } from "./deps.js";
export async function readConfigFile() {
    try {
        const conf = yamlParse(await dntShim.Deno.readTextFile("wmill.yaml"));
        if (conf?.defaultTs == undefined) {
            log.warn("No defaultTs defined in your wmill.yaml. Using 'bun' as default.");
        }
        return typeof conf == "object" ? conf : {};
    }
    catch (e) {
        log.warn("No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime.");
        return {};
    }
}
export async function mergeConfigWithConfigFile(opts) {
    const configFile = await readConfigFile();
    return Object.assign(configFile, opts);
}
