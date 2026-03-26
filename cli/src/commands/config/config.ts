import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import {
  formatConfigReference,
  formatConfigReferenceJson,
} from "../init/template.ts";

interface ConfigReferenceOptions {
  json?: boolean;
}

async function referenceAction(opts: ConfigReferenceOptions) {
  if (opts.json) {
    log.info(formatConfigReferenceJson());
  } else {
    log.info(formatConfigReference());
  }
}

const command = new Command()
  .name("config")
  .description("Manage wmill.yaml configuration")
  .command(
    "reference",
    new Command()
      .description("Show all available wmill.yaml configuration options")
      .option("--json", "Output as JSON for programmatic consumption")
      .action(referenceAction as any)
  );

export default command;
