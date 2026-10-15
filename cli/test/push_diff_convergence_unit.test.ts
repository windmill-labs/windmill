import { afterAll, beforeAll, expect, test } from "bun:test";
import JSZip from "jszip";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, sep } from "node:path";
import {
  checkoutInlineNames,
  compareDynFSElement,
  ZipFSElement,
} from "../src/commands/sync/sync.ts";
import { changedCustomPath } from "../src/commands/app/app.ts";

// The differ also reads the working tree (shared lockfiles, dependency files);
// an empty one keeps that out of the picture.
const originalCwd = process.cwd();
beforeAll(() => {
  process.chdir(mkdtempSync(join(tmpdir(), "wmill-push-diff-")));
});
afterAll(() => {
  process.chdir(originalCwd);
});

// A push is only useful when a second run of it finds nothing left to do.
// These pin the two shapes that used to be listed on every run of a push into
// a fork while the push itself either applied nothing or aborted.

type Mock = {
  isDirectory: boolean;
  path: string;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<Mock>;
};

// Both sides of the differ use the OS separator; fixtures are written with
// "/" and rows are read back the same way.
const osPath = (p: string) => p.split("/").join(sep);
const slashPath = (p: string) => p.split(sep).join("/");

function local(files: Record<string, string>): Mock {
  return {
    isDirectory: true,
    path: "",
    async getContentText() {
      return "";
    },
    async *getChildren() {
      for (const [path, content] of Object.entries(files)) {
        yield {
          isDirectory: false,
          path: osPath(path),
          async getContentText() {
            return content;
          },
          async *getChildren() {},
        };
      }
    },
  };
}

const noIgnore = () => false;

async function diff(
  localEl: Mock,
  remoteEl: Mock,
  skips: Record<string, unknown>,
  parentOwnsScheduleEnabled?: (scheduleFilePath: string) => boolean,
  isEls1Remote = false,
) {
  const { changes } = await compareDynFSElement(
    localEl as any,
    remoteEl as any,
    noIgnore,
    false,
    skips as any,
    true,
    [],
    false,
    undefined,
    undefined,
    isEls1Remote,
    false,
    parentOwnsScheduleEnabled,
  );
  return changes.map((c) => `${c.name} ${slashPath(c.path)}`);
}

const SCHEDULE = (enabled: string) =>
  `summary: nightly\nargs: {}\nenabled: ${enabled}\nis_flow: true\nschedule: 0 0 0 * * *\nscript_path: f/mail/flow\ntimezone: UTC\n`;

