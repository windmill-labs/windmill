import { getRootStore } from "../../core/store.ts";
import { Command } from "../../../deps.ts";
import { allWorkspaces, Workspace } from "../workspace/workspace.ts";

type WorkspaceWithOptionalToken = Omit<Workspace, 'token'> & { token?: string };

const command = new Command()
    .description("Shows current workspace config. Use -t to show tokens. Use -p to only show the config folder path.")
    .option("-p, --path", "show config path")
    .option("-t, --token", "show tokens")
    .action(async (opts) => {
        if (opts.path) {
            const configDir = await getRootStore();
            console.log(configDir);
        } else {
            const all = await allWorkspaces();
            const sanitized = all.map((workspace) => {
                const res: WorkspaceWithOptionalToken = { ...workspace };
                if (!opts.token) {
                    delete res.token;
                }
                return res; 
            });
            console.log(JSON.stringify(sanitized, null, 2));
        }
    });

export default command;