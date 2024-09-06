import * as dntShim from "./_dnt.shims.js";
import { ensureDir } from "./deps.js";
function hash_string(str) {
    let hash = 0, i, chr;
    if (str.length === 0)
        return hash;
    for (i = 0; i < str.length; i++) {
        chr = str.charCodeAt(i);
        hash = (hash << 5) - hash + chr;
        hash |= 0; // Convert to 32bit integer
    }
    return hash;
}
export async function getRootStore() {
    const store = (config_dir() ?? tmp_dir() ?? "/tmp/") + "/windmill/";
    await ensureDir(store);
    return store;
}
export async function getStore(baseUrl) {
    const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
    const baseStore = (await getRootStore()) + baseHash + "/";
    await ensureDir(baseStore);
    return baseStore;
}
//inlined import dir from "https://deno.land/x/dir/mod.ts";
function tmp_dir() {
    switch (dntShim.Deno.build.os) {
        case "linux": {
            const xdg = dntShim.Deno.env.get("XDG_RUNTIME_DIR");
            if (xdg)
                return `${xdg}-/tmp`;
            const tmpDir = dntShim.Deno.env.get("TMPDIR");
            if (tmpDir)
                return tmpDir;
            const tempDir = dntShim.Deno.env.get("TEMP");
            if (tempDir)
                return tempDir;
            const tmp = dntShim.Deno.env.get("TMP");
            if (tmp)
                return tmp;
            return "/var/tmp";
        }
        case "darwin":
            return dntShim.Deno.env.get("TMPDIR") ?? null;
        case "windows":
            return dntShim.Deno.env.get("TMP") ?? dntShim.Deno.env.get("TEMP") ?? null;
    }
    return null;
}
function config_dir() {
    switch (dntShim.Deno.build.os) {
        case "linux": {
            const xdg = dntShim.Deno.env.get("XDG_CONFIG_HOME");
            if (xdg)
                return xdg;
            const home = dntShim.Deno.env.get("HOME");
            if (home)
                return `${home}/.config`;
            break;
        }
        case "darwin": {
            const home = dntShim.Deno.env.get("HOME");
            if (home)
                return `${home}/Library/Preferences`;
            break;
        }
        case "windows":
            return dntShim.Deno.env.get("APPDATA") ?? null;
    }
    return null;
}
