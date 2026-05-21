import { Command } from "@cliffy/command";
import promptsCommand from "./prompts.ts";

const command = new Command()
  .description("Refresh wmill-managed project files (AGENTS.cli.md and skills)")
  .command("prompts", promptsCommand);

export default command;
