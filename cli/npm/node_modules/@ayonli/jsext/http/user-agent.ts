import type { RuntimeInfo, WellknownPlatform } from "../runtime.ts";

/**
 * Represents the user agent information.
 * 
 * Most browsers use the `User-Agent` or the `navigator.userAgent` string to
 * indicate what they are compatible instead of what they are made of. This
 * interface tries to represent the actual user agent information as accurate
 * as possible.
 */
export interface UserAgentInfo {
    /**
     * The name of the user agent, e.g. `Chrome`, `Firefox`, `Safari`, `Edge`,
     * etc.
     */
    name: string;
    /**
     * The version of the user agent, e.g. `127.0.0.0`. This value can be
     * `undefined` if the version is not provided.
     */
    version?: string | undefined;
    /**
     * JavaScript runtime information defined by the user agent, only available
     * if the user agent is a browser or a JavaScript server runtime.
     */
    runtime?: Pick<RuntimeInfo, "identity" | "version"> | undefined;
    /**
     * The platform on which the user agent is running, e.g. `darwin`, `windows`,
     * `linux`, `android`, etc.
     */
    platform: WellknownPlatform | "unknown";
    /**
     * Whether the user agent is running on a mobile device, includes tablets.
     */
    mobile: boolean;
    /**
     * The original user agent string passed into {@link parseUserAgent}.
     */
    raw?: string;
}

/**
 * Parses the `User-Agent` header or the `navigator.userAgent` property.
 * 
 * @example
 * ```ts
 * const ua = parseUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");
 * console.log(ua);
 * // {
 * //     name: "Chrome",
 * //     version: "91.0.4472.124",
 * //     runtime: { identity: "chrome", version: "91.0.4472.124" },
 * //     platform: "windows",
 * //     mobile: false,
 * //     raw: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
 * // }
 * ```
 */
