import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import {
  formatConfigReference,
  formatConfigReferenceJson,
} from "../init/template.ts";

interface ConfigOptions {
  json?: boolean;
}

async function configAction(opts: ConfigOptions) {
  if (opts.json) {
    console.log(formatConfigReferenceJson());
  } else {
    log.info(formatConfigReference());
  }
}

const command = new Command()
  .name("config")
  .description("Show all available wmill.yaml configuration options")
  .option("--json", "Output as JSON for programmatic consumption")
  .action(configAction as any);

export default command;
