import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { mergeConfigWithConfigFile } from "../../core/conf.ts";
import * as wmill from "../../../gen/services.gen.ts";

function formatTimestamp(ts: string): string {
  const date = new Date(ts);
  return date.toISOString().replace("T", " ").substring(0, 19);
}

async function list(
  opts: GlobalOptions & {
    json?: boolean;
    username?: string;
    operation?: string;
    actionKind?: string;
    before?: string;
    after?: string;
    limit?: number;
    resource?: string;
  }
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const logs = await wmill.listAuditLogs({
    workspace: workspace.workspaceId,
    username: opts.username,
    operation: opts.operation,
    actionKind: opts.actionKind as any,
    before: opts.before,
    after: opts.after,
    perPage: opts.limit ?? 30,
  });

  if (opts.json) {
    console.log(JSON.stringify(logs));
  } else {
    if (logs.length === 0) {
      log.info("No audit logs found.");
      return;
    }
    new Table()
      .header(["ID", "Timestamp", "Username", "Operation", "Action", "Resource"])
      .padding(2)
      .border(true)
      .body(
        logs.map((l) => [
          String(l.id),
          formatTimestamp(l.timestamp),
          l.username,
          l.operation,
          l.action_kind,
          l.resource ?? "-",
        ])
      )
      .render();
  }
}

async function get(
  opts: GlobalOptions & { json?: boolean },
  id: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const auditLog = await wmill.getAuditLog({
    workspace: workspace.workspaceId,
    id: parseInt(id, 10),
  });

  if (opts.json) {
    console.log(JSON.stringify(auditLog));
  } else {
    console.log(colors.bold("ID:") + " " + auditLog.id);
    console.log(colors.bold("Timestamp:") + " " + formatTimestamp(auditLog.timestamp));
    console.log(colors.bold("Username:") + " " + auditLog.username);
    console.log(colors.bold("Operation:") + " " + auditLog.operation);
    console.log(colors.bold("Action Kind:") + " " + auditLog.action_kind);
    console.log(colors.bold("Resource:") + " " + (auditLog.resource ?? "-"));
    if (auditLog.parameters && Object.keys(auditLog.parameters).length > 0) {
      console.log(colors.bold("Parameters:"));
      console.log(JSON.stringify(auditLog.parameters, null, 2));
    }
  }
}

const command = new Command()
  .description("View audit logs (requires admin)")
  .option("--json", "Output as JSON (for piping to jq)")
  .option("--username <username:string>", "Filter by username")
  .option("--operation <operation:string>", "Filter by operation (exact or prefix)")
  .option("--action-kind <actionKind:string>", "Filter by action kind (Create, Update, Delete, Execute)")
  .option("--before <before:string>", "Filter events before this timestamp")
  .option("--after <after:string>", "Filter events after this timestamp")
  .option("--limit <limit:number>", "Number of entries to return (default 30, max 100)")
  .action(list as any)
  .command("list", "List audit log entries")
  .option("--json", "Output as JSON (for piping to jq)")
  .option("--username <username:string>", "Filter by username")
  .option("--operation <operation:string>", "Filter by operation (exact or prefix)")
  .option("--action-kind <actionKind:string>", "Filter by action kind (Create, Update, Delete, Execute)")
  .option("--before <before:string>", "Filter events before this timestamp")
  .option("--after <after:string>", "Filter events after this timestamp")
  .option("--limit <limit:number>", "Number of entries to return (default 30, max 100)")
  .action(list as any)
  .command("get", "Get a specific audit log entry")
  .arguments("<id:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any);

export default command;
