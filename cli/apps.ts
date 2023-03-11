import { Any, model, property } from "./decoverto.ts";
import {
  AppService,
  colors,
  microdiff,
  Policy,
} from "./deps.ts";
import { Difference, PushDiffs, Resource, setValueByPath } from "./types.ts";

@model()
export class AppFile implements Resource, PushDiffs {
  @property(Any)
  value: any;
  @property(() => String)
  summary: string;
  @property(Any)
  policy: Policy;


  constructor(value: string, summary: string, policy: Policy) {
    this.value = value;
    this.summary = summary;
    this.policy = policy;
  }
  async pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[],
  ): Promise<void> {
    if (await AppService.existsApp({ workspace, path: remotePath })) {
      console.log(
        colors.bold.yellow(
          `Applying ${diffs.length} diffs to existing app... ${remotePath}`,
        ),
      );
      const changeset: {
        path?: string | undefined;
        summary?: string | undefined;
        value?: any;
        policy?: Policy | undefined;
      } = {};
      for (const diff of diffs) {
        if (
          diff.type !== "REMOVE" &&
          (
            diff.path[0] !== "value" && diff.path[0] !== "policy" && (
              diff.path.length !== 1 ||
              !["path", "summary"].includes(
                diff.path[0] as string,
              )
            )
          )
        ) {
          throw new Error("Invalid app diff with path " + diff.path);
        }
        if (diff.type === "CREATE" || diff.type === "CHANGE") {
          setValueByPath(changeset, diff.path, diff.value);
        } else if (diff.type === "REMOVE") {
          setValueByPath(changeset, diff.path, null);
        }
      }

      const hasChanges = Object.values(changeset).some((v) =>
        v !== null && typeof v !== "undefined"
      );
      if (!hasChanges) {
        return;
      }

      await AppService.updateApp({
        workspace,
        path: remotePath,
        requestBody: changeset,
      });
    } else {
      console.log(colors.yellow.bold("Creating new app..."));
      await AppService.createApp({
        workspace,
        requestBody: {
          path: remotePath,
          policy: this.policy,
          summary: this.summary,
          value: this.value,
        },
      });
    }
  }
  async push(workspace: string, remotePath: string): Promise<void> {
    await this.pushDiffs(
      workspace,
      remotePath,
      microdiff({}, this, { cyclesFix: false }),
    );
  }
}
