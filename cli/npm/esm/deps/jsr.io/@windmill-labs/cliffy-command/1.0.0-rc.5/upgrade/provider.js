import { bold, brightBlue, cyan, green, red, yellow } from "../../../../@std/fmt/0.225.6/colors.js";
import { ValidationError } from "../_errors.js";
import { Table } from "../../../cliffy-table/1.0.0-rc.5/mod.js";
/**
 * Upgrade provider.
 *
 * The upgrade provider is an api wrapper for a javascript registry which is
 * used by the upgrade command to upgrade the cli to a specific version.
 *
 * @example Upgrade provider example.
 *
 * ```
 * import { Command } from "@cliffy/command";
 * import { UpgradeCommand } from "@cliffy/command/upgrade";
 * import { DenoLandProvider } from "@cliffy/command/upgrade/provider/deno-land";
 * import { GithubProvider } from "@cliffy/command/upgrade/provider/github";
 * import { JsrProvider } from "@cliffy/command/upgrade/provider/jsr";
 * import { NestLandProvider } from "@cliffy/command/upgrade/provider/nest-land";
 * import { NpmProvider } from "@cliffy/command/upgrade/provider/npm";
 *
 * const upgradeCommand = new UpgradeCommand({
 *   provider: [
 *     new JsrProvider({ package: "@examples/package" }),
 *   ],
 * });
 * ```
 */
export class Provider {
    constructor({ main, logger = console } = {}) {
        Object.defineProperty(this, "main", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "maxListSize", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 25
        });
        Object.defineProperty(this, "logger", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: void 0
        });
        Object.defineProperty(this, "maxCols", {
            enumerable: true,
            configurable: true,
            writable: true,
            value: 8
        });
        this.main = main;
        this.logger = logger;
    }
    getSpecifier(name, version, defaultMain) {
        return `${this.getRegistryUrl(name, version)}${this.getMain(defaultMain)}`;
    }
    async isOutdated(name, currentVersion, targetVersion) {
        const { latest, versions } = await this.getVersions(name);
        if (targetVersion === "latest") {
            targetVersion = latest;
        }
        // Check if requested version exists.
        if (targetVersion && !versions.includes(targetVersion)) {
            throw new ValidationError(`The provided version ${bold(red(targetVersion))} is not found.\n\n    ${cyan(`Visit ${brightBlue(this.getRepositoryUrl(name))} for available releases or run again with the ${(yellow("-l"))} or ${(yellow("--list-versions"))} command.`)}`);
        }
        // Check if requested version is already the latest available version.
        if (latest && latest === currentVersion && latest === targetVersion) {
            this.logger.warn(yellow(`You're already using the latest available version ${currentVersion} of ${name}.`));
            return false;
        }
        // Check if requested version is already installed.
        if (targetVersion && currentVersion === targetVersion) {
            this.logger.warn(yellow(`You're already using version ${currentVersion} of ${name}.`));
            return false;
        }
        return true;
    }
    async listVersions(name, currentVersion) {
        const { versions } = await this.getVersions(name);
        this.printVersions(versions, currentVersion);
    }
    printVersions(versions, currentVersion, { maxCols = this.maxCols, indent = 0 } = {}) {
        versions = versions.slice();
        if (versions?.length) {
            versions = versions.map((version) => currentVersion && currentVersion === version
                ? green(`* ${version}`)
                : `  ${version}`);
            if (versions.length > this.maxListSize) {
                const table = new Table().indent(indent);
                const rowSize = Math.ceil(versions.length / maxCols);
                const colSize = Math.min(versions.length, maxCols);
                let versionIndex = 0;
                for (let colIndex = 0; colIndex < colSize; colIndex++) {
                    for (let rowIndex = 0; rowIndex < rowSize; rowIndex++) {
                        if (!table[rowIndex]) {
                            table[rowIndex] = [];
                        }
                        table[rowIndex][colIndex] = versions[versionIndex++];
                    }
                }
                console.log(table.toString());
            }
            else {
                console.log(versions.map((version) => " ".repeat(indent) + version).join("\n"));
            }
        }
    }
    setLogger(logger) {
        this.logger = logger;
    }
    getMain(defaultMain) {
        const main = this.main ?? defaultMain;
        return main ? `/${main}` : "";
    }
}