export function parseUserAgent(str: string): UserAgentInfo {
    if (!str)
        throw new TypeError("The user agent string cannot be empty");

    const ua: UserAgentInfo = {
        name: "",
        version: undefined,
        runtime: undefined,
        platform: "unknown",
        mobile: /Mobile/i.test(str),
        raw: str,
    };
    type Match = {
        name: string;
        version: string;
        extra?: string;
    };
    let matches = [...str.matchAll(/(\S+)\/(\d+(\.\d+)*)(\s\((.+?)\))?/g)].map((match) => {
        const [, name, version, , , extra] = match;
        return { name: name!, version: version!, extra };
    }) as Match[];

    if (matches.length === 0 && str.length) {
        ua.name = str;

        if (/Cloudflare[-\s]Workers?/i.test(str)) {
            ua.runtime = { identity: "workerd", version: undefined };
        }

        return ua;
    }

    const osInfo = matches.find(match => match.extra?.includes(";"))?.extra;
    if (osInfo) {
        if (osInfo.includes("Macintosh") ||
            osInfo.includes("iPhone") ||
            osInfo.includes("iPad") ||
            osInfo.includes("iPod")
        ) {
            ua.platform = "darwin";
        } else if (osInfo.includes("Windows")) {
            ua.platform = "windows";
        } else if (osInfo.includes("Android")) {
            ua.platform = "android";
        } else if (osInfo.includes("Linux")) {
            ua.platform = "linux";
        } else if (osInfo.includes("FreeBSD")) {
            ua.platform = "freebsd";
        } else if (osInfo.includes("OpenBSD")) {
            ua.platform = "openbsd";
        } else if (osInfo.includes("NetBSD")) {
            ua.platform = "netbsd";
        } else if (osInfo.includes("AIX")) {
            ua.platform = "aix";
        } else if (osInfo.match(/SunOS|Solaris/)) {
            ua.platform = "solaris";
        }
    }

    let safari = matches.find(match => match.name === "Safari");
    const chrome = matches.find(match => /Chrome|CriOS/i.test(match.name));
    const firefox = matches.find(match => /Firefox|FxiOS/i.test(match.name));
    const opera = matches.find(match => /Opera|OPR(iOS)?|OPT|OPR/i.test(match.name));
    const edge = matches.find(match => /Edg(e|A|iOS)?/i.test(match.name));
    const duckDuckGo = matches.find(match => /DuckDuckGo|Ddg(A|iOS)?/i.test(match.name));
    const weChat = matches.find(match => /(Mac)?WeChat/i.test(match.name))
        ?? matches.find(match => /MicroMessenger/i.test(match.name));
    const dingTalk = str.match(/\b(DingTalk)\/(\d+(\.\d+)+)/);
    let node: Match | undefined;
    let deno: Match | undefined;
    let bun: Match | undefined;

    if (ua.mobile && ua.platform === "darwin" && !safari && !chrome && !firefox) {
        safari = matches.find(match => match.name === "AppleWebKit"); // fallback to WebKit
    }

    if (ua.mobile && ua.platform === "darwin" && safari) {
        // In iOS and iPadOS, browsers are always Safari-based.
        ua.runtime = { identity: "safari", version: safari.version };

        if (opera) {
            ua.name = "Opera";
            ua.version = opera.version;
        } else if (edge) {
            ua.name = "Edge";
            ua.version = edge.version;
        } else if (chrome) {
            ua.name = "Chrome";
            ua.version = chrome.version;
        } else if (firefox) {
            ua.name = "Firefox";
            ua.version = firefox.version;
        } else if (duckDuckGo) {
            ua.name = "DuckDuckGo";
            ua.version = duckDuckGo.version;
        } else if (weChat) {
            ua.name = "WeChat";
            ua.version = weChat.version;
        } else if (dingTalk) {
            ua.name = "DingTalk";
            ua.version = dingTalk[2]!;
        } else {
            const last = matches[matches.length - 1]!;
            ua.name = last.name;
            ua.version = last.version;
        }
    } else if (safari && !chrome && !firefox) {
        ua.runtime = { identity: "safari", version: safari.version };

        if (duckDuckGo) {
            ua.name = "DuckDuckGo";
            ua.version = duckDuckGo.version;
        } else if (weChat) {
            ua.name = "WeChat";
            ua.version = weChat.version;
        } else if (dingTalk) {
            ua.name = "DingTalk";
            ua.version = dingTalk[2]!;
        } else {
            const index = matches.findIndex(match => match === safari);
            const next = matches[index + 1] ?? matches[matches.length - 1]!;
            ua.name = next.name;
            ua.version = next.version;
        }
    } else if (chrome && !firefox) {
        ua.runtime = { identity: "chrome", version: chrome.version };

        if (opera) {
            ua.name = "Opera";
            ua.version = opera.version;
        } else if (edge) {
            ua.name = "Edge";
            ua.version = edge.version;
        } else if (duckDuckGo) {
            ua.name = "DuckDuckGo";
            ua.version = duckDuckGo.version;
        } else if (weChat) {
            ua.name = "WeChat";
            ua.version = weChat.version;
        } else if (dingTalk) {
            ua.name = "DingTalk";
            ua.version = dingTalk[2]!;
        } else {
            const index = matches.findIndex(match => match === chrome);
            const next = matches[index + 1] ?? matches[matches.length - 1]!;

            if (next.name === "Safari") { // Chrome pretending to be Safari
                ua.name = "Chrome";
                ua.version = chrome.version;
            } else {
                ua.name = next.name;
                ua.version = next.version;
            }
        }
    } else if (firefox && !chrome) {
        ua.runtime = { identity: "firefox", version: firefox.version };
        const index = matches.findIndex(match => match === firefox);
        const next = matches[index + 1] ?? matches[matches.length - 1]!;

        if (next.name === "Safari") { // Firefox pretending to be Safari
            ua.name = "Firefox";
            ua.version = firefox.version;
        } else {
            ua.name = next.name;
            ua.version = next.version;
        }
    } else if (edge) { // Old Edge
        ua.name = "Edge";
        ua.version = edge.version;
        ua.runtime = { identity: "unknown", version: edge.version };
    } else if (duckDuckGo) {
        ua.name = "DuckDuckGo";
        ua.version = duckDuckGo.version;
        ua.runtime = { identity: "unknown", version: duckDuckGo.version };
    } else if (node = matches.find(match => match.name === "Node.js")) {
        ua.name = "Node.js";
        ua.version = node.version;
        ua.runtime = { identity: "node", version: node.version };
    } else if (deno = matches.find(match => match.name === "Deno")) {
        ua.name = "Deno";
        ua.version = deno.version;
        ua.runtime = { identity: "deno", version: deno.version };
    } else if (bun = matches.find(match => match.name === "Bun")) {
        ua.name = "Bun";
        ua.version = bun.version;
        ua.runtime = { identity: "bun", version: bun.version };
    } else if (dingTalk) {
        ua.name = "DingTalk";
        ua.version = dingTalk[2]!;
    } else {
        const last = matches[matches.length - 1]!;
        ua.name = last.name;
        ua.version = last.version;
    }

    return ua;
}
