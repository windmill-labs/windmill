import { Command } from "@cliffy/command";
import promptsCommand from "./prompts.ts";
import tsconfigCommand from "./tsconfig.ts";

const command = new Command()
  .description(
    "Refresh wmill-managed project files (AGENTS.wmill.md, skills, tsconfig.wmill.json)"
  )
  .command("prompts", promptsCommand)
  .command("tsconfig", tsconfigCommand);

export default command;