test("push into a fork: a schedule the parent also has compares without `enabled`", async () => {
  const remote = local({ "f/mail/nightly.schedule.yaml": SCHEDULE("true") });
  const skips = { includeSchedules: true };
  // The parent has `f/mail/nightly`; any other schedule is the fork's own.
  const parentHas = (filePath: string) =>
    slashPath(filePath) === "f/mail/nightly.schedule.yaml";

  // Not a fork: `enabled` is compared like any other field.
  expect(
    await diff(
      local({ "f/mail/nightly.schedule.yaml": SCHEDULE("false") }),
      remote,
      skips,
    ),
  ).toEqual(["edited f/mail/nightly.schedule.yaml"]);
  expect(
    await diff(
      local({ "f/mail/nightly.schedule.yaml": SCHEDULE("false") }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual([]);
  // The key being absent is the same case as it differing.
  expect(
    await diff(
      local({
        "f/mail/nightly.schedule.yaml": SCHEDULE("false").replace(
          "enabled: false\n",
          "",
        ),
      }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual([]);
  // Only `enabled` is set aside.
  expect(
    await diff(
      local({
        "f/mail/nightly.schedule.yaml": SCHEDULE("false").replace(
          "0 0 0 * * *",
          "0 0 1 * * *",
        ),
      }),
      remote,
      skips,
      parentHas,
    ),
  ).toEqual(["edited f/mail/nightly.schedule.yaml"]);
  // A schedule only the fork has keeps toggling from the file.
  expect(
    await diff(
      local({ "f/mail/fork_only.schedule.yaml": SCHEDULE("true") }),
      local({ "f/mail/fork_only.schedule.yaml": SCHEDULE("false") }),
      skips,
      parentHas,
    ),
  ).toEqual(["edited f/mail/fork_only.schedule.yaml"]);
});

const SUMMARY = "process one mail end-to-end (spam check, classify)";

function remoteFlow(content: string) {
  const zip = new JSZip();
  zip.file(
    "f/mail/flow_v2.flow.json",
    JSON.stringify({
      summary: "Flow V2",
      description: "",
      value: {
        modules: [
          {
            id: "a",
            summary: SUMMARY,
            value: {
              type: "rawscript",
              content,
              input_transforms: {},
              language: "python3",
            },
          },
        ],
      },
      schema: { type: "object", properties: {} },
    }),
    // The backend's archive carries no directory entries.
    { createFolders: false },
  );
  return zip;
}

function localFlow(content: string) {
  return local({
    "f/mail/flow_v2.flow/flow.yaml": `summary: Flow V2\ndescription: ''\nvalue:\n  modules:\n    - id: a\n      summary: ${SUMMARY}\n      value:\n        type: rawscript\n        content: '!inline process_mail.inline_script.py'\n        input_transforms: {}\n        language: python3\nschema:\n  type: object\n  properties: {}\n`,
    "f/mail/flow_v2.flow/process_mail.inline_script.py": content,
  });
}

// The checkout's `!inline` references, as `push` reads them from its flow.yaml.
const checkoutNames = async (flowDir: string) =>
  slashPath(flowDir) === "f/mail/flow_v2.flow"
    ? { a: "process_mail.inline_script.py" }
    : {};

test("push: an inline script the checkout names differently from the step summary is not a rename", async () => {
  const skips = { includeSchedules: false };
  const render = (content: string, withCheckout: boolean) =>
    ZipFSElement(
      remoteFlow(content),
      true,
      "bun",
      {},
      {},
      false,
      true,
      withCheckout ? checkoutNames : undefined,
    ) as any;

  // Same content, file named by hand: three rows before, none after.
  expect(
    await diff(
      localFlow("def main():\n    return 1\n"),
      render("def main():\n    return 1\n", false),
      skips,
    ),
  ).toEqual([
    "deleted f/mail/flow_v2.flow/process_one_mail_end-to-end_(spam_check,_classify).inline_script.py",
    "edited f/mail/flow_v2.flow/flow.yaml",
    "added f/mail/flow_v2.flow/process_mail.inline_script.py",
  ]);
  expect(
    await diff(
      localFlow("def main():\n    return 1\n"),
      render("def main():\n    return 1\n", true),
      skips,
    ),
  ).toEqual([]);

  // A real edit is still one.
  expect(
    await diff(
      localFlow("def main():\n    return 2\n"),
      render("def main():\n    return 1\n", true),
      skips,
    ),
  ).toEqual(["edited f/mail/flow_v2.flow/process_mail.inline_script.py"]);
});

test("push: a checkout name that collides with another step's summary-derived name keeps two files", async () => {
  const zip = new JSZip();
  zip.file(
    "f/mail/flow_v2.flow.json",
    JSON.stringify({
      summary: "Flow V2",
      description: "",
      value: {
        modules: [
          {
            id: "a",
            summary: SUMMARY,
            value: {
              type: "rawscript",
              content: "a",
              input_transforms: {},
              language: "python3",
            },
          },
          {
            id: "b",
            summary: "process_mail",
            value: {
              type: "rawscript",
              content: "b",
              input_transforms: {},
              language: "python3",
            },
          },
        ],
      },
      schema: { type: "object", properties: {} },
    }),
    { createFolders: false },
  );
  // Nothing local: every rendered file is a "deleted" row, one per path.
  const rows = await diff(
    local({}),
    ZipFSElement(zip, true, "bun", {}, {}, false, true, checkoutNames) as any,
    { includeSchedules: false },
  );
  expect(rows.filter((r) => r.endsWith(".py")).sort()).toEqual([
    "deleted f/mail/flow_v2.flow/process_mail.inline_script.py",
    "deleted f/mail/flow_v2.flow/process_one_mail_end-to-end_(spam_check,_classify).inline_script.py",
  ]);
});

test("push: checkout inline names stay inside the flow folder", async () => {
  const flowYaml = join(process.cwd(), "flow.yaml");
  writeFileSync(
    flowYaml,
    `summary: x\nvalue:\n  modules:\n    - id: a\n      value:\n        type: rawscript\n        content: '!inline a.inline_script.py'\n        language: python3\n    - id: b\n      value:\n        type: rawscript\n        content: '!inline ../shared/b.py'\n        language: python3\n    - id: c\n      value:\n        type: rawscript\n        content: '!inline /tmp/c.py'\n        language: python3\n`,
  );
  expect(await checkoutInlineNames(flowYaml)).toEqual({
    a: "a.inline_script.py",
  });
  expect(
    await checkoutInlineNames(join(process.cwd(), "missing.yaml")),
  ).toEqual({});
});

// A push never applies the workspace's display name and applies its color only
// when the local file carries one (see pushWorkspaceSettings), so a file that
// differs only in what would not be applied is not a push change; a pull still
// rewrites the file.
test("push: settings.yaml differing only by name or an unset color is not a change", async () => {
  const remote = local({
    "settings.yaml": "name: prod\ncolor: '#ff0000'\nerror_handler: null\n",
  });
  const unsetColor = local({
    "settings.yaml": "name: staging\nerror_handler: null\n",
  });
  const skips = { includeSettings: true };
  expect(await diff(unsetColor, remote, skips)).toEqual([]);
  expect(await diff(remote, unsetColor, skips, undefined, true)).toEqual([
    "edited settings.yaml",
  ]);

  const otherColor = local({
    "settings.yaml": "name: staging\ncolor: '#00ff00'\nerror_handler: null\n",
  });
  expect(await diff(otherColor, remote, skips)).toEqual(["edited settings.yaml"]);
});

// A push applies auto_invite.instance_groups only when the local file declares
// them (see pushWorkspaceSettings).
test("push: settings.yaml without instance_groups is not a change", async () => {
  const remote = local({
    "settings.yaml":
      "name: prod\nauto_invite:\n  enabled: false\n  instance_groups:\n    - eng\n  instance_groups_roles:\n    eng: developer\n",
  });
  const undeclared = local({
    "settings.yaml": "name: prod\nauto_invite:\n  enabled: false\n",
  });
  const skips = { includeSettings: true };
  expect(await diff(undeclared, remote, skips)).toEqual([]);

  const groupsOnlyRemote = local({
    "settings.yaml": "name: prod\nauto_invite:\n  instance_groups:\n    - eng\n",
  });
  const noAutoInvite = local({ "settings.yaml": "name: prod\n" });
  expect(await diff(noAutoInvite, groupsOnlyRemote, skips)).toEqual([]);

  const otherGroups = local({
    "settings.yaml":
      "name: prod\nauto_invite:\n  enabled: false\n  instance_groups: []\n",
  });
  expect(await diff(otherGroups, remote, skips)).toEqual(["edited settings.yaml"]);
});

function remoteRawApp(customPath?: string) {
  const zip = new JSZip();
  zip.file(
    "f/test/myapp.raw_app.json",
    JSON.stringify({
      summary: "My app",
      policy: { execution_mode: "publisher" },
      ...(customPath ? { custom_path: customPath } : {}),
      value: { files: {}, runnables: {} },
    }),
    { createFolders: false },
  );
  return ZipFSElement(zip, true, "bun", {}, {}, false, true) as any;
}

async function rendered(el: Mock): Promise<Record<string, string>> {
  const files: Record<string, string> = {};
  const walk = async (el: Mock) => {
    for await (const child of el.getChildren()) {
      if (child.isDirectory) await walk(child);
      else files[slashPath(child.path)] = await child.getContentText();
    }
  };
  await walk(el);
  return files;
}

// Workspace-named as push and pull name it, without any specificItems.
async function diffIn(ws: string, localEl: Mock, remoteEl: Mock, pull: boolean) {
  const { changes } = await compareDynFSElement(
    (pull ? remoteEl : localEl) as any,
    (pull ? localEl : remoteEl) as any,
    noIgnore,
    false,
    {} as any,
    !pull,
    [],
    pull,
    undefined,
    ws,
    pull,
    false,
  );
  return changes.map((c) => `${c.name} ${slashPath(c.path)}`);
}

test("a custom path is its own workspace-specific file", async () => {
  const files = await rendered(remoteRawApp("my-dashboard"));
  expect(files["f/test/myapp.custom_path.yaml"]).toBe(
    "custom_path: my-dashboard\n",
  );
  const rawAppYaml = files["f/test/myapp.raw_app/raw_app.yaml"];
  expect(rawAppYaml).toBeDefined();
  expect(rawAppYaml).not.toContain("custom_path");

  const { "f/test/myapp.custom_path.yaml": _, ...appFiles } = files;
  const checkout = (extra: Record<string, string>) =>
    local({ ...appFiles, ...extra });

  // The workspace's own file is the one compared; another's is ignored.
  const own = checkout({
    "f/test/myapp.prod.custom_path.yaml": "custom_path: my-dashboard\n",
    "f/test/myapp.staging.custom_path.yaml": "custom_path: staging-dash\n",
  });
  for (const pull of [true, false]) {
    expect(await diffIn("prod", own, remoteRawApp("my-dashboard"), pull)).toEqual([]);
  }
  expect(
    await diffIn("staging", own, remoteRawApp("my-dashboard"), false),
  ).toEqual(["edited f/test/myapp.custom_path.yaml"]);

  // Promotion: a checkout without the target's file leaves its path alone on
  // push, and a pull brings it in as the target's file.
  const promoted = checkout({
    "f/test/myapp.staging.custom_path.yaml": "custom_path: staging-dash\n",
  });
  expect(await diffIn("prod", promoted, remoteRawApp("my-dashboard"), false)).toEqual([]);
  expect(await diffIn("prod", promoted, remoteRawApp("my-dashboard"), true)).toEqual([
    "added f/test/myapp.custom_path.yaml",
  ]);
  // A pull mirrors a cleared custom path by deleting the workspace's file.
  expect(await diffIn("prod", own, remoteRawApp(), true)).toEqual([
    "deleted f/test/myapp.custom_path.yaml",
  ]);
  // `custom_path: ''` clears it on push, and is in sync once it is cleared.
  const cleared = checkout({
    "f/test/myapp.prod.custom_path.yaml": "custom_path: ''\n",
  });
  expect(await diffIn("prod", cleared, remoteRawApp("my-dashboard"), false)).toEqual([
    "edited f/test/myapp.custom_path.yaml",
  ]);
  expect(await diffIn("prod", cleared, remoteRawApp(), false)).toEqual([]);
});

// A checkout from before custom paths had their own file keeps the value in
// raw_app.yaml: a push must converge on it, and a pull moves it out.
test("an in-file custom path converges on push", async () => {
  const files = await rendered(remoteRawApp("my-dashboard"));
  const { "f/test/myapp.custom_path.yaml": _, ...appFiles } = files;
  const inFile = (value: string) =>
    local({
      ...appFiles,
      "f/test/myapp.raw_app/raw_app.yaml":
        files["f/test/myapp.raw_app/raw_app.yaml"] + `custom_path: ${value}\n`,
    });

  expect(await diffIn("prod", inFile("my-dashboard"), remoteRawApp("my-dashboard"), false)).toEqual([]);
  expect(await diffIn("prod", inFile("other"), remoteRawApp("my-dashboard"), false)).toEqual([
    "edited f/test/myapp.custom_path.yaml",
  ]);
  expect(await diffIn("prod", inFile("my-dashboard"), remoteRawApp("my-dashboard"), true)).toEqual([
    "edited f/test/myapp.raw_app/raw_app.yaml",
    "added f/test/myapp.custom_path.yaml",
  ]);
});

test("push: an unchanged custom_path is not sent", () => {
  expect(
    changedCustomPath("my-dashboard", { custom_path: "my-dashboard" }),
  ).toBeUndefined();
  expect(changedCustomPath(undefined, { custom_path: "my-dashboard" })).toBeUndefined();
  expect(changedCustomPath("", undefined)).toBeUndefined();
  expect(changedCustomPath("new", { custom_path: "my-dashboard" })).toBe("new");
  expect(changedCustomPath("", { custom_path: "my-dashboard" })).toBe("");
  expect(changedCustomPath("new", undefined)).toBe("new");
});
