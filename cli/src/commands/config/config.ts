import { getRootStore } from "../../core/store.ts";
import { Command } from "../../../deps.ts";
import { allWorkspaces } from "../workspace/workspace.ts";

const command = new Command()
    .description("config related actions")
    .option("-p, --path", "show config path")
    .action(async (opts) => {
        if (opts.path) {
            const configDir = await getRootStore();
            console.log(configDir);
        } else {
            const all = await allWorkspaces();
            console.log(JSON.stringify(all, null, 2));
        }
    });

export default command;