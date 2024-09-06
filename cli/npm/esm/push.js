// deno-lint-ignore-file no-explicit-any
import { colors, Command, log } from "./deps.js";
function stub(_opts, _dir) {
    log.info(colors.red.underline('Push is deprecated. Use "sync push --raw" instead. See <TODO_LINK_HERE> for more information.'));
}
const command = new Command()
    .description("Push all files from a folder")
    .arguments("[dir:string]")
    .action(stub);
export default command;
