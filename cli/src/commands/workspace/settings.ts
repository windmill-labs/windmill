import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import { stringify as yamlStringify } from "yaml";
import { GlobalOptions } from "../../types.ts";
import * as log from "../../core/log.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";

export async function getSettings(
  opts: GlobalOptions & { encrypted?: boolean; json?: boolean },
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);
  const document = await wmill.getWorkspaceSettingsDocument({
    workspace: workspace.workspaceId,
    secrets: opts.encrypted ? "encrypted" : "redacted",
  });
  console.log(
    opts.json ? JSON.stringify(document, null, 2) : yamlStringify(document),
  );
}

export async function applySettings(
  opts: GlobalOptions & { dryRun?: boolean; replace?: boolean; yes?: boolean },
  file: string,
) {
  await requireLogin(opts);
  const workspace = await resolveWorkspace(opts);
  const document = await yamlParseFile(file);
  if (!document || typeof document !== "object" || Array.isArray(document)) {
    throw new Error(`${file} must hold a settings document (a mapping of setting to value)`);
  }
  const apply = (dry_run: boolean) =>
    wmill.applyWorkspaceSettingsDocument({
      workspace: workspace.workspaceId,
      requestBody: { document, replace: opts.replace ?? false, dry_run },
    });

  const plan = await apply(true);
  for (const warning of plan.warnings ?? []) {
    log.warn(warning);
  }
  if (plan.changes.length === 0) {
    log.info(colors.green("Workspace settings are already up to date"));
    return;
  }
  for (const change of plan.changes) {
    log.info(
      `${colors.bold(change.key)}: ${JSON.stringify(change.before)} -> ${JSON.stringify(change.after)}`,
    );
  }
  if (opts.dryRun) {
    return;
  }
  if (
    !opts.yes &&
    !(await Confirm.prompt({
      message: `Apply ${plan.changes.length} change(s) to workspace ${workspace.workspaceId}?`,
      default: true,
    }))
  ) {
    return;
  }
  const result = await apply(false);
  for (const warning of result.warnings ?? []) {
    log.warn(warning);
  }
  log.info(
    colors.bold.underline.green(
      `Applied ${result.changes.length} change(s) to workspace ${workspace.workspaceId}`,
    ),
  );
}
