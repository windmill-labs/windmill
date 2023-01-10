import { PushDiffs, Resource } from "./types.ts";
import { colors, VariableService } from "./deps.ts";
import { model, property } from "./decoverto.ts";

@model()
export class VariableFile implements Resource, PushDiffs {
  @property(() => String)
  value: string;
  @property(() => Boolean)
  is_secret: boolean;
  @property(() => String)
  description: string;
  @property(() => Number)
  account?: number;
  @property(() => Boolean)
  is_oauth?: boolean;

  constructor(value: string, is_secret: boolean, description: string) {
    this.value = value;
    this.is_secret = is_secret;
    this.description = description;
  }
  async push(workspace: string, remotePath: string): Promise<void> {
    if (await VariableService.existsVariable({ workspace, path: remotePath })) {
      const existing = await VariableService.getVariable({
        workspace: workspace,
        path: remotePath,
      });
      if (existing.is_oauth != this.is_oauth) {
        console.log(
          colors.red.underline.bold(
            "Remote variable at " +
              remotePath +
              " exists & has a different oauth state. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource.",
          ),
        );
        return;
      }

      if (existing.account != this.account) {
        console.log(
          colors.red.underline.bold(
            "Remote variable at " +
              remotePath +
              " exists & has a different account state. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource.",
          ),
        );
        return;
      }

      if (existing.is_secret && !this.is_secret) {
        console.log(
          colors.red.underline.bold(
            "Remote variable at " +
              remotePath +
              " exists & is secret. Variables cannot be updated to be no longer secret. If you wish to do this anyways, consider deleting the remote resource.",
          ),
        );
        return;
      }

      const actual_secret = this.is_secret ? true : undefined;

      console.log(colors.yellow("Updating existing variable..."));
      await VariableService.updateVariable({
        workspace,
        path: remotePath,
        requestBody: {
          description: this.description,
          is_secret: actual_secret,
          path: remotePath,
          value: this.value,
        },
      });
    } else {
      console.log(colors.yellow("Creating new variable..."));
      await VariableService.createVariable({
        workspace,
        requestBody: {
          path: remotePath,
          description: this.description,
          is_secret: this.is_secret,
          value: this.value,
          account: this.account,
          is_oauth: this.is_oauth,
        },
      });
    }
  }
}
